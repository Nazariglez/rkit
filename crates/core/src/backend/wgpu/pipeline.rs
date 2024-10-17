use crate::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use crate::gfx::{BindGroupLayoutRef, ColorMask, PipelineId};
use arrayvec::ArrayVec;
use std::sync::Arc;
use wgpu::{ColorWrites, RenderPipeline as RawRenderPipeline};

#[derive(Clone)]
pub struct RenderPipeline {
    pub(crate) id: PipelineId,
    pub(crate) raw: Arc<RawRenderPipeline>,
    pub(crate) index_format: wgpu::IndexFormat,
    pub(crate) uses_depth: bool,
    pub(crate) uses_stencil: bool,
    pub(crate) bind_group_layout: ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>,
}

impl PartialEq for RenderPipeline {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl RenderPipeline {
    pub fn id(&self) -> PipelineId {
        self.id
    }

    pub fn bind_group_layout_ref(&self, index: u32) -> Result<&BindGroupLayoutRef, String> {
        self.bind_group_layout
            .get(index as usize)
            .ok_or_else(|| format!("Invalid Bind Group '{}' in pipeline", index))
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
