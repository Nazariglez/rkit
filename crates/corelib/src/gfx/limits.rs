use arrayvec::ArrayVec;
use strum::EnumCount;

use crate::gfx::TextureFormat;

/// Limit are overridden by the graphic implementation
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_texture_size_2d: u32,
    pub max_texture_size_3d: u32,
    pub surface_formats: ArrayVec<TextureFormat, { TextureFormat::COUNT }>,
}
