use crate::backend::gfx::BindGroup;
use crate::gfx::consts::{
    MAX_BIND_GROUPS_PER_PIPELINE, MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE, MAX_VERTEX_BUFFERS,
};
use crate::gfx::pipeline::ClearOptions;
use crate::gfx::{Buffer, Color, RenderPipeline};
use arrayvec::ArrayVec;
use glam::{vec2, Vec2};
use std::ops::Range;

const MAX_BUFFERS: usize = MAX_VERTEX_BUFFERS + MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE + 1;

#[derive(Default)]
pub(crate) struct RPassVertices {
    pub(crate) range: Range<u32>,
    pub(crate) instances: Option<u32>,
}

#[derive(Default)]
pub struct RenderPass<'a> {
    pub(crate) size: Option<Vec2>,
    pub(crate) pipeline: Option<&'a RenderPipeline>,
    pub(crate) buffers: ArrayVec<&'a Buffer, MAX_BUFFERS>,
    pub(crate) clear_options: ClearOptions,
    pub(crate) vertices: Vec<RPassVertices>,
    pub(crate) bind_groups: ArrayVec<&'a BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub(crate) stencil_ref: Option<u8>,
}

impl<'a> RenderPass<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(&mut self, width: f32, height: f32) -> &mut Self {
        self.size = Some(vec2(width, height));
        self
    }

    pub fn stencil_reference(&mut self, reference: u8) -> &mut Self {
        self.stencil_ref = Some(reference);
        self
    }

    pub fn clear_color(&mut self, color: Color) -> &mut Self {
        self.clear_options.color = Some(color);
        self
    }

    pub fn clear_depth(&mut self, depth: f32) -> &mut Self {
        self.clear_options.depth = Some(depth);
        self
    }

    pub fn clear_stencil(&mut self, stencil: u32) -> &mut Self {
        self.clear_options.stencil = Some(stencil);
        self
    }

    pub fn pipeline(&mut self, pipeline: &'a RenderPipeline) -> &mut Self {
        self.pipeline = Some(pipeline);
        self
    }

    pub fn buffers(&mut self, buffers: &[&'a Buffer]) -> &mut Self {
        self.buffers.try_extend_from_slice(buffers).unwrap();
        self
    }

    pub fn bindings(&mut self, groups: &[&'a BindGroup]) -> &mut Self {
        self.bind_groups.try_extend_from_slice(groups).unwrap();
        self
    }

    pub fn draw(&mut self, vertices: Range<u32>) -> &mut Self {
        self.vertices.push(RPassVertices {
            range: vertices,
            instances: None,
        });
        self
    }

    pub fn draw_instanced(&mut self, vertices: Range<u32>, instances: u32) -> &mut Self {
        self.vertices.push(RPassVertices {
            range: vertices,
            instances: Some(instances),
        });
        self
    }
}

#[derive(Default)]
pub struct Renderer<'a> {
    pub(crate) passes: Vec<RenderPass<'a>>,
}

impl<'a> Renderer<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_pass(&mut self, rpass: RenderPass<'a>) {
        self.passes.push(rpass);
    }

    pub fn begin_pass(&mut self) -> &mut RenderPass<'a> {
        self.passes.push(RenderPass::default());
        self.passes.last_mut().unwrap()
    }
}
