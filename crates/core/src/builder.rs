use crate::app::WindowConfig;
use crate::backend::run;

pub(crate) type InitCb<S> = Box<dyn FnOnce() -> S>;
pub(crate) type UpdateCb<S> = Box<dyn FnMut(&mut S)>;
pub(crate) type CleanupCb<S> = Box<dyn FnOnce(&mut S)>;

pub struct AppBuilder<S>
where
    S: 'static,
{
    pub(crate) window: WindowConfig,
    pub(crate) init_cb: InitCb<S>,
    pub(crate) update_cb: UpdateCb<S>,
    pub(crate) cleanup_cb: CleanupCb<S>,
    // events, cleanup, maybe on? once?
}

pub(crate) fn builder<F, S>(cb: F) -> AppBuilder<S>
where
    F: FnOnce() -> S + 'static,
    S: 'static,
{
    AppBuilder {
        window: WindowConfig::default(),
        init_cb: Box::new(cb),
        update_cb: Box::new(|_| ()),
        cleanup_cb: Box::new(|_| ()),
    }
}

impl<S> AppBuilder<S>
where
    S: 'static,
{
    pub fn with_window(mut self, config: WindowConfig) -> Self {
        self.window = config;
        self
    }

    pub fn on_update<F>(mut self, cb: F) -> Self
    where
        F: FnMut(&mut S) + 'static,
    {
        self.update_cb = Box::new(cb);
        self
    }

    pub fn on_cleanup<F>(mut self, cb: F) -> Self
    where
        F: FnOnce(&mut S) + 'static,
    {
        self.cleanup_cb = Box::new(cb);
        self
    }

    pub fn run(self) -> Result<(), String> {
        run(self)
    }
}
