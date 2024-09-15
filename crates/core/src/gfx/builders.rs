use crate::backend::{get_mut_backend, BackendImpl, GfxBackendImpl};
use crate::gfx::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutRef, BlendMode,
    Buffer, BufferDescriptor, BufferUsage, ColorMask, CompareMode, CullMode, DepthStencil,
    IndexFormat, Primitive, RenderPipeline, RenderPipelineDescriptor, RenderTexture,
    RenderTextureDescriptor, Sampler, SamplerDescriptor, Stencil, Texture, TextureData,
    TextureDescriptor, TextureFilter, TextureFormat, TextureWrap, VertexLayout,
};
use image::EncodableLayout;

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

    pub fn with_compatible_texture(mut self, format: TextureFormat) -> Self {
        self.desc.compatible_textures.push(format);
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

pub struct BindGroupBuilder<'a> {
    desc: BindGroupDescriptor<'a>,
}

impl<'a> BindGroupBuilder<'a> {
    pub(crate) fn new() -> Self {
        let desc = Default::default();
        Self { desc }
    }

    pub fn with_layout(mut self, layout: &'a BindGroupLayoutRef) -> Self {
        self.desc.layout = Some(layout);
        self
    }

    pub fn with_texture(mut self, location: u32, texture: &'a Texture) -> Self {
        self.desc
            .entry
            .push(BindGroupEntry::Texture { location, texture });

        self
    }

    pub fn with_sampler(mut self, location: u32, sampler: &'a Sampler) -> Self {
        self.desc
            .entry
            .push(BindGroupEntry::Sampler { location, sampler });
        self
    }

    pub fn with_uniform(mut self, location: u32, buffer: &'a Buffer) -> Self {
        self.desc
            .entry
            .push(BindGroupEntry::Uniform { location, buffer });
        self
    }

    pub fn build(self) -> Result<BindGroup, String> {
        let Self { desc } = self;
        get_mut_backend().gfx().create_bind_group(desc)
    }
}

pub struct BufferWriteBuilder<'a> {
    buffer: &'a Buffer,
    offset: u64,
    data: Option<&'a [u8]>,
}

impl<'a> BufferWriteBuilder<'a> {
    pub fn new(buffer: &'a Buffer) -> Self {
        Self {
            buffer,
            offset: 0,
            data: None,
        }
    }

    pub fn with_data<D: bytemuck::Pod>(mut self, data: &'a [D]) -> Self {
        self.data = Some(bytemuck::cast_slice(data));
        self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn build(self) -> Result<(), String> {
        let Self {
            buffer,
            offset,
            data,
        } = self;

        if !buffer.is_writable() {
            return Err("Buffer is not Writable".to_string());
        }

        let data = data.unwrap_or(&[]);
        get_mut_backend().gfx().write_buffer(buffer, offset, data)
    }
}

pub struct SamplerBuilder<'a> {
    desc: SamplerDescriptor<'a>,
}
impl<'a> SamplerBuilder<'a> {
    pub fn new() -> Self {
        let desc = SamplerDescriptor::default();
        Self { desc }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_wrap_x(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_x = wrap;
        self
    }

    pub fn with_wrap_y(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_y = wrap;
        self
    }

    pub fn with_wrap_z(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_z = wrap;
        self
    }

    pub fn with_min_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.min_filter = filter;
        self
    }

    pub fn with_mag_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.mag_filter = filter;
        self
    }

    pub fn with_mipmap_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.mipmap_filter = Some(filter);
        self
    }

    pub fn build(self) -> Result<Sampler, String> {
        let Self { desc } = self;
        get_mut_backend().gfx().create_sampler(desc)
    }
}

enum TextureRawData<'a> {
    Empty,
    Image(&'a [u8]),
    Raw {
        bytes: &'a [u8],
        width: u32,
        height: u32,
    },
}

pub struct TextureBuilder<'a> {
    desc: TextureDescriptor<'a>,
    data: TextureRawData<'a>,
}

impl<'a> TextureBuilder<'a> {
    pub fn new() -> Self {
        let desc = TextureDescriptor::default();
        let data = TextureRawData::Empty;
        Self { desc, data }
    }

    pub fn from_image(mut self, image: &'a [u8]) -> Self {
        self.data = TextureRawData::Image(image);
        self
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.desc.format = format;
        self
    }

    pub fn with_write_flag(mut self, writable: bool) -> Self {
        self.desc.write = writable;
        self
    }

    pub fn build(self) -> Result<Texture, String> {
        let Self { desc, data } = self;
        match data {
            TextureRawData::Empty => get_mut_backend().gfx().create_texture(desc, None),
            TextureRawData::Image(bytes) => {
                let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
                let rgba = img.to_rgba8();
                get_mut_backend().gfx().create_texture(
                    desc,
                    Some(TextureData {
                        bytes: rgba.as_bytes(),
                        width: rgba.width(),
                        height: rgba.height(),
                    }),
                )
            }
            TextureRawData::Raw {
                bytes,
                width,
                height,
            } => get_mut_backend().gfx().create_texture(
                desc,
                Some(TextureData {
                    bytes,
                    width,
                    height,
                }),
            ),
        }
    }
}

pub struct RenderTextureBuilder<'a> {
    desc: RenderTextureDescriptor<'a>,
}

impl<'a> RenderTextureBuilder<'a> {
    pub fn new() -> Self {
        let desc = RenderTextureDescriptor::default();
        Self { desc }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_depth(mut self, enabled: bool) -> Self {
        self.desc.depth = enabled;
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.desc.width = width;
        self.desc.height = height;
        self
    }

    pub fn build(self) -> Result<RenderTexture, String> {
        let Self { desc } = self;

        let no_size = self.desc.width == 0 || self.desc.height == 0;
        if no_size {
            return Err(format!(
                "RenderTexture size cannot be zero 'width={}', 'height={}'",
                self.desc.width, self.desc.height
            ));
        }

        get_mut_backend().gfx().create_render_texture(desc)
    }
}
