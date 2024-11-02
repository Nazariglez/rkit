use atomic_refcell::AtomicRefCell;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::sync::Arc;

const MAX_EVENT_LISTENER_HINT: usize = 5;

// - CoreEvents
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum CoreEvent {
    Init,
    PreUpdate,
    PostUpdate,
    CleanUp,
}

pub(crate) static CORE_EVENTS_MAP: Lazy<AtomicRefCell<CoreEventsMap>> =
    Lazy::new(|| AtomicRefCell::new(CoreEventsMap::default()));

#[derive(Default)]
pub(crate) struct CoreEventsMap {
    inner: FxHashMap<
        CoreEvent,
        SmallVec<Arc<dyn Fn() + Send + Sync + 'static>, MAX_EVENT_LISTENER_HINT>,
    >,
}

impl CoreEventsMap {
    pub fn insert<F: Fn() + Send + Sync + 'static>(&mut self, evt: CoreEvent, cb: F) {
        self.inner.entry(evt).or_default().push(Arc::new(cb));
    }

    pub fn trigger(&self, evt: CoreEvent) {
        let Some(listeners) = self.inner.get(&evt) else {
            return;
        };

        listeners.iter().for_each(|listener| listener());
    }
}
