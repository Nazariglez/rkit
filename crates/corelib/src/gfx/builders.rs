use crate::backend::{BackendImpl, GfxBackendImpl, get_mut_backend};
use crate::gfx::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutRef, BlendMode,
    Buffer, BufferDescriptor, BufferUsage, ColorMask, CompareMode, CullMode, DepthStencil,
    IndexFormat, Primitive, RenderPipeline, RenderPipelineDescriptor, RenderTexture,
    RenderTextureDescriptor, Sampler, SamplerDescriptor, Stencil, Texture, TextureDescriptor,
    TextureFilter, TextureFormat, TextureMipLevel, TextureUpload, TextureWrap, VertexLayout,
};
use glam::{UVec2, uvec2};
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

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
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

#[derive(Default)]
pub struct SamplerBuilder<'a> {
    desc: SamplerDescriptor<'a>,
}
impl<'a> SamplerBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    #[inline]
    pub fn with_wrap_x(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_x = wrap;
        self
    }

    #[inline]
    pub fn with_wrap_y(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_y = wrap;
        self
    }

    #[inline]
    pub fn with_wrap_z(mut self, wrap: TextureWrap) -> Self {
        self.desc.wrap_z = wrap;
        self
    }

    #[inline]
    pub fn with_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.min_filter = filter;
        self.desc.mag_filter = filter;
        self
    }

    #[inline]
    pub fn with_min_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.min_filter = filter;
        self
    }

    #[inline]
    pub fn with_mag_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.mag_filter = filter;
        self
    }

    #[inline]
    pub fn with_mipmap_filter(mut self, filter: TextureFilter) -> Self {
        self.desc.mipmap_filter = filter;
        self
    }

    #[inline]
    pub fn build(self) -> Result<Sampler, String> {
        let Self { desc } = self;
        get_mut_backend().gfx().create_sampler(desc)
    }
}

enum TextureSource<'a> {
    Empty { width: u32, height: u32 },
    Image(&'a [u8]),
    Raw(TextureMipLevel<'a>),
    Mipmaps(&'a [TextureMipLevel<'a>]),
}

pub struct TextureWriteBuilder<'a> {
    offset: UVec2,
    size: UVec2,
    tex: &'a Texture,
    data: Option<&'a [u8]>,
}

impl<'a> TextureWriteBuilder<'a> {
    pub fn new(texture: &'a Texture) -> Self {
        let size = uvec2(texture.width() as _, texture.height() as _);
        Self {
            offset: Default::default(),
            size,
            tex: texture,
            data: None,
        }
    }

    pub fn from_data(mut self, data: &'a [u8]) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_offset(mut self, offset: UVec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_size(mut self, size: UVec2) -> Self {
        self.size = size;
        self
    }

    pub fn build(self) -> Result<(), String> {
        let data = self
            .data
            .ok_or_else(|| "Texture write requires pixel data".to_string())?;
        if !self.tex.is_writable() {
            return Err(format!("Texture '{:?}' is not writable", self.tex.id()));
        }
        if self.size.x == 0 || self.size.y == 0 {
            return Err("Texture write size must be nonzero".to_string());
        }
        let end = self
            .offset
            .checked_add(self.size)
            .ok_or_else(|| "Texture write bounds overflow".to_string())?;
        let bounds = uvec2(self.tex.width() as _, self.tex.height() as _);
        if end.x > bounds.x || end.y > bounds.y {
            return Err(format!(
                "Texture write region {}x{} at {},{} exceeds {}x{} texture",
                self.size.x, self.size.y, self.offset.x, self.offset.y, bounds.x, bounds.y
            ));
        }
        let expected = self
            .tex
            .format()
            .byte_len(self.size.x, self.size.y)
            .ok_or_else(|| {
                format!(
                    "Texture format {:?} does not support pixel writes",
                    self.tex.format()
                )
            })?;
        if data.len() != expected {
            return Err(format!(
                "Texture write requires {expected} bytes but got {}",
                data.len()
            ));
        }
        get_mut_backend()
            .gfx()
            .write_texture(self.tex, self.offset, self.size, data)
    }
}

pub struct TextureBuilder<'a> {
    desc: TextureDescriptor<'a>,
    source: TextureSource<'a>,
    mipmaps: bool,
}

impl Default for TextureBuilder<'_> {
    fn default() -> Self {
        Self {
            desc: TextureDescriptor::default(),
            source: TextureSource::Empty {
                width: 1,
                height: 1,
            },
            mipmaps: false,
        }
    }
}

impl<'a> TextureBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_image(mut self, image: &'a [u8]) -> Self {
        self.source = TextureSource::Image(image);
        self
    }

    pub fn from_bytes(mut self, bytes: &'a [u8], width: u32, height: u32) -> Self {
        self.source = TextureSource::Raw(TextureMipLevel::new(bytes, width, height));
        self
    }

    pub fn from_mipmaps(mut self, mipmaps: &'a [TextureMipLevel<'a>]) -> Self {
        self.source = TextureSource::Mipmaps(mipmaps);
        self
    }

    pub fn with_empty_size(mut self, width: u32, height: u32) -> Self {
        if matches!(self.source, TextureSource::Empty { .. }) {
            self.source = TextureSource::Empty { width, height };
        }
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

    pub fn with_mipmaps(mut self) -> Self {
        self.mipmaps = true;
        self
    }

    pub fn build(self) -> Result<Texture, String> {
        let Self {
            desc,
            source,
            mipmaps,
        } = self;
        if mipmaps && matches!(source, TextureSource::Mipmaps(_)) {
            return Err(texture_error(
                desc.label,
                "cannot combine manual and generated mipmaps",
            ));
        }

        match source {
            TextureSource::Empty { width, height } => {
                validate_size(desc.label, width, height)?;
                if desc.format.is_depth() {
                    if mipmaps {
                        return Err(texture_error(
                            desc.label,
                            "depth formats cannot generate mipmaps",
                        ));
                    }
                    return get_mut_backend().gfx().create_texture(
                        desc,
                        TextureUpload::Single(TextureMipLevel::new(&[], width, height)),
                    );
                }
                texture_byte_len(desc.format, width, height)
                    .map_err(|message| texture_error(desc.label, &message))?;
                create_texture(desc, TextureMipLevel::new(&[], width, height), mipmaps)
            }
            TextureSource::Image(bytes) => {
                if !matches!(
                    desc.format,
                    TextureFormat::Rgba8UNorm | TextureFormat::Rgba8UNormSrgb
                ) {
                    return Err(texture_error(
                        desc.label,
                        "decoded images require Rgba8UNorm or Rgba8UNormSrgb",
                    ));
                }
                let image = image::load_from_memory(bytes).map_err(|err| {
                    texture_error(desc.label, &format!("could not decode image: {err}"))
                })?;
                let rgba = image.to_rgba8();
                let level = TextureMipLevel::new(rgba.as_bytes(), rgba.width(), rgba.height());
                create_texture(desc, level, mipmaps)
            }
            TextureSource::Raw(level) => {
                validate_level(desc, level, 0)?;
                create_texture(desc, level, mipmaps)
            }
            TextureSource::Mipmaps(levels) => {
                validate_mipmaps(desc, levels)?;
                get_mut_backend()
                    .gfx()
                    .create_texture(desc, TextureUpload::Levels(levels))
            }
        }
    }
}

fn create_texture(
    desc: TextureDescriptor<'_>,
    level: TextureMipLevel<'_>,
    mipmaps: bool,
) -> Result<Texture, String> {
    let upload = if mipmaps {
        TextureUpload::Generate(level)
    } else {
        TextureUpload::Single(level)
    };
    get_mut_backend().gfx().create_texture(desc, upload)
}

fn validate_mipmaps(
    desc: TextureDescriptor<'_>,
    levels: &[TextureMipLevel<'_>],
) -> Result<(), String> {
    let Some(base) = levels.first().copied() else {
        return Err(texture_error(desc.label, "mipmap chain cannot be empty"));
    };
    validate_level(desc, base, 0)?;

    let mut previous = base;
    for (index, level) in levels.iter().copied().enumerate().skip(1) {
        if previous.width == 1 && previous.height == 1 {
            return Err(texture_error(
                desc.label,
                &format!("mip {index} follows the terminal 1x1 level"),
            ));
        }
        let width = (previous.width / 2).max(1);
        let height = (previous.height / 2).max(1);
        if level.width != width || level.height != height {
            return Err(texture_error(
                desc.label,
                &format!(
                    "mip {index} must be {width}x{height} but is {}x{}",
                    level.width, level.height
                ),
            ));
        }
        validate_level(desc, level, index)?;
        previous = level;
    }
    Ok(())
}

fn validate_level(
    desc: TextureDescriptor<'_>,
    level: TextureMipLevel<'_>,
    index: usize,
) -> Result<(), String> {
    if level.width == 0 || level.height == 0 {
        return Err(texture_error(
            desc.label,
            &format!(
                "mip {index} dimensions must be nonzero but are {}x{}",
                level.width, level.height
            ),
        ));
    }
    let expected = texture_byte_len(desc.format, level.width, level.height)
        .map_err(|message| texture_error(desc.label, &format!("mip {index} {message}")))?;
    if level.bytes.len() != expected {
        return Err(texture_error(
            desc.label,
            &format!(
                "mip {index} requires {expected} bytes but got {}",
                level.bytes.len()
            ),
        ));
    }
    Ok(())
}

fn validate_size(label: Option<&str>, width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err(texture_error(
            label,
            &format!("dimensions must be nonzero but are {width}x{height}"),
        ));
    }
    Ok(())
}

fn texture_byte_len(format: TextureFormat, width: u32, height: u32) -> Result<usize, String> {
    if format.bytes_per_texel().is_none() {
        return Err(format!("format {format:?} does not support color uploads"));
    }
    format
        .byte_len(width, height)
        .ok_or_else(|| format!("byte size for {width}x{height} {format:?} overflows"))
}

fn texture_error(label: Option<&str>, message: &str) -> String {
    match label {
        Some(label) => format!("Texture '{label}' {message}"),
        None => format!("Texture {message}"),
    }
}

#[derive(Default)]
pub struct RenderTextureBuilder<'a> {
    desc: RenderTextureDescriptor<'a>,
}

impl<'a> RenderTextureBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    #[inline]
    pub fn with_depth(mut self, enabled: bool) -> Self {
        self.desc.depth = enabled;
        self
    }

    #[inline]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.desc.width = width;
        self.desc.height = height;
        self
    }

    #[inline]
    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.desc.format = Some(format);
        self
    }

    #[inline]
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
