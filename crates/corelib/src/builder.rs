use crate::app::WindowConfig;
use crate::backend::run;

#[cfg(feature = "logs")]
use crate::app::logger::{init_logs, LogConfig};

pub(crate) type InitCb<S> = Box<dyn FnOnce() -> S>;
pub(crate) type UpdateCb<S> = Box<dyn FnMut(&mut S)>;
pub(crate) type CleanupCb<S> = Box<dyn FnOnce(&mut S)>;

struct FixedUpdate<S> {
    delta: f32,
    accumulator: f32,
    cb: UpdateCb<S>,
}

impl<S> FixedUpdate<S> {
    fn tick(&mut self, state: &mut S) {
        let dt = crate::time::delta_f32();
        self.accumulator += dt;

        while self.accumulator >= self.delta {
            (self.cb)(state);
            self.accumulator -= self.delta;
        }
    }
}

pub struct AppBuilder<S>
where
    S: 'static,
{
    pub(crate) window: WindowConfig,
    pub(crate) init_cb: InitCb<S>,
    pub(crate) update_cb: UpdateCb<S>,
    pub(crate) cleanup_cb: CleanupCb<S>,

    fixed_update_cb: Option<Vec<FixedUpdate<S>>>,

    #[cfg(feature = "logs")]
    log_config: LogConfig,
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
        fixed_update_cb: None,
        cleanup_cb: Box::new(|_| ()),

        #[cfg(feature = "logs")]
        log_config: LogConfig::default(),
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

    #[cfg(feature = "logs")]
    pub fn with_logs(mut self, config: LogConfig) -> Self {
        self.log_config = config;
        self
    }

    pub fn update<F, P>(mut self, mut cb: F) -> Self
    where
        F: Handler<S, P> + 'static,
    {
        self.update_cb = Box::new(move |s| cb.call(s));
        self
    }

    pub fn fixed_update<F, P>(mut self, delta: f32, mut cb: F) -> Self
    where
        F: Handler<S, P> + 'static,
    {
        let cb = FixedUpdate {
            delta,
            cb: Box::new(move |s| cb.call(s)),
            accumulator: 0.0,
        };

        let list = self.fixed_update_cb.get_or_insert_with(Vec::new);
        list.push(cb);
        self
    }

    pub fn cleanup<F, P>(mut self, mut cb: F) -> Self
    where
        F: Handler<S, P> + 'static,
    {
        self.cleanup_cb = Box::new(move |s| cb.call(s));
        self
    }

    pub fn run(mut self) -> Result<(), String> {
        #[cfg(feature = "logs")]
        init_logs(self.log_config.clone());

        // Prepare for fixed update
        if let Some(mut fixed) = self.fixed_update_cb.take() {
            let mut update = self.update_cb;
            self.update_cb = Box::new(move |s| {
                fixed.iter_mut().for_each(|cb| {
                    cb.tick(s);
                });
                update(s);
            });
        }

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
