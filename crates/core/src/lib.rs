pub mod app;
mod backend;
mod builder;
pub mod gfx;
pub mod input;
pub mod math;
mod utils;

use crate::builder::{builder, AppBuilder};

pub fn init_with<F, S>(callback: F) -> AppBuilder<S>
where
    F: FnOnce() -> S + 'static,
    S: 'static,
{
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    builder(callback)
}

pub fn init() -> AppBuilder<()> {
    init_with(|| ())
}
