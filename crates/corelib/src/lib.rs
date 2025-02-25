pub mod app;
mod backend;
mod builder;
mod events;
pub mod gfx;
pub mod input;
pub mod math;
pub mod time;
mod utils;

use crate::builder::{AppBuilder, builder};

pub fn init_with<F, S>(callback: F) -> AppBuilder<S>
where
    F: FnOnce() -> S + 'static,
    S: 'static,
{
    builder(callback)
}

pub fn init() -> AppBuilder<()> {
    init_with(|| ())
}
