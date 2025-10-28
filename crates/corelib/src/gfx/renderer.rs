use crate::{
    backend::{BackendImpl, GfxBackendImpl, get_mut_backend, gfx::BindGroup},
    gfx::{
        Buffer, Color, RenderPipeline, RenderTexture,
        consts::{
            MAX_BIND_GROUPS_PER_PIPELINE, MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE, MAX_VERTEX_BUFFERS,
        },
        pipeline::ClearOptions,
    },
    math::{Vec2, vec2},
};
use arrayvec::ArrayVec;
use smallvec::SmallVec;
use std::{
    collections::Bound,
    ops::{Range, RangeBounds},
};

const MAX_BUFFERS: usize = MAX_VERTEX_BUFFERS + MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE + 1;

#[derive(Default, Clone, Debug)]
pub(crate) struct RPassVertices {
    pub(crate) range: Range<u32>,
    pub(crate) instances: Option<u32>,
}

#[derive(Default, Clone)]
pub struct RenderPass<'a> {
    pub(crate) size: Option<Vec2>,
    pub(crate) pipeline: Option<&'a RenderPipeline>,
    pub(crate) buffers: ArrayVec<(&'a Buffer, Range<u64>), MAX_BUFFERS>,
    pub(crate) clear_options: ClearOptions,
    pub(crate) vertices: SmallVec<RPassVertices, 10>,
    pub(crate) bind_groups: ArrayVec<&'a BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub(crate) stencil_ref: Option<u8>,
    pub(crate) scissors: Option<[u32; 4]>,
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

    pub fn scissors(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
        self.scissors = Some([x, y, width, height]);
        self
    }

    pub fn pipeline(&mut self, pipeline: &'a RenderPipeline) -> &mut Self {
        self.pipeline = Some(pipeline);
        self
    }

    pub fn buffers_with_offset<S: RangeBounds<u64>>(
        &mut self,
        buffers: &[(&'a Buffer, S)],
    ) -> &mut Self {
        self.buffers.extend(buffers.iter().map(|(b, r)| {
            let start = match r.start_bound() {
                Bound::Included(n) => *n,
                Bound::Excluded(n) => *n + 1,
                Bound::Unbounded => 0,
            };
            let end = match r.end_bound() {
                Bound::Included(n) => *n + 1,
                Bound::Excluded(n) => *n,
                Bound::Unbounded => b.size() as _,
            };

            (*b, start..end)
        }));
        self
    }

    pub fn buffers(&mut self, buffers: &[&'a Buffer]) -> &mut Self {
        self.buffers
            .extend(buffers.iter().map(|b| (*b, 0..b.size() as _)));
        self
    }

    pub fn bindings(&mut self, groups: &[&'a BindGroup]) -> &mut Self {
        self.bind_groups
            .try_extend_from_slice(groups)
            .map_err(|_| {
                format!(
                    "Exceeded the maximum number ({}) of supported BindGroups.",
                    self.bind_groups.capacity()
                )
            })
            .unwrap();
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

#[derive(Default, Clone)]
pub struct Renderer<'a> {
    pub(crate) passes: SmallVec<RenderPass<'a>, 20>,
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

pub trait AsRenderer {
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String>;

    fn flush(&self, renderer: &Renderer, target: Option<&RenderTexture>) -> Result<(), String> {
        match target {
            None => get_mut_backend().gfx().render(renderer),
            Some(rt) => get_mut_backend().gfx().render_to(rt, renderer),
        }
    }
}

impl AsRenderer for Renderer<'_> {
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        self.flush(self, target)
    }
}
