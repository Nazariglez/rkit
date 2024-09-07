use super::{create_pixel_pipeline, create_shapes_2d_pipeline_ctx, PipelineContext};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use core::gfx::{self, BindGroup, Buffer, RenderPass, SamplerId, TextureId};
use core::math::Mat4;
use internment::Intern;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub(crate) static PAINTER_2D: Lazy<AtomicRefCell<Painter2D>> =
    Lazy::new(|| AtomicRefCell::new(Painter2D::default()));

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithreading
unsafe impl Sync for Painter2D {}
unsafe impl Send for Painter2D {}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DrawPipeline {
    Pixel,
    Shapes,
    Custom(Intern<str>),
}

impl DrawPipeline {
    pub fn id(&self) -> &str {
        match self {
            DrawPipeline::Pixel => "gk_pixel",
            DrawPipeline::Shapes => "gk_shapes",
            DrawPipeline::Custom(inner) => inner,
        }
    }

    pub(crate) fn id_intern(&self) -> Intern<str> {
        match self {
            DrawPipeline::Pixel => "gk_pixel".into(),
            DrawPipeline::Shapes => "gk_shapes".into(),
            DrawPipeline::Custom(inner) => *inner,
        }
    }
}

impl From<&str> for DrawPipeline {
    fn from(value: &str) -> Self {
        Self::Custom(value.into())
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct SpriteId {
    texture: TextureId,
    sampler: SamplerId,
}

pub(crate) struct Painter2D {
    pub pipelines: HashMap<Intern<str>, PipelineContext>,
    pub ubo_transform: Buffer,
    pub vbo: Buffer,
    pub ebo: Buffer,
    pub sprites_cache: HashMap<SpriteId, BindGroup>,
}

impl Default for Painter2D {
    fn default() -> Self {
        let ubo = gfx::create_uniform_buffer(Mat4::IDENTITY.as_ref())
            .with_label("Painter2D UBO Transform")
            .with_write_flag(true)
            .build()
            .unwrap();

        let vbo = gfx::create_vertex_buffer(&[] as &[f32])
            .with_label("Painter2D VBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let ebo = gfx::create_index_buffer(&[] as &[u32])
            .with_label("Painter2D EBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let mut painter = Self {
            pipelines: Default::default(),
            ubo_transform: ubo,
            vbo,
            ebo,
            sprites_cache: Default::default(),
        };

        // painter.add_pipeline(DrawPipeline::Pixel.id(), create_pixel_pipeline_ctx().unwrap());
        painter.add_pipeline(
            DrawPipeline::Shapes.id(),
            create_shapes_2d_pipeline_ctx(&painter.ubo_transform).unwrap(),
        );

        painter
    }
}

impl Painter2D {
    pub fn add_pipeline(&mut self, id: &str, pip: PipelineContext) -> Option<PipelineContext> {
        self.pipelines.insert(id.into(), pip)
    }

    pub fn remove_pipeline(&mut self, id: &str) -> Option<PipelineContext> {
        self.pipelines.remove(&id.into())
    }

    pub fn ctx(&self, id: &Intern<str>) -> Option<&PipelineContext> {
        self.pipelines.get(id)
    }
}

pub(crate) fn get_2d_painter() -> AtomicRef<'static, Painter2D> {
    PAINTER_2D.borrow()
}

pub(crate) fn get_mut_2d_painter() -> AtomicRefMut<'static, Painter2D> {
    PAINTER_2D.borrow_mut()
}
