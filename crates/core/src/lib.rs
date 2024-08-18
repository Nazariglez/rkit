pub mod app;
mod backend;
mod builder;

use crate::builder::{builder, AppBuilder};

pub fn init_with<F, S>(callback: F) -> AppBuilder<S>
where
    F: FnOnce() -> Result<S, String> + 'static,
    S: 'static,
{
    builder(callback)
}

pub fn init() -> AppBuilder<()> {
    init_with(|| Ok(()))
}
