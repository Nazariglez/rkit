use crate::gfx::{SamplerId, TextureFilter, TextureFormat, TextureId, TextureWrap};
use crate::math::Vec2;
use arrayvec::ArrayVec;
use std::fmt::{Debug, Formatter};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use wgpu::{
    Sampler as RawSampler, Texture as RawTexture, TextureFormat as WTextureFormat, TextureView,
};

#[derive(Clone)]
pub struct Texture {
    pub(crate) id: TextureId,
    pub(crate) raw: Arc<RawTexture>,
    pub(crate) view: Arc<TextureView>,
    pub(crate) size: Vec2,
    pub(crate) write: bool,
    pub(crate) storage: bool,
    pub(crate) copyable: bool,
    pub(crate) format: TextureFormat,
    pub(crate) mip_level_count: u32,
    pub(crate) revision: Arc<AtomicU32>,
}

impl PartialEq<Self> for Texture {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Texture {
    #[inline]
    pub fn id(&self) -> TextureId {
        self.id
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.size.x
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.y
    }

    #[inline]
    pub fn is_writable(&self) -> bool {
        self.write
    }

    #[inline]
    pub(crate) fn is_storage(&self) -> bool {
        self.storage
    }

    #[inline]
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    #[inline]
    pub fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }

    #[inline]
    pub fn revision(&self) -> u32 {
        self.revision.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn mark_written(&self) {
        self.revision.fetch_add(1, Ordering::Relaxed);
    }
}

impl Debug for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("write", &self.write)
            .field("storage", &self.storage)
            .field("format", &self.format)
            .field("mip_level_count", &self.mip_level_count)
            .finish()
    }
}

impl TextureFormat {
    pub(crate) fn from_naga_storage(format: wgpu::naga::StorageFormat) -> Option<Self> {
        use wgpu::naga::StorageFormat;

        Some(match format {
            StorageFormat::R8Unorm => Self::R8UNorm,
            StorageFormat::R8Snorm => Self::R8INorm,
            StorageFormat::R8Uint => Self::R8UInt,
            StorageFormat::R8Sint => Self::R8Int,
            StorageFormat::R16Unorm => Self::R16UNorm,
            StorageFormat::R16Snorm => Self::R16INorm,
            StorageFormat::R16Uint => Self::R16UInt,
            StorageFormat::R16Sint => Self::R16Int,
            StorageFormat::R16Float => Self::R16Float,
            StorageFormat::Rg8Unorm => Self::Rg8UNorm,
            StorageFormat::Rg8Snorm => Self::Rg8INorm,
            StorageFormat::Rg8Uint => Self::Rg8UInt,
            StorageFormat::Rg8Sint => Self::Rg8Int,
            StorageFormat::Rg16Unorm => Self::Rg16UNorm,
            StorageFormat::Rg16Snorm => Self::Rg16INorm,
            StorageFormat::Rg16Uint => Self::Rg16UInt,
            StorageFormat::Rg16Sint => Self::Rg16Int,
            StorageFormat::Rg16Float => Self::Rg16Float,
            StorageFormat::R32Uint => Self::R32UInt,
            StorageFormat::R32Sint => Self::R32Int,
            StorageFormat::R32Float => Self::R32Float,
            StorageFormat::Rg32Uint => Self::Rg32UInt,
            StorageFormat::Rg32Sint => Self::Rg32Int,
            StorageFormat::Rg32Float => Self::Rg32Float,
            StorageFormat::Rgba8Unorm => Self::Rgba8UNorm,
            StorageFormat::Rgba8Snorm => Self::Rgba8INorm,
            StorageFormat::Rgba8Uint => Self::Rgba8UInt,
            StorageFormat::Rgba8Sint => Self::Rgba8Int,
            StorageFormat::Bgra8Unorm => Self::Bgra8UNorm,
            StorageFormat::Rgba16Unorm => Self::Rgba16UNorm,
            StorageFormat::Rgba16Snorm => Self::Rgba16INorm,
            StorageFormat::Rgba16Uint => Self::Rgba16UInt,
            StorageFormat::Rgba16Sint => Self::Rgba16Int,
            StorageFormat::Rgba16Float => Self::Rgba16Float,
            StorageFormat::Rgba32Uint => Self::Rgba32UInt,
            StorageFormat::Rgba32Sint => Self::Rgba32Int,
            StorageFormat::Rgba32Float => Self::Rgba32Float,
            StorageFormat::R64Uint
            | StorageFormat::Rgb10a2Uint
            | StorageFormat::Rgb10a2Unorm
            | StorageFormat::Rg11b10Ufloat => return None,
        })
    }

    pub(crate) fn view_formats(&self) -> ArrayVec<WTextureFormat, 1> {
        let mut v = ArrayVec::default();
        match self {
            TextureFormat::Rgba8UNorm => v.push(WTextureFormat::Rgba8UnormSrgb),
            TextureFormat::Rgba8UNormSrgb => v.push(WTextureFormat::Rgba8Unorm),
            TextureFormat::Bgra8UNorm => v.push(WTextureFormat::Bgra8UnormSrgb),
            TextureFormat::Bgra8UNormSrgb => v.push(WTextureFormat::Bgra8Unorm),
            _ => {}
        }
        v
    }

    pub(crate) fn as_wgpu(&self) -> WTextureFormat {
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
            TextureFormat::Bgra8UNormSrgb => WTextureFormat::Bgra8UnormSrgb,

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

    pub(crate) fn from_wgpu(format: WTextureFormat) -> Option<TextureFormat> {
        match format {
            WTextureFormat::R8Unorm => Some(TextureFormat::R8UNorm),
            WTextureFormat::R8Snorm => Some(TextureFormat::R8INorm),
            WTextureFormat::R8Uint => Some(TextureFormat::R8UInt),
            WTextureFormat::R8Sint => Some(TextureFormat::R8Int),

            WTextureFormat::Rg8Unorm => Some(TextureFormat::Rg8UNorm),
            WTextureFormat::Rg8Snorm => Some(TextureFormat::Rg8INorm),
            WTextureFormat::Rg8Uint => Some(TextureFormat::Rg8UInt),
            WTextureFormat::Rg8Sint => Some(TextureFormat::Rg8Int),

            WTextureFormat::Rgba8Unorm => Some(TextureFormat::Rgba8UNorm),
            WTextureFormat::Rgba8UnormSrgb => Some(TextureFormat::Rgba8UNormSrgb),
            WTextureFormat::Rgba8Snorm => Some(TextureFormat::Rgba8INorm),
            WTextureFormat::Rgba8Uint => Some(TextureFormat::Rgba8UInt),
            WTextureFormat::Rgba8Sint => Some(TextureFormat::Rgba8Int),
            WTextureFormat::Bgra8Unorm => Some(TextureFormat::Bgra8UNorm),
            WTextureFormat::Bgra8UnormSrgb => Some(TextureFormat::Bgra8UNormSrgb),

            WTextureFormat::R16Unorm => Some(TextureFormat::R16UNorm),
            WTextureFormat::R16Snorm => Some(TextureFormat::R16INorm),
            WTextureFormat::R16Uint => Some(TextureFormat::R16UInt),
            WTextureFormat::R16Sint => Some(TextureFormat::R16Int),
            WTextureFormat::R16Float => Some(TextureFormat::R16Float),

            WTextureFormat::Rg16Unorm => Some(TextureFormat::Rg16UNorm),
            WTextureFormat::Rg16Snorm => Some(TextureFormat::Rg16INorm),
            WTextureFormat::Rg16Uint => Some(TextureFormat::Rg16UInt),
            WTextureFormat::Rg16Sint => Some(TextureFormat::Rg16Int),
            WTextureFormat::Rg16Float => Some(TextureFormat::Rg16Float),

            WTextureFormat::Rgba16Unorm => Some(TextureFormat::Rgba16UNorm),
            WTextureFormat::Rgba16Snorm => Some(TextureFormat::Rgba16INorm),
            WTextureFormat::Rgba16Uint => Some(TextureFormat::Rgba16UInt),
            WTextureFormat::Rgba16Sint => Some(TextureFormat::Rgba16Int),
            WTextureFormat::Rgba16Float => Some(TextureFormat::Rgba16Float),

            WTextureFormat::R32Uint => Some(TextureFormat::R32UInt),
            WTextureFormat::R32Sint => Some(TextureFormat::R32Int),
            WTextureFormat::R32Float => Some(TextureFormat::R32Float),

            WTextureFormat::Rg32Uint => Some(TextureFormat::Rg32UInt),
            WTextureFormat::Rg32Sint => Some(TextureFormat::Rg32Int),
            WTextureFormat::Rg32Float => Some(TextureFormat::Rg32Float),

            WTextureFormat::Rgba32Uint => Some(TextureFormat::Rgba32UInt),
            WTextureFormat::Rgba32Sint => Some(TextureFormat::Rgba32Int),
            WTextureFormat::Rgba32Float => Some(TextureFormat::Rgba32Float),

            WTextureFormat::Depth16Unorm => Some(TextureFormat::Depth16),
            WTextureFormat::Depth24Plus => Some(TextureFormat::Depth24),
            WTextureFormat::Depth32Float => Some(TextureFormat::Depth32Float),
            WTextureFormat::Depth24PlusStencil8 => Some(TextureFormat::Depth24Stencil8),
            WTextureFormat::Depth32FloatStencil8 => Some(TextureFormat::Depth32FloatStencil8),

            _ => None,
        }
    }
}

impl TextureWrap {
    pub(crate) fn as_wgpu(&self) -> wgpu::AddressMode {
        match self {
            TextureWrap::Clamp => wgpu::AddressMode::ClampToEdge,
            TextureWrap::Repeat => wgpu::AddressMode::Repeat,
            TextureWrap::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

impl TextureFilter {
    pub(crate) fn as_wgpu(&self) -> wgpu::FilterMode {
        match self {
            TextureFilter::Linear => wgpu::FilterMode::Linear,
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
        }
    }

    pub(crate) fn as_wgpu_mipmap(&self) -> wgpu::MipmapFilterMode {
        match self {
            TextureFilter::Linear => wgpu::MipmapFilterMode::Linear,
            TextureFilter::Nearest => wgpu::MipmapFilterMode::Nearest,
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
    pub(crate) mipmap_filter: TextureFilter,
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

    pub fn mipmap_filter(&self) -> TextureFilter {
        self.mipmap_filter
    }

    pub(crate) fn is_filtering(&self) -> bool {
        matches!(
            (self.min_filter, self.mag_filter, self.mipmap_filter),
            (TextureFilter::Linear, _, _)
                | (_, TextureFilter::Linear, _)
                | (_, _, TextureFilter::Linear)
        )
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
            .field("mipmap_filter", &self.mipmap_filter)
            .finish()
    }
}
