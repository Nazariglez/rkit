use super::{get_2d_painter, get_mut_2d_painter, Pixel};
use crate::m2d::images::Image2D;
use crate::m2d::mat3_stack::Mat3Stack;
use crate::m2d::painter::DrawPipeline;
use crate::m2d::shapes::{Line2D, Path2D, Rectangle2D, Triangle2D};
use crate::m2d::text::Text2D;
use crate::sprite::Sprite;
use crate::text::get_mut_text_system;
use crate::{Camera2D, Circle2D, Ellipse2D, Polygon2D, Star2D};
use arrayvec::ArrayVec;
use core::app::window_size;
use core::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use core::gfx::{self, AsRenderer, BindGroup, Color, RenderPipeline, RenderTexture, Renderer};
use core::math::{vec3, Mat3, Mat4, Rect, Vec2};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut, Range};

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
    vbo_range: Range<u64>,
    ebo_range: Range<u64>,
    start_idx: usize,
    end_idx: usize,
    pipeline: RenderPipeline,
    bind_groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    sprite: Option<Sprite>,
}

impl Clone for BatchInfo {
    fn clone(&self) -> Self {
        Self {
            vbo_range: self.vbo_range.clone(),
            ebo_range: self.ebo_range.clone(),
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

    fn count(&self) -> usize {
        self.end_idx - self.start_idx
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
    pub camera: Camera2D,
    clear_color: Option<Color>,
    alpha: f32,

    matrix_stack: Mat3Stack,

    indices_offset: usize,
    batches: SmallVec<BatchInfo, STACK_ALLOCATED_QUADS>,
    vertices: SmallVec<f32, { STACK_ALLOCATED_QUADS * 12 }>,
    indices: SmallVec<u32, { STACK_ALLOCATED_QUADS * 6 }>,

    pub(crate) last_text_bounds: Rect,
}

impl Draw2D {
    pub fn new() -> Self {
        let mut camera = Camera2D::new(window_size());
        camera.update();
        Self {
            camera,
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

        if matches!(info.pipeline, DrawPipeline::Text) {
            groups.push(get_mut_text_system().bind_group(&pipeline).clone());
        }

        let vbo_start = (self.vertices.len() as u64 * 4);
        let ebo_start = (self.indices.len() as u64 * 4);

        let batch = BatchInfo {
            vbo_range: vbo_start..vbo_start,
            ebo_range: ebo_start..ebo_start,
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
            self.indices_offset = 0;
            self.batches.push(batch);
        }

        let current = self.batches.last_mut().unwrap();
        current.end_idx = end_idx;

        let vbo_count = (info.vertices.len() as u64 * 4); // f32=4bytes
        let ebo_count = (info.indices.len() as u64 * 4); // u32=4bytes
        current.vbo_range.end = current.vbo_range.end + vbo_count;
        current.ebo_range.end = current.ebo_range.end + ebo_count;

        // the indices must use an offset
        self.indices.extend(
            info.indices
                .iter()
                .map(|idx| idx + self.indices_offset as u32),
        );

        self.indices_offset += info.vertices.len() / vertex_offset;

        self.camera.update();
        let matrix = self.camera.transform() * self.matrix() * info.transform;
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

    pub fn last_text_bounds(&self) -> Rect {
        self.last_text_bounds
    }

    // - Transform TODO
    pub fn projection(&self) -> Mat4 {
        Mat4::IDENTITY
    }

    pub fn push_matrix(&mut self, m: Mat3) {
        self.matrix_stack.push(m);
    }

    pub fn set_matrix(&mut self, m: Mat3) {
        self.matrix_stack.set_matrix(m);
    }

    pub fn matrix(&self) -> Mat3 {
        self.matrix_stack.matrix()
    }

    pub fn pop_matrix(&mut self) {
        self.matrix_stack.pop();
    }

    pub fn clear_matrix_stack(&mut self) {
        self.matrix_stack.clear();
    }

    // // - included methods
    // pub fn pixel(&mut self, pos: Vec2) -> Drawing<'_, Pixel> {
    //     Drawing::new(self, Pixel::new(pos))
    // }

    // - shapes
    pub fn path(&mut self) -> Drawing<'_, Path2D> {
        Drawing::new(self, Path2D::new())
    }

    pub fn line(&mut self, p1: Vec2, p2: Vec2) -> Drawing<'_, Line2D> {
        Drawing::new(self, Line2D::new(p1, p2))
    }

    pub fn triangle(&mut self, p1: Vec2, p2: Vec2, p3: Vec2) -> Drawing<'_, Triangle2D> {
        Drawing::new(self, Triangle2D::new(p1, p2, p3))
    }

    pub fn rect(&mut self, pos: Vec2, size: Vec2) -> Drawing<'_, Rectangle2D> {
        Drawing::new(self, Rectangle2D::new(pos, size))
    }

    pub fn circle(&mut self, radius: f32) -> Drawing<'_, Circle2D> {
        Drawing::new(self, Circle2D::new(radius))
    }

    pub fn ellipse(&mut self, pos: Vec2, size: Vec2) -> Drawing<'_, Ellipse2D> {
        Drawing::new(self, Ellipse2D::new(pos, size))
    }

    pub fn star(
        &mut self,
        spikes: u8,
        outer_radius: f32,
        inner_radius: f32,
    ) -> Drawing<'_, Star2D> {
        Drawing::new(self, Star2D::new(spikes, outer_radius, inner_radius))
    }

    pub fn polygon(&mut self, sides: u8, radius: f32) -> Drawing<'_, Polygon2D> {
        Drawing::new(self, Polygon2D::new(sides, radius))
    }

    // - images
    pub fn image(&mut self, sprite: &Sprite) -> Drawing<'_, Image2D> {
        Drawing::new(self, Image2D::new(sprite))
    }

    // - text
    pub fn text<'a, 'b: 'a>(&'a mut self, text: &'b str) -> Drawing<'a, Text2D> {
        Drawing::new(self, Text2D::new(text))
    }

    // pub fn fps(&mut self) -> Drawing<'_, Text2D> {
    //     Drawing::new(self, Text2D::new(""))
    // }
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
            .with_data(self.camera.projection().as_ref())
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
                .buffers_with_offset(&[(&vbo, b.vbo_range.clone()), (&ebo, b.ebo_range.clone())])
                .bindings(&binds);

            let count = b.count() as u32;
            pass.draw(0..count);
        });

        self.flush(&renderer, target)
    }
}