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

    pub fn update<F, P>(mut self, mut cb: F) -> Self
    where
        F: Handler<S, P> + 'static,
    {
        self.update_cb = Box::new(move |s| cb.call(s));
        self
    }

    pub fn cleanup<F, P>(mut self, mut cb: F) -> Self
    where
        F: Handler<S, P> + 'static,
    {
        self.cleanup_cb = Box::new(move |s| cb.call(s));
        self
    }

    pub fn run(self) -> Result<(), String> {
        run(self)
    }
}

pub trait Handler<S, Params> {
    fn call(&mut self, state: &mut S);
}

impl<S, Fun> Handler<S, ()> for Fun
where
    S: 'static,
    Fun: FnMut(),
{
    fn call(&mut self, _state: &mut S) {
        (*self)();
    }
}

impl<S, Fun> Handler<S, (S,)> for Fun
where
    S: 'static,
    Fun: FnMut(&mut S),
{
    fn call(&mut self, state: &mut S) {
        (*self)(state);
    }
}
