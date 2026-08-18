mod traits;
#[cfg(target_arch = "wasm32")]
mod web;
mod wgpu;

#[cfg(native_windowed)]
mod winit;

#[cfg(native_windowed)]
mod limiter;

#[cfg(native_headless)]
mod headless;

pub(crate) use traits::{BackendImpl, GfxBackendImpl};

#[cfg(native_windowed)]
pub(crate) use winit::*;

#[cfg(native_headless)]
pub(crate) use headless::*;

#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

pub mod gfx {
    pub use super::wgpu::*;
}
