use atomic_refcell::AtomicRefCell;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::any::{Any, TypeId};

// static EVENT_MAP: Lazy<AtomicRefCell<EventMap>> = Lazy::new(|| AtomicRefCell::new(EventMap::new()));

const MAX_EVENT_LISTENER_HINT: usize = 8;

pub(crate) struct EventListener {
    pub id: usize,
    pub typ: ListenerType,
}

pub(crate) enum ListenerType {
    Once(Option<Box<dyn Any>>),
    Mut(Box<dyn Any>),
}

impl EventListener {
    pub(crate) fn is_once(&self) -> bool {
        matches!(self.typ, ListenerType::Once(_))
    }
}

pub(crate) type EventHandlerFn<E, S> = dyn FnMut(&E, &mut S);
pub(crate) type EventHandlerFnOnce<E, S> = dyn FnOnce(&E, &mut S);

/// Represent a event's handler
/// It allow to use as parameter the App's State
pub trait EventHandler<Evt, S: 'static, T> {
    fn call(&mut self, evt: &Evt, state: &mut S);
}

/// Represent a event's handler
/// It allow to use as parameter the App's State
pub trait EventHandlerOnce<Evt, S: 'static, T> {
    fn call(self, evt: &Evt, state: &mut S);
}

pub(crate) struct EventMap {
    event_ids: usize,
    inner: FxHashMap<TypeId, SmallVec<EventListener, MAX_EVENT_LISTENER_HINT>>,
    to_clean: Vec<usize>,
}

impl EventMap {
    pub fn new() -> Self {
        Self {
            event_ids: 0,
            inner: Default::default(),
            to_clean: Vec::with_capacity(10),
        }
    }

    pub fn on<E, S, T, H>(mut self, mut handler: H) -> Self
    where
        S: 'static,
        E: 'static,
        H: EventHandler<E, S, T> + 'static,
    {
        let k = TypeId::of::<E>();
        let cb: Box<EventHandlerFn<E, S>> = Box::new(move |e: &E, s: &mut S| handler.call(e, s));
        self.inner.entry(k).or_default().push(EventListener {
            id: self.event_ids,
            typ: ListenerType::Mut(Box::new(cb)),
        });
        self.event_ids += 1;
        self
    }

    pub fn once<E, S, T, H>(mut self, handler: H) -> Self
    where
        S: 'static,
        E: 'static,
        H: EventHandlerOnce<E, S, T> + 'static,
    {
        let k = TypeId::of::<E>();
        let cb: Box<EventHandlerFnOnce<E, S>> =
            Box::new(move |e: &E, s: &mut S| handler.call(e, s));
        self.inner.entry(k).or_default().push(EventListener {
            id: self.event_ids,
            typ: ListenerType::Once(Some(Box::new(cb))),
        });
        self.event_ids += 1;
        self
    }

    pub fn emit<E, S>(&mut self, evt: E, state: &mut S)
    where
        E: 'static,
        S: 'static,
    {
        let k = TypeId::of::<E>();
        let Some(listeners) = self.inner.get_mut(&k) else {
            return;
        };

        listeners
            .iter_mut()
            .for_each(|listener| match &mut listener.typ {
                ListenerType::Once(opt_cb) => {
                    if let Some(cb) = opt_cb.take() {
                        let cb = cb.downcast::<Box<EventHandlerFnOnce<E, S>>>().unwrap();
                        cb(&evt, state);
                    }
                    self.to_clean.push(listener.id);
                }
                ListenerType::Mut(cb) => {
                    let cb = cb.downcast_mut::<Box<EventHandlerFn<E, S>>>();
                    if let Some(cb) = cb {
                        cb(&evt, state);
                    }
                }
            });

        listeners.retain(|listener| !self.to_clean.contains(&listener.id));
        self.to_clean.clear();
    }
}
