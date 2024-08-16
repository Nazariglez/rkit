type InitCb<S> = Box<dyn FnOnce() -> Result<S, String>>;
type UpdateCb<S> = Box<dyn FnMut(&mut S)>;

pub struct AppBuilder<S>
where
    S: 'static,
{
    pub(crate) init_cb: InitCb<S>,
    pub(crate) update_cb: UpdateCb<S>,
    // events, cleanup, maybe on? once?
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
        let AppBuilder {
            init_cb,
            mut update_cb,
        } = self;
        let mut state = init_cb()?;
        update_cb(&mut state);
        Ok(())
    }
}
