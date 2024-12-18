use crate::ui::graph::UIGraph;
use crate::ui::manager::{
    EventHandlerFn, EventHandlerFnOnce, ListenerStorage, NodeIterInfo, UIRawHandler,
};
use crate::ui::UIControl;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::any::Any;
use std::any::TypeId;
use std::collections::VecDeque;

pub(super) struct EventListener {
    pub id: usize,
    pub typ: ListenerType,
}

pub(super) enum ListenerType {
    Once(Option<Box<dyn Any>>),
    Mut(Box<dyn Any>),
}

type EvtCb<S> =
    dyn FnOnce(&[NodeIterInfo], &mut ListenerStorage, &mut UIGraph<S>, &mut UIEvents<S>, &mut S);

pub struct UIEvents<S: 'static> {
    pub(super) events: VecDeque<Box<EvtCb<S>>>,
}

impl<S: 'static> Default for UIEvents<S> {
    fn default() -> Self {
        Self {
            events: VecDeque::default(),
        }
    }
}

impl<S: 'static> UIEvents<S> {
    /// Send a new event to the queue
    pub fn send<E: Send + Sync + 'static>(&mut self, evt: E) {
        self.events
            .push_front(Box::new(move |nodes, storage, graph, queue, state| {
                let k = TypeId::of::<E>();
                let Some(listeners) = storage.get_mut(&k) else {
                    return;
                };

                let mut skip_siblings = None;
                for info in nodes {
                    // only skip siblings when parent is Some
                    let skip =
                        skip_siblings.is_some_and(|skip_idx| info.parent_handler == Some(skip_idx));
                    if skip {
                        continue;
                    }

                    skip_siblings = None;

                    // execute event callback
                    let control =
                        dispatch(&evt, &info.node_handler, listeners, graph, state, queue);
                    match control {
                        UIControl::Continue => {}
                        UIControl::SkipSiblings => {
                            skip_siblings = info.parent_handler;
                        }
                        UIControl::Consume => break,
                    }
                }
            }));
    }

    /// Send a new event to a unique node
    pub fn send_to<H: Into<UIRawHandler> + 'static, E: Send + Sync + 'static>(
        &mut self,
        handler: H,
        evt: E,
    ) {
        self.events
            .push_front(Box::new(move |_nodes, storage, graph, queue, state| {
                let k = TypeId::of::<E>();
                let Some(listeners) = storage.get_mut(&k) else {
                    return;
                };

                let raw = handler.into();
                let _c = dispatch(&evt, &raw, listeners, graph, state, queue);
            }));
    }

    /// Send a new event to the start of th queue
    pub fn push<E: Send + Sync + 'static>(&mut self, evt: E) {
        self.events
            .push_back(Box::new(move |nodes, storage, graph, queue, state| {
                let k = TypeId::of::<E>();
                let Some(listeners) = storage.get_mut(&k) else {
                    return;
                };

                nodes.iter().for_each(|info| {
                    dispatch(&evt, &info.node_handler, listeners, graph, state, queue);
                });
            }));
    }

    /// Send a new event to the start of the queue targeting only a unique node
    pub fn push_to<H: Into<UIRawHandler> + 'static, E: Send + Sync + 'static>(
        &mut self,
        handler: H,
        evt: E,
    ) {
        self.events
            .push_back(Box::new(move |_nodes, storage, graph, queue, state| {
                let k = TypeId::of::<E>();
                let Some(listeners) = storage.get_mut(&k) else {
                    return;
                };

                let raw = handler.into();
                dispatch(&evt, &raw, listeners, graph, state, queue);
            }));
    }

    /// Take the first event of the queue
    pub(super) fn take_event(&mut self) -> Option<Box<EvtCb<S>>> {
        self.events.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

fn dispatch<E, S>(
    evt: &E,
    raw: &UIRawHandler,
    listeners: &mut FxHashMap<UIRawHandler, SmallVec<EventListener, 5>>,
    graph: &mut UIGraph<S>,
    state: &mut S,
    queue: &mut UIEvents<S>,
) -> UIControl
where
    S: 'static,
    E: Send + Sync + 'static,
{
    let Some(cbs) = listeners.get_mut(raw) else {
        return UIControl::Continue;
    };

    let mut control = UIControl::Continue;
    cbs.iter_mut().for_each(|cb| {
        let c = match &mut cb.typ {
            ListenerType::Once(opt_cb) => match opt_cb.take() {
                Some(cb) => {
                    let cb = cb.downcast::<Box<EventHandlerFnOnce<E, S>>>().unwrap();
                    cb(evt, raw, graph, state, queue)
                }
                None => UIControl::Continue,
            },
            ListenerType::Mut(cb) => {
                let cb = cb.downcast_mut::<Box<EventHandlerFn<E, S>>>().unwrap();
                cb(evt, raw, graph, state, queue)
            }
        };

        if c > control {
            control = c;
        }
    });

    cbs.retain(|listener| !matches!(listener.typ, ListenerType::Once(None)));
    control
}
