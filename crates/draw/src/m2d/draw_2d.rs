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
use internment::Intern;
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

// TODO Cached elements is a must
// TODO Camera2D

// Quads(1000) * Vertices(6)(2f32 per vert) * f32(4bytes) = 48kbs
// Quads(1000) * Indices(6) * u32(4bytes) = 24kbs
// I think that windows, and web have 1MB of default stack size, so this should be fine for now
const STACK_ALLOCATED_QUADS: usize = 1000;

#[derive(Clone)]
pub struct PipelineContext {
    pub pipeline: RenderPipeline,
    pub groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
}

struct BatchInfo {
    start_idx: usize,
    end_idx: usize,
    pipeline: DrawPipeline,
    sprite: Option<Sprite>,
}

impl Clone for BatchInfo {
    fn clone(&self) -> Self {
        Self {
            start_idx: self.start_idx,
            end_idx: self.end_idx,
            pipeline: self.pipeline.clone(),
            sprite: None,
        }
    }
}

impl BatchInfo {
    fn is_compatible(&self, other: &Self) -> bool {
        if self.pipeline != other.pipeline {
            return false;
        }

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
        let pipeline = info.pipeline.clone();
        let batch = BatchInfo {
            start_idx,
            end_idx,
            pipeline,
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
        self.indices_offset += info.vertices.len() / info.offset;

        let matrix = self.matrix() * info.transform;
        // NOTE: we make the assumption that the order is always [x, y, r, g, b, a,... (anything else until offset), then repeat]
        // then we multiply the position by the matrix and the alpha by the global alpha
        self.vertices
            .extend(info.vertices.chunks_exact(info.offset).flat_map(|v| {
                debug_assert!(v.len() >= 6);
                let &[x, y, r, g, b, a, ..] = v else {
                    unreachable!("Vertices are always 6, this should not happen")
                };
                let xyz = matrix * vec3(x, y, 1.0);
                [xyz.x, xyz.y, r, g, b, a * self.alpha]
                    .into_iter()
                    .chain(v[6..].iter().copied())
            }));
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
    pub vertices: &'a [f32],
    pub indices: &'a [u32],
    pub offset: usize,
    pub transform: Mat3,
    pub sprite: Option<&'a Sprite>,
}

pub trait Element2D {
    fn process(&self, draw: &mut Draw2D);
}

impl AsRenderer for Draw2D {
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let painter = get_2d_painter();
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

        let mut owned_binds: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> = ArrayVec::new();
        let mut binds: ArrayVec<&BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> = ArrayVec::new();

        let mut cleared = false;
        let mut renderer = Renderer::new();
        self.batches
            .iter()
            .for_each(|b| match painter.ctx(&b.pipeline.id_intern()) {
                Some(ctx) => {
                    owned_binds.clear();
                    binds.clear();

                    let mut pass = renderer.begin_pass();

                    // clear only once
                    if !cleared {
                        if let Some(color) = self.clear_color {
                            pass.clear_color(color);
                            cleared = true;
                        }
                    }

                    owned_binds.extend(ctx.groups.iter().cloned());

                    if let Some(sprite) = &b.sprite {
                        let texture_group = get_mut_2d_painter().cached_bind_group_for(sprite);
                        owned_binds.push(texture_group);
                    }

                    binds.extend(owned_binds.iter());

                    pass.pipeline(&ctx.pipeline)
                        .buffers(&[vbo, ebo])
                        .bindings(&binds);

                    let start = b.start_idx as u32;
                    let end = b.end_idx as u32;
                    pass.draw(start..end);
                }
                None => {
                    log::error!(
                        "There is no Pipeline in the Draw2D ctx for '{}'",
                        b.pipeline.id()
                    );
                    if cfg!(debug_assertions) {
                        panic!("Missing 2D Pipeline '{}'", b.pipeline.id());
                    }
                }
            });

        self.flush(&renderer, target)
    }
}
