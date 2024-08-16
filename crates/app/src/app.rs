use crate::builder::AppBuilder;

pub fn init_with<F, S>(callback: F) -> AppBuilder<S>
where
    F: FnOnce() -> Result<S, String> + 'static,
    S: 'static,
{
    AppBuilder {
        init_cb: Box::new(callback),
        update_cb: Box::new(|_| {}),
    }
}

pub fn init() -> AppBuilder<()> {
    init_with(|| Ok(()))
}
