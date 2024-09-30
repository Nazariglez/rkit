use super::{create_pixel_pipeline, create_shapes_2d_pipeline_ctx, PipelineContext};
use crate::sprite::SpriteId;
use crate::{
    create_images_2d_pipeline_ctx, create_pattern_2d_pipeline_ctx, create_text_2d_pipeline_ctx,
    Sprite,
};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use core::gfx::{self, BindGroup, Buffer, RenderPass, RenderPipeline, SamplerId, TextureId};
use core::math::Mat4;
use internment::Intern;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use utils::drop_signal::DropSignal;

pub(crate) static PAINTER_2D: Lazy<AtomicRefCell<Painter2D>> =
    Lazy::new(|| AtomicRefCell::new(Painter2D::default()));

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithreading
unsafe impl Sync for Painter2D {}
unsafe impl Send for Painter2D {}

pub struct PipelineResources<'a> {
    pub ubo: &'a Buffer,
    pub vbo: &'a Buffer,
    pub ebo: &'a Buffer,
    pub sprite_bind_group: &'a BindGroup,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DrawPipeline {
    Pixel,
    Shapes,
    Images,
    Text,
    Pattern,
    Custom(Intern<String>),
}

impl DrawPipeline {
    pub fn id(&self) -> &str {
        match self {
            DrawPipeline::Pixel => "gk_pixel",
            DrawPipeline::Shapes => "gk_shapes",
            DrawPipeline::Images => "gk_images",
            DrawPipeline::Text => "gk_text",
            DrawPipeline::Pattern => "gk_pattern",
            DrawPipeline::Custom(inner) => inner.as_str(),
        }
    }

    pub(crate) fn id_intern(&self) -> Intern<String> {
        match self {
            DrawPipeline::Pixel => "gk_pixel".to_string().into(),
            DrawPipeline::Shapes => "gk_shapes".to_string().into(),
            DrawPipeline::Images => "gk_images".to_string().into(),
            DrawPipeline::Text => "gk_text".to_string().into(),
            DrawPipeline::Pattern => "gk_pattern".to_string().into(),
            DrawPipeline::Custom(inner) => *inner,
        }
    }
}

impl From<&str> for DrawPipeline {
    fn from(value: &str) -> Self {
        Self::Custom(Intern::new(value.to_string()))
    }
}

struct CachedBindGroup {
    signal: DropSignal,
    bind: BindGroup,
}

impl CachedBindGroup {
    fn expired(&self) -> bool {
        self.signal.is_expired()
    }
}

pub(crate) struct Painter2D {
    pub pipelines: HashMap<Intern<String>, PipelineContext>,
    pub ubo: Buffer,
    pub vbo: Buffer,
    pub ebo: Buffer,
    pub dummy_sprite_bg: Option<BindGroup>,
    pub sprites_cache: HashMap<SpriteId, CachedBindGroup>,
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
            ubo,
            vbo,
            ebo,
            dummy_sprite_bg: None,
            sprites_cache: Default::default(),
        };

        painter.add_pipeline(
            DrawPipeline::Shapes.id(),
            create_shapes_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.add_pipeline(
            DrawPipeline::Images.id(),
            create_images_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.add_pipeline(
            DrawPipeline::Text.id(),
            create_text_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.add_pipeline(
            DrawPipeline::Pattern.id(),
            create_pattern_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        // we need this to create custom shaders
        let dummy_sprite_bg = {
            let layout = painter
                .pipelines
                .get(&Intern::new(DrawPipeline::Images.id().to_string()))
                .map(|ctx| ctx.pipeline.bind_group_layout_ref(1).unwrap())
                .unwrap();

            let texture = gfx::create_texture().with_empty_size(1, 1).build().unwrap();

            let sampler = gfx::create_sampler().build().unwrap();

            gfx::create_bind_group()
                .with_label("Dummy Sprite Bind Group")
                .with_layout(layout)
                .with_texture(0, &texture)
                .with_sampler(1, &sampler)
                .build()
                .unwrap()
        };

        painter.dummy_sprite_bg = Some(dummy_sprite_bg);

        painter
    }
}

impl Painter2D {
    pub fn pip_resources(&self) -> PipelineResources {
        PipelineResources {
            ubo: &self.ubo,
            vbo: &self.vbo,
            ebo: &self.ebo,

            // this should be there
            sprite_bind_group: self.dummy_sprite_bg.as_ref().unwrap(),
        }
    }
    pub fn add_pipeline(&mut self, id: &str, pip: PipelineContext) -> Option<PipelineContext> {
        self.pipelines.insert(id.to_string().into(), pip)
    }

    pub fn remove_pipeline(&mut self, id: &str) -> Option<PipelineContext> {
        self.pipelines.remove(&id.to_string().into())
    }

    pub fn ctx(&self, id: &Intern<String>) -> Option<&PipelineContext> {
        self.pipelines.get(id)
    }

    pub fn cached_bind_group_for(&mut self, pip: &RenderPipeline, sprite: &Sprite) -> BindGroup {
        self.sprites_cache
            .entry(sprite.id())
            .or_insert_with(|| {
                let bind = gfx::create_bind_group()
                    .with_layout(pip.bind_group_layout_ref(1).unwrap())
                    .with_texture(0, sprite.texture())
                    .with_sampler(1, sprite.sampler())
                    .build()
                    .unwrap(); // TODO raise error?

                let signal = sprite.drop_observer.signal();
                CachedBindGroup { signal, bind }
            })
            .bind
            .clone()
    }

    pub fn clean(&mut self) {
        self.sprites_cache.retain(|k, v| !v.expired());
    }
}

pub(crate) fn get_2d_painter() -> AtomicRef<'static, Painter2D> {
    PAINTER_2D.borrow()
}

pub(crate) fn get_mut_2d_painter() -> AtomicRefMut<'static, Painter2D> {
    PAINTER_2D.borrow_mut()
}
