use super::{get_2d_painter, get_mut_2d_painter, Pixel, Triangle};
use crate::m2d::images_2d::Image;
use crate::m2d::painter_2d::DrawPipeline;
use crate::sprite::Sprite;
use arrayvec::ArrayVec;
use core::app::window_size;
use core::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use core::gfx::{
    self, AsRenderer, BindGroup, Buffer, Color, RenderPipeline, RenderTexture, Renderer, Texture,
};
use core::math::{vec3, Mat3, Mat4, Vec2};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

// TODO Cached elements is a must
// TODO Camera2D

// This is used to avoid heap allocations when doing small number of drawcalls
const STACK_ALLOCATED_QUADS: usize = 200;

#[derive(Clone)]
pub struct PipelineContext {
    pub pipeline: RenderPipeline,
    pub groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub vertex_offset: usize,
    pub x_pos: usize,
    pub y_pos: usize,
    pub alpha_pos: Option<usize>,
}

struct BatchInfo {
    start_idx: usize,
    end_idx: usize,
    pipeline: RenderPipeline,
    bind_groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    sprite: Option<Sprite>,
}

impl Clone for BatchInfo {
    fn clone(&self) -> Self {
        Self {
            start_idx: self.start_idx,
            end_idx: self.end_idx,
            pipeline: self.pipeline.clone(),
            bind_groups: self.bind_groups.clone(),
            sprite: None,
        }
    }
}

impl BatchInfo {
    fn is_compatible(&self, other: &Self) -> bool {
        if self.pipeline != other.pipeline {
            return false;
        }

        // FIXME check bind_groups instead of sprite?
        if self.sprite != other.sprite {
            return false;
        }

        true
    }
}

pub struct Drawing<'a, T>
where
    T: Element2D,
{
    inner: Option<T>,
    draw: &'a mut Draw2D,
}

impl<'a, T> Drawing<'a, T>
where
    T: Element2D,
{
    pub fn new(draw: &'a mut Draw2D, inner: T) -> Self {
        Self {
            inner: Some(inner),
            draw,
        }
    }

    pub fn into_inner(mut self) -> T {
        self.inner.take().unwrap()
    }
}

impl<T> Drop for Drawing<'_, T>
where
    T: Element2D,
{
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            self.draw.add_element(&inner);
        }
    }
}

impl<T> Deref for Drawing<'_, T>
where
    T: Element2D,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T> DerefMut for Drawing<'_, T>
where
    T: Element2D,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

#[derive(Default, Clone)]
pub struct Draw2D {
    projection: Mat4,
    clear_color: Option<Color>,
    alpha: f32,

    indices_offset: usize,
    batches: SmallVec<BatchInfo, STACK_ALLOCATED_QUADS>,
    vertices: SmallVec<f32, { STACK_ALLOCATED_QUADS * 12 }>,
    indices: SmallVec<u32, { STACK_ALLOCATED_QUADS * 6 }>,
}

impl Draw2D {
    pub fn new() -> Self {
        let size = window_size();
        let projection = Mat4::orthographic_rh(0.0, size.x, size.y, 0.0, 0.0, 1.0);
        Self {
            projection,
            alpha: 1.0,
            ..Default::default()
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.clear_color = Some(color);
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    pub fn add_element<T>(&mut self, element: &T)
    where
        T: Element2D,
    {
        element.process(self);
    }

    pub fn add_to_batch<'a>(&'a mut self, info: DrawingInfo<'a>) {
        let start_idx = self.indices.len();
        let end_idx = start_idx + info.indices.len();
        let mut painter = get_mut_2d_painter();
        let PipelineContext {
            pipeline,
            mut groups,
            vertex_offset,
            x_pos,
            y_pos,
            alpha_pos,
        } = painter
            .pipelines
            .get(&info.pipeline.id_intern())
            .ok_or_else(|| "Missing Pipeline")
            .unwrap()
            .clone();

        if let Some(sp) = info.sprite {
            let bind_group = painter.cached_bind_group_for(&pipeline, sp);
            // FIXME this is wrong, it should be bind_groups[1] = bind_group
            groups.push(bind_group);
        }

        let batch = BatchInfo {
            start_idx,
            end_idx,
            pipeline,
            bind_groups: groups,
            sprite: info.sprite.cloned(),
        };

        let new_batch = match self.batches.last() {
            None => true,
            Some(last) => !last.is_compatible(&batch),
        };

        if new_batch {
            println!("#----> new batch ");
            self.batches.push(batch);
        }

        let current = self.batches.last_mut().unwrap();
        current.end_idx = end_idx;

        // the indices must use an offset
        self.indices.extend(
            info.indices
                .iter()
                .map(|idx| idx + self.indices_offset as u32),
        );
        self.indices_offset += info.vertices.len() / vertex_offset;

        let matrix = self.matrix() * info.transform;
        info.vertices
            .chunks_exact_mut(vertex_offset)
            .for_each(|chunk| {
                debug_assert!(chunk.len() >= 2);

                let x = chunk[x_pos];
                let y = chunk[y_pos];

                let xyz = matrix * vec3(x, y, 1.0);
                chunk[x_pos] = xyz.x;
                chunk[y_pos] = xyz.y;

                if let Some(a_pos) = alpha_pos {
                    let alpha = chunk[a_pos] * self.alpha;
                    chunk[a_pos] = alpha;
                }
            });
        self.vertices.extend_from_slice(info.vertices);
    }

    // - Transform TODO
    pub fn projection(&self) -> Mat4 {
        Mat4::IDENTITY
    }

    pub fn push_transform(&mut self) {}

    pub fn set_matrix(&mut self) {}

    pub fn matrix(&self) -> Mat3 {
        Mat3::IDENTITY
    }

    pub fn pop_transform(&mut self) {}

    // - included methods
    pub fn pixel(&mut self, pos: Vec2) -> Drawing<'_, Pixel> {
        Drawing::new(self, Pixel::new(pos))
    }

    pub fn triangle(&mut self, p1: Vec2, p2: Vec2, p3: Vec2) -> Drawing<'_, Triangle> {
        Drawing::new(self, Triangle::new(p1, p2, p3))
    }

    pub fn image(&mut self, sprite: &Sprite) -> Drawing<'_, Image> {
        Drawing::new(self, Image::new(sprite))
    }
}

pub struct DrawingInfo<'a> {
    pub pipeline: DrawPipeline,
    pub vertices: &'a mut [f32],
    pub indices: &'a [u32],
    pub transform: Mat3,
    pub sprite: Option<&'a Sprite>,
}

pub trait Element2D {
    fn process(&self, draw: &mut Draw2D);
}

impl AsRenderer for Draw2D {
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let mut painter = get_mut_2d_painter();

        let ubo_transform = &painter.ubo_transform;
        let vbo = &painter.vbo;
        let ebo = &painter.ebo;

        // TODO check dirty transform flag to avoid update all the time this
        gfx::write_buffer(ubo_transform)
            .with_data(self.projection.as_ref())
            .build()
            .unwrap();

        gfx::write_buffer(vbo)
            .with_data(&self.vertices)
            .build()
            .unwrap();

        gfx::write_buffer(ebo)
            .with_data(&self.indices)
            .build()
            .unwrap();

        let mut cleared = false;
        let mut renderer = Renderer::new();
        self.batches.iter().for_each(|b| {
            let mut pass = renderer.begin_pass();

            // clear only once
            if !cleared {
                if let Some(color) = self.clear_color {
                    pass.clear_color(color);
                    cleared = true;
                }
            }

            let binds: ArrayVec<&BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> =
                b.bind_groups.iter().collect();

            pass.pipeline(&b.pipeline)
                .buffers(&[vbo, ebo])
                .bindings(&binds);

            let start = b.start_idx as u32;
            let end = b.end_idx as u32;
            pass.draw(start..end);
        });

        self.flush(&renderer, target)
    }
}
