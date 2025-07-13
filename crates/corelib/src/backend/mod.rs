#[cfg(feature = "gamepad")]
mod gamepad_gilrs;
mod traits;
#[cfg(target_arch = "wasm32")]
mod web;
mod wgpu;
#[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
mod winit;

#[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
mod headless;
mod limiter;

pub(crate) use traits::{BackendImpl, GfxBackendImpl};

#[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
pub(crate) use winit::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
pub(crate) use headless::*;

#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

pub mod gfx {
    pub use super::wgpu::*;
}
