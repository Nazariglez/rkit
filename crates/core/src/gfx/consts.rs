#![allow(unused)]

use super::TextureFormat;

pub const SURFACE_DEFAULT_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24Stencil8;

pub const MAX_VERTEX_BUFFERS: usize = 8;
pub const MAX_VERTEX_ATTRIBUTES: usize = 16;
pub const MAX_SAMPLERS_PER_SHADER_STAGE: usize = 16;
pub const MAX_SAMPLED_TEXTURES_PER_SHADER_STAGE: usize = 16;
pub const MAX_BIND_GROUPS_PER_PIPELINE: usize = 4;
pub const MAX_PIPELINE_COMPATIBLE_TEXTURES: usize = 10;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
pub const MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE: usize = 11;

#[cfg(not(all(target_arch = "wasm32", feature = "webgl")))]
pub const MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE: usize = 12;
