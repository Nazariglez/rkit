use crate::gfx::consts::{
    MAX_BIND_GROUPS_PER_PIPELINE, MAX_PIPELINE_COMPATIBLE_TEXTURES, MAX_VERTEX_BUFFERS,
};
use crate::gfx::{
    BindGroupLayoutRef, ColorMask, CullMode, DepthStencil, PipelineId, Primitive, Stencil,
    VertexLayout,
};
use arrayvec::ArrayVec;
use std::sync::Arc;
use wgpu::{ColorWrites, RenderPipeline as RawRenderPipeline};

#[derive(Clone)]
pub(crate) struct PipelineRecipe {
    pub label: Option<String>,
    pub shader: wgpu::ShaderModule,
    pub vertex_layout: ArrayVec<VertexLayout, MAX_VERTEX_BUFFERS>,
    pub primitive: Primitive,
    pub cull_mode: Option<CullMode>,
    pub depth: Option<DepthStencil>,
    pub stencil: Option<Stencil>,
    pub targets: ArrayVec<Option<wgpu::ColorTargetState>, MAX_PIPELINE_COMPATIBLE_TEXTURES>,
    pub vs_entry: Option<String>,
    pub fs_entry: Option<String>,
}

pub(crate) struct PipelineInner {
    pub id: PipelineId,
    pub raw: RawRenderPipeline,
    pub index_format: wgpu::IndexFormat,
    pub uses_depth: bool,
    pub uses_stencil: bool,
    pub bind_group_layout: ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub recipe: PipelineRecipe,
}

#[derive(Clone)]
pub struct RenderPipeline {
    pub(crate) inner: Arc<PipelineInner>,
}

impl PartialEq for RenderPipeline {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}

impl RenderPipeline {
    pub fn id(&self) -> PipelineId {
        self.inner.id
    }

    #[doc(hidden)]
    pub fn uses_stencil(&self) -> bool {
        self.inner.uses_stencil
    }

    #[doc(hidden)]
    pub fn uses_depth_stencil(&self) -> bool {
        self.inner.uses_depth || self.inner.uses_stencil
    }

    pub fn bind_group_layout_ref(&self, index: u32) -> Result<&BindGroupLayoutRef, String> {
        self.inner
            .bind_group_layout
            .get(index as usize)
            .ok_or_else(|| format!("Invalid Bind Group '{index}' in pipeline"))
    }
}

impl ColorMask {
    pub(crate) fn as_wgpu(&self) -> wgpu::ColorWrites {
        let mut raw_mask = ColorWrites::empty();
        if self.r {
            raw_mask |= ColorWrites::RED;
        }

        if self.g {
            raw_mask |= ColorWrites::GREEN;
        }

        if self.b {
            raw_mask |= ColorWrites::BLUE;
        }

        if self.a {
            raw_mask |= ColorWrites::ALPHA;
        }

        raw_mask
    }
}
