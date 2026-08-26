use super::{PipelineContext, create_shapes_2d_pipeline_ctx};
use crate::sprite::SpriteId;
use crate::{
    Sprite, clean_2d, create_images_2d_pipeline_ctx, create_pattern_2d_pipeline_ctx,
    create_text_2d_pipeline_ctx,
};
use arrayvec::ArrayVec;
use atomic_refcell::{AtomicRefCell, AtomicRefMut};
use corelib::{
    gfx::{
        self, BindGroup, BindGroupLayoutId, Buffer, ColorMask, CompareMode, RenderPipeline,
        Stencil, StencilAction, VertexFormat, VertexLayout, consts::MAX_BIND_GROUPS_PER_PIPELINE,
    },
    math::Mat4,
};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use utils::drop_signal::DropSignal;

const MASK_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4<f32> {
    return model.position;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0.0);
}
"#;

pub(crate) static PAINTER_2D: Lazy<AtomicRefCell<Painter2D>> = Lazy::new(|| {
    corelib::app::on_sys_post_update(clean_2d);

    AtomicRefCell::new(Painter2D::default())
});

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithreading
unsafe impl Sync for Painter2D {}
unsafe impl Send for Painter2D {}

pub struct PipelineResources<'a> {
    pub ubo: &'a Buffer,
    pub vbo: &'a Buffer,
    pub ebo: &'a Buffer,
    pub sprite_bind_group: &'a BindGroup,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DrawPipelineId {
    Shapes,
    Images,
    Text,
    Pattern,
    Custom(u64),
}

pub(crate) struct ResolvedPipeline<'a> {
    pub content: &'a Arc<ContentPipeline>,
    pub groups: ArrayVec<&'a BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub vertex_offset: usize,
    pub x_pos: usize,
    pub y_pos: usize,
    pub alpha_pos: Option<usize>,
    pub uses_stencil: bool,
}

pub(crate) struct CachedBindGroup {
    pub signal: DropSignal,
    pub bind: BindGroup,
}

impl CachedBindGroup {
    pub fn expired(&self) -> bool {
        self.signal.is_expired()
    }
}

pub(crate) struct ContentPipeline {
    base: PipelineContext,
    clipped: Result<RenderPipeline, String>,
}

impl ContentPipeline {
    pub(crate) fn base(&self) -> &RenderPipeline {
        &self.base.pipeline
    }

    pub(crate) fn clipped(&self) -> Result<&RenderPipeline, String> {
        self.clipped.as_ref().map_err(Clone::clone)
    }
}

struct MaskPipelines {
    push: RenderPipeline,
    pop: RenderPipeline,
}

pub(crate) struct Painter2D {
    pipelines: FxHashMap<DrawPipelineId, Arc<ContentPipeline>>,
    pip_ctx_id: u64,

    pub ubo: Buffer,
    pub vbo: Buffer,
    pub ebo: Buffer,
    pub dummy_sprite_bg: Option<BindGroup>,
    sprites_cache: FxHashMap<(BindGroupLayoutId, SpriteId), CachedBindGroup>,
    masks: Result<MaskPipelines, String>,
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
            pip_ctx_id: 0,
            ubo,
            vbo,
            ebo,
            dummy_sprite_bg: None,
            sprites_cache: Default::default(),
            masks: create_mask_pipelines(),
        };

        painter.set_pipeline(
            &DrawPipelineId::Shapes,
            create_shapes_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.set_pipeline(
            &DrawPipelineId::Images,
            create_images_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.set_pipeline(
            &DrawPipelineId::Text,
            create_text_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        painter.set_pipeline(
            &DrawPipelineId::Pattern,
            create_pattern_2d_pipeline_ctx(&painter.ubo).unwrap(),
        );

        // we need this to create custom shaders
        let dummy_sprite_bg = {
            let layout = painter
                .pipelines
                .get(&DrawPipelineId::Images)
                .map(|registered| registered.base.pipeline.bind_group_layout_ref(1).unwrap())
                .unwrap();

            let texture = gfx::create_texture()
                .with_label("Draw2D Dummy Texture")
                .with_empty_size(1, 1)
                .build()
                .unwrap();

            let sampler = gfx::create_sampler()
                .with_label("Draw2D Dummy Sampler")
                .build()
                .unwrap();

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
    pub fn pip_resources(&self) -> PipelineResources<'_> {
        PipelineResources {
            ubo: &self.ubo,
            vbo: &self.vbo,
            ebo: &self.ebo,

            // this should be there
            sprite_bind_group: self.dummy_sprite_bg.as_ref().unwrap(),
        }
    }
    pub fn add_pipeline(&mut self, pipeline: PipelineContext) -> DrawPipelineId {
        let id = DrawPipelineId::Custom(self.pip_ctx_id);
        self.pip_ctx_id += 1;
        self.pipelines.insert(id, register_pipeline(pipeline));
        id
    }

    pub fn set_pipeline(
        &mut self,
        id: &DrawPipelineId,
        pipeline: PipelineContext,
    ) -> Option<PipelineContext> {
        self.pipelines
            .insert(*id, register_pipeline(pipeline))
            .map(|content| content.base.clone())
    }

    pub fn remove_pipeline(&mut self, id: &DrawPipelineId) -> Option<PipelineContext> {
        self.pipelines
            .remove(id)
            .map(|content| content.base.clone())
    }

    pub(crate) fn resolve_pipeline(
        &mut self,
        id: DrawPipelineId,
        sprite: Option<&Sprite>,
    ) -> Result<ResolvedPipeline<'_>, String> {
        let Self {
            pipelines,
            sprites_cache,
            ..
        } = self;
        let registered = pipelines
            .get(&id)
            .ok_or_else(|| format!("Missing pipeline '{id:?}'"))?;
        let pipeline = &registered.base.pipeline;
        let mut groups = registered
            .base
            .groups
            .iter()
            .collect::<ArrayVec<_, MAX_BIND_GROUPS_PER_PIPELINE>>();

        if let Some(sprite) = sprite {
            let layout = pipeline.bind_group_layout_ref(1)?;
            let bind_group = &sprites_cache
                .entry((layout.id(), sprite.id()))
                .or_insert_with(|| {
                    let bind = gfx::create_bind_group()
                        .with_label(&format!("Draw2D Sprite BindGroup for {:?}", sprite.id()))
                        .with_layout(layout)
                        .with_texture(0, sprite.texture())
                        .with_sampler(1, sprite.sampler())
                        .build()
                        .unwrap();
                    CachedBindGroup {
                        signal: sprite.drop_observer.signal(),
                        bind,
                    }
                })
                .bind;
            if groups.len() > 1 {
                groups[1] = bind_group;
            } else {
                groups.push(bind_group);
            }
        }

        Ok(ResolvedPipeline {
            content: registered,
            groups,
            vertex_offset: registered.base.vertex_offset,
            x_pos: registered.base.x_pos,
            y_pos: registered.base.y_pos,
            alpha_pos: registered.base.alpha_pos,
            uses_stencil: registered.base.pipeline.uses_stencil(),
        })
    }

    pub(crate) fn mask_pipelines(&self) -> Result<(&RenderPipeline, &RenderPipeline), String> {
        self.masks
            .as_ref()
            .map(|masks| (&masks.push, &masks.pop))
            .map_err(Clone::clone)
    }

    pub fn clean(&mut self) {
        self.sprites_cache.retain(|_k, v| !v.expired());
    }
}

fn register_pipeline(base: PipelineContext) -> Arc<ContentPipeline> {
    let clipped = if base.pipeline.uses_stencil() {
        Err("A stencil-owning Draw2D pipeline cannot be used inside a rounded clip".to_string())
    } else {
        gfx::create_stencil_variant(
            &base.pipeline,
            Stencil {
                stencil_fail: StencilAction::Keep,
                depth_fail: StencilAction::Keep,
                pass: StencilAction::Keep,
                compare: CompareMode::Equal,
                read_mask: 0xff,
                write_mask: 0x00,
                reference: 0,
            },
        )
    };
    Arc::new(ContentPipeline { base, clipped })
}

fn create_mask_pipelines() -> Result<MaskPipelines, String> {
    Ok(MaskPipelines {
        push: create_mask_pipeline(StencilAction::Increment)?,
        pop: create_mask_pipeline(StencilAction::Decrement)?,
    })
}

fn create_mask_pipeline(action: StencilAction) -> Result<RenderPipeline, String> {
    gfx::create_render_pipeline(MASK_SHADER)
        .with_label("Draw2D clip mask pipeline")
        .with_vertex_layout(VertexLayout::new().with_attr(0, VertexFormat::Float32x4))
        .with_color_mask(ColorMask::NONE)
        .with_stencil(Stencil {
            stencil_fail: StencilAction::Keep,
            depth_fail: StencilAction::Keep,
            pass: action,
            compare: CompareMode::Equal,
            read_mask: 0xff,
            write_mask: 0xff,
            reference: 0,
        })
        .build()
}

pub(crate) fn get_mut_2d_painter() -> AtomicRefMut<'static, Painter2D> {
    PAINTER_2D.borrow_mut()
}
