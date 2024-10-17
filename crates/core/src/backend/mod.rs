#[cfg(feature = "gamepad")]
mod gamepad_gilrs;
mod traits;
#[cfg(target_arch = "wasm32")]
mod web;
mod wgpu;
#[cfg(not(target_arch = "wasm32"))]
mod winit;

pub(crate) use traits::{BackendImpl, GfxBackendImpl};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use winit::*;

#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

pub mod gfx {
    pub use super::wgpu::*;
}
