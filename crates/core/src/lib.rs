pub mod app;
mod backend;
mod builder;
pub mod input;
mod utils;

use crate::builder::{builder, AppBuilder};

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
