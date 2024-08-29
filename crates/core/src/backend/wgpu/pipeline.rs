use crate::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use crate::gfx::{BindGroupLayoutRef, PipelineId};
use arrayvec::ArrayVec;
use std::sync::Arc;
use wgpu::RenderPipeline as RawRenderPipeline;

#[derive(Clone)]
pub struct RenderPipeline {
    pub(crate) id: PipelineId,
    pub(crate) raw: Arc<RawRenderPipeline>,
    pub(crate) index_format: wgpu::IndexFormat,
    pub(crate) uses_depth: bool,
    pub(crate) uses_stencil: bool,
    pub(crate) bind_group_layout: ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>,
}
