use crate::gfx::{SamplerId, TextureFilter, TextureFormat, TextureId, TextureWrap};
use crate::math::UVec2;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use wgpu::{
    Sampler as RawSampler, Texture as RawTexture, TextureFormat as WTextureFormat, TextureView,
};

#[derive(Clone)]
pub struct Texture {
    pub(crate) id: TextureId,
    pub(crate) raw: Arc<RawTexture>,
    pub(crate) view: Arc<TextureView>,
    pub(crate) size: UVec2,
    pub(crate) write: bool,
    pub(crate) format: TextureFormat,
}

impl Texture {
    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn width(&self) -> u32 {
        self.size.x
    }

    pub fn height(&self) -> u32 {
        self.size.y
    }

    pub fn is_writable(&self) -> bool {
        self.write
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

impl Debug for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("write", &self.write)
            .field("format", &self.format)
            .finish()
    }
}

impl TextureFormat {
    pub(crate) fn to_wgpu(&self) -> WTextureFormat {
        match self {
            TextureFormat::R8UNorm => WTextureFormat::R8Unorm,
            TextureFormat::R8INorm => WTextureFormat::R8Snorm,
            TextureFormat::R8UInt => WTextureFormat::R8Uint,
            TextureFormat::R8Int => WTextureFormat::R8Sint,

            TextureFormat::Rg8UNorm => WTextureFormat::Rg8Unorm,
            TextureFormat::Rg8INorm => WTextureFormat::Rg8Snorm,
            TextureFormat::Rg8UInt => WTextureFormat::Rg8Uint,
            TextureFormat::Rg8Int => WTextureFormat::Rg8Sint,

            TextureFormat::Rgba8UNorm => WTextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UNormSrgb => WTextureFormat::Rgba8UnormSrgb,
            TextureFormat::Rgba8INorm => WTextureFormat::Rgba8Snorm,
            TextureFormat::Rgba8UInt => WTextureFormat::Rgba8Uint,
            TextureFormat::Rgba8Int => WTextureFormat::Rgba8Sint,
            TextureFormat::Bgra8UNorm => WTextureFormat::Bgra8Unorm,
            // TextureFormat::Bgra8UNormSrgb => WTextureFormat::Bgra8UnormSrgb,
            TextureFormat::R16UNorm => WTextureFormat::R16Unorm,
            TextureFormat::R16INorm => WTextureFormat::R16Snorm,
            TextureFormat::R16UInt => WTextureFormat::R16Uint,
            TextureFormat::R16Int => WTextureFormat::R16Sint,
            TextureFormat::R16Float => WTextureFormat::R16Float,

            TextureFormat::Rg16UNorm => WTextureFormat::Rg16Unorm,
            TextureFormat::Rg16INorm => WTextureFormat::Rg16Snorm,
            TextureFormat::Rg16UInt => WTextureFormat::Rg16Uint,
            TextureFormat::Rg16Int => WTextureFormat::Rg16Sint,
            TextureFormat::Rg16Float => WTextureFormat::Rg16Float,

            TextureFormat::Rgba16UNorm => WTextureFormat::Rgba16Unorm,
            TextureFormat::Rgba16INorm => WTextureFormat::Rgba16Snorm,
            TextureFormat::Rgba16UInt => WTextureFormat::Rgba16Uint,
            TextureFormat::Rgba16Int => WTextureFormat::Rgba16Sint,
            TextureFormat::Rgba16Float => WTextureFormat::Rgba16Float,

            TextureFormat::R32UInt => WTextureFormat::R32Uint,
            TextureFormat::R32Int => WTextureFormat::R32Sint,
            TextureFormat::R32Float => WTextureFormat::R32Float,

            TextureFormat::Rg32UInt => WTextureFormat::Rg32Uint,
            TextureFormat::Rg32Int => WTextureFormat::Rg32Sint,
            TextureFormat::Rg32Float => WTextureFormat::Rg32Float,

            TextureFormat::Rgba32UInt => WTextureFormat::Rgba32Uint,
            TextureFormat::Rgba32Int => WTextureFormat::Rgba32Sint,
            TextureFormat::Rgba32Float => WTextureFormat::Rgba32Float,

            TextureFormat::Depth16 => WTextureFormat::Depth16Unorm,
            TextureFormat::Depth24 => WTextureFormat::Depth24Plus,
            TextureFormat::Depth32Float => WTextureFormat::Depth32Float,
            TextureFormat::Depth24Stencil8 => WTextureFormat::Depth24PlusStencil8,
            TextureFormat::Depth32FloatStencil8 => WTextureFormat::Depth32FloatStencil8,
        }
    }
}

impl TextureWrap {
    pub(crate) fn to_wgpu(&self) -> wgpu::AddressMode {
        match self {
            TextureWrap::Clamp => wgpu::AddressMode::ClampToEdge,
            TextureWrap::Repeat => wgpu::AddressMode::Repeat,
            TextureWrap::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

impl TextureFilter {
    pub(crate) fn to_wgpu(&self) -> wgpu::FilterMode {
        match self {
            TextureFilter::Linear => wgpu::FilterMode::Linear,
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}

// - Sampler
#[derive(Clone)]
pub struct Sampler {
    pub(crate) id: SamplerId,
    pub(crate) raw: Arc<RawSampler>,
    pub(crate) wrap_x: TextureWrap,
    pub(crate) wrap_y: TextureWrap,
    pub(crate) wrap_z: TextureWrap,
    pub(crate) mag_filter: TextureFilter,
    pub(crate) min_filter: TextureFilter,
    pub(crate) mipmap_filter: Option<TextureFilter>,
}

impl Sampler {
    pub fn id(&self) -> SamplerId {
        self.id
    }

    pub fn wrap_x(&self) -> TextureWrap {
        self.wrap_x
    }

    pub fn wrap_y(&self) -> TextureWrap {
        self.wrap_y
    }

    pub fn wrap_z(&self) -> TextureWrap {
        self.wrap_z
    }

    pub fn mag_filter(&self) -> TextureFilter {
        self.mag_filter
    }

    pub fn min_filter(&self) -> TextureFilter {
        self.min_filter
    }
}

impl Debug for Sampler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sampler")
            .field("id", &self.id)
            .field("wrap_x", &self.wrap_x)
            .field("wrap_y", &self.wrap_y)
            .field("wrap_z", &self.wrap_z)
            .field("mag_filter", &self.mag_filter)
            .field("min_filter", &self.min_filter)
            .finish()
    }
}
