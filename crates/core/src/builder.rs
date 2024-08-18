use crate::backend::{get_backend, run};

pub(crate) type InitCb<S> = Box<dyn FnOnce() -> Result<S, String>>;
pub(crate) type UpdateCb<S> = Box<dyn FnMut(&mut S)>;

pub struct AppBuilder<S>
where
    S: 'static,
{
    pub(crate) init_cb: InitCb<S>,
    pub(crate) update_cb: UpdateCb<S>,
    // events, cleanup, maybe on? once?
}

pub(crate) fn builder<F, S>(cb: F) -> AppBuilder<S>
where
    F: FnOnce() -> Result<S, String> + 'static,
    S: 'static,
{
    AppBuilder {
        init_cb: Box::new(cb),
        update_cb: Box::new(|_| ()),
    }
}

impl<S> AppBuilder<S>
where
    S: 'static,
{
    pub fn update<F>(mut self, cb: F) -> Self
    where
        F: FnMut(&mut S) + 'static,
    {
        self.update_cb = Box::new(cb);
        self
    }

    pub fn run(self) -> Result<(), String> {
        let AppBuilder { init_cb, update_cb } = self;
        let state = init_cb()?;
        run(state, update_cb)?;
        Ok(())
    }
}
