use crate::gfx::TextureFormat;
use wgpu::{
    Adapter, Device, Instance, PowerPreference, Queue, Surface as RawSurface,
    TextureFormat as WTextureFormat,
};

pub(crate) struct Context {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

fn texture_to_wgpu(t: TextureFormat) -> WTextureFormat {
    match t {
        // Single channel 8-bit textures
        TextureFormat::R8UNorm => WTextureFormat::R8Unorm,
        TextureFormat::R8INorm => WTextureFormat::R8Snorm,
        TextureFormat::R8UInt => WTextureFormat::R8Uint,
        TextureFormat::R8Int => WTextureFormat::R8Sint,

        // Two channel 8-bit textures
        TextureFormat::Rg8UNorm => WTextureFormat::Rg8Unorm,
        TextureFormat::Rg8INorm => WTextureFormat::Rg8Snorm,
        TextureFormat::Rg8UInt => WTextureFormat::Rg8Uint,
        TextureFormat::Rg8Int => WTextureFormat::Rg8Sint,

        // Four channel 8-bit textures
        TextureFormat::Rgba8UNorm => WTextureFormat::Rgba8Unorm,
        TextureFormat::Rgba8UNormSrgb => WTextureFormat::Rgba8UnormSrgb,
        TextureFormat::Rgba8INorm => WTextureFormat::Rgba8Snorm,
        TextureFormat::Rgba8UInt => WTextureFormat::Rgba8Uint,
        TextureFormat::Rgba8Int => WTextureFormat::Rgba8Sint,
        TextureFormat::Bgra8UNorm => WTextureFormat::Bgra8Unorm,

        // Single channel 16-bit textures
        TextureFormat::R16UNorm => WTextureFormat::R16Unorm,
        TextureFormat::R16INorm => WTextureFormat::R16Snorm,
        TextureFormat::R16UInt => WTextureFormat::R16Uint,
        TextureFormat::R16Int => WTextureFormat::R16Sint,
        TextureFormat::R16Float => WTextureFormat::R16Float,

        // Two channel 16-bit textures
        TextureFormat::Rg16UNorm => WTextureFormat::Rg16Unorm,
        TextureFormat::Rg16INorm => WTextureFormat::Rg16Snorm,
        TextureFormat::Rg16UInt => WTextureFormat::Rg16Uint,
        TextureFormat::Rg16Int => WTextureFormat::Rg16Sint,
        TextureFormat::Rg16Float => WTextureFormat::Rg16Float,

        // Four channel 16-bit textures
        TextureFormat::Rgba16UNorm => WTextureFormat::Rgba16Unorm,
        TextureFormat::Rgba16INorm => WTextureFormat::Rgba16Snorm,
        TextureFormat::Rgba16UInt => WTextureFormat::Rgba16Uint,
        TextureFormat::Rgba16Int => WTextureFormat::Rgba16Sint,
        TextureFormat::Rgba16Float => WTextureFormat::Rgba16Float,

        // Single channel 32-bit textures
        TextureFormat::R32UInt => WTextureFormat::R32Uint,
        TextureFormat::R32Int => WTextureFormat::R32Sint,
        TextureFormat::R32Float => WTextureFormat::R32Float,

        // Two channel 32-bit textures
        TextureFormat::Rg32UInt => WTextureFormat::Rg32Uint,
        TextureFormat::Rg32Int => WTextureFormat::Rg32Sint,
        TextureFormat::Rg32Float => WTextureFormat::Rg32Float,

        // Four channel 32-bit textures
        TextureFormat::Rgba32UInt => WTextureFormat::Rgba32Uint,
        TextureFormat::Rgba32Int => WTextureFormat::Rgba32Sint,
        TextureFormat::Rgba32Float => WTextureFormat::Rgba32Float,

        // Depth and stencil formats
        TextureFormat::Depth16 => WTextureFormat::Depth16Unorm,
        TextureFormat::Depth24 => WTextureFormat::Depth24Plus,
        TextureFormat::Depth32Float => WTextureFormat::Depth32Float,
        TextureFormat::Depth24Stencil8 => WTextureFormat::Depth24PlusStencil8,
        TextureFormat::Depth32FloatStencil8 => WTextureFormat::Depth32FloatStencil8,
    }
}
