use crate::backend::{get_mut_backend, BackendImpl, GfxBackendImpl};
use crate::gfx::{
    BindGroupLayout, BlendMode, Buffer, BufferDescriptor, BufferUsage, ColorMask, CompareMode,
    CullMode, DepthStencil, IndexFormat, Primitive, RenderPipeline, RenderPipelineDescriptor,
    Stencil, VertexLayout,
};

pub struct RenderPipelineBuilder<'a> {
    desc: RenderPipelineDescriptor<'a>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub(crate) fn new(shader: &'a str) -> Self {
        let desc = RenderPipelineDescriptor {
            shader,
            ..Default::default()
        };
        Self { desc }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_vertex_layout(mut self, layout: VertexLayout) -> Self {
        self.desc.vertex_layout.push(layout);
        self
    }

    pub fn with_index_format(mut self, format: IndexFormat) -> Self {
        self.desc.index_format = format;
        self
    }

    pub fn with_primitive(mut self, primitive: Primitive) -> Self {
        self.desc.primitive = primitive;
        self
    }

    pub fn with_bind_group_layout(mut self, layout: BindGroupLayout) -> Self {
        self.desc.bind_group_layout.push(layout);
        self
    }

    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.desc.blend_mode = Some(mode);
        self
    }

    pub fn with_cull_mode(mut self, mode: CullMode) -> Self {
        self.desc.cull_mode = Some(mode);
        self
    }

    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self {
        self.desc.vs_entry = Some(entry);
        self
    }

    pub fn with_fragment_entry(mut self, entry: &'a str) -> Self {
        self.desc.fs_entry = Some(entry);
        self
    }

    pub fn with_depth_stencil(mut self, mode: CompareMode, write: bool) -> Self {
        self.desc.depth_stencil = Some(DepthStencil {
            write,
            compare: mode,
        });
        self
    }

    pub fn with_stencil(mut self, opts: Stencil) -> Self {
        self.desc.stencil = Some(opts);
        self
    }

    pub fn with_color_mask(mut self, mask: ColorMask) -> Self {
        self.desc.color_mask = mask;
        self
    }

    pub fn build(self) -> Result<RenderPipeline, String> {
        let Self { desc } = self;
        get_mut_backend().gfx().create_render_pipeline(desc)
    }
}

pub struct BufferBuilder<'a> {
    desc: BufferDescriptor<'a>,
}

impl<'a> BufferBuilder<'a> {
    pub(crate) fn new<D: bytemuck::Pod>(usage: BufferUsage, data: &'a [D]) -> Self {
        let desc = BufferDescriptor {
            content: bytemuck::cast_slice(data),
            usage,
            ..Default::default()
        };
        Self { desc }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_write_flag(mut self, writable: bool) -> Self {
        self.desc.write = writable;
        self
    }

    pub fn build(self) -> Result<Buffer, String> {
        let Self { desc } = self;
        get_mut_backend().gfx().create_buffer(desc)
    }
}
