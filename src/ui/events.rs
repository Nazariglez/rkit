use crate::ui::element::UIElement;
use crate::ui::graph::{UIGraph, UINode};
use crate::ui::manager::{iter_call_event, ListenerStorage, NodeIterInfo, UIRawHandler};
use scene_graph::SceneGraph;
use std::any::Any;
use std::any::TypeId;
use std::collections::VecDeque;

pub(super) type EventHandlerFn<E, S> =
    dyn FnMut(&mut dyn UIElement<S>, &E, &mut S, &mut UIEventQueue<S>);

pub(super) type EventHandlerFnOnce<E, S> =
    dyn FnOnce(&mut dyn UIElement<S>, &E, &mut S, &mut UIEventQueue<S>);

pub(super) struct EventListener {
    pub id: usize,
    pub typ: ListenerType,
}

pub(super) enum ListenerType {
    Once(Option<Box<dyn Any>>),
    Mut(Box<dyn Any>),
}

pub struct UIEvents<S: 'static> {
    pub(super) events: VecDeque<
        Box<
            dyn FnOnce(
                &[NodeIterInfo],
                &mut ListenerStorage,
                &mut UIGraph<S>,
                &mut UIEvents<S>,
                &mut S,
            ),
        >,
    >,
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
        self.events.push_front(Box::new(
            move |nodes, storage, manager, queue, state| {
                println!("inside event");
                let k = TypeId::of::<E>();
                let Some(listeners) = storage.get_mut(&k) else {
                    return;
                };

                println!("here1 {:?}", listeners.len());

                nodes.iter().for_each(|info| {
                    println!("looking for {:?}", info.node_handler);
                    let Some(cbs) = listeners.get_mut(&info.node_handler) else {
                        return;
                    };
                    println!("here2");

                    cbs.iter_mut().for_each(|cb| {
                        match &mut cb.typ {
                            ListenerType::Once(opt_cb) => {
                                //TODO: set *to_remove = true;
                                if let Some(cb) = opt_cb.take() {
                                    let cb =
                                        cb.downcast::<Box<EventHandlerFnOnce<E, S>>>().unwrap();
                                    // cb(node.inner.as_mut(), evt, state, queue);
                                }
                                // TODO: clean once listeners or clean None
                            }
                            ListenerType::Mut(cb) => {
                                // TODO
                                println!("here3!");
                            }
                        }
                    });
                });
                // iter_call_event(evt, scene_graph, queue, state, to_remove, placeholder);
            },
        ));
    }

    /// Send a new event to a unique node
    pub fn send_to<H: Into<UIRawHandler> + 'static, E: Send + Sync + 'static>(
        &mut self,
        handler: H,
        evt: E,
    ) {
        // self.events
        //     .push_front(Box::new(move |scene_graph, queue, state, to_remove| {
        //         iter_call_event(
        //             evt,
        //             scene_graph,
        //             queue,
        //             state,
        //             to_remove,
        //             Some(handler.into().raw_id),
        //         );
        //     }));
    }

    /// Take the first event of the queue
    pub(super) fn take_event(
        &mut self,
    ) -> Option<
        Box<
            dyn FnOnce(
                &[NodeIterInfo],
                &mut ListenerStorage,
                &mut UIGraph<S>,
                &mut UIEvents<S>,
                &mut S,
            ),
        >,
    > {
        self.events.pop_front()
    }
}

#[derive(Default)]
pub struct UIEventQueue<S: 'static> {
    pub(super) events: VecDeque<
        Box<dyn FnOnce(&mut SceneGraph<UINode<S>>, &mut UIEventQueue<S>, &mut S, &mut Vec<usize>)>,
    >,
    pub(super) current_event_consumed: bool,
}

impl<S: 'static> UIEventQueue<S> {
    pub(super) fn new() -> Self {
        Self {
            events: VecDeque::new(),
            current_event_consumed: false,
        }
    }

    /// Push events in the order that should be executed
    pub(super) fn push<E: Send + Sync + 'static>(&mut self, evt: E) {
        self.events
            .push_back(Box::new(move |scene_graph, queue, state, to_remove| {
                let placeholder: Option<u64> = None;
                iter_call_event(evt, scene_graph, queue, state, to_remove, placeholder);
            }));
    }

    /// Push events in the order that should be executed
    pub(super) fn push_to<H: Into<UIRawHandler> + 'static, E: Send + Sync + 'static>(
        &mut self,
        handler: H,
        evt: E,
    ) {
        self.events
            .push_back(Box::new(move |scene_graph, queue, state, to_remove| {
                iter_call_event(
                    evt,
                    scene_graph,
                    queue,
                    state,
                    to_remove,
                    Some(handler.into().raw_id),
                );
            }));
    }

    /// Take the first event of the queue
    pub(super) fn take_event(
        &mut self,
    ) -> Option<
        Box<dyn FnOnce(&mut SceneGraph<UINode<S>>, &mut UIEventQueue<S>, &mut S, &mut Vec<usize>)>,
    > {
        self.current_event_consumed = false;
        self.events.pop_front()
    }

    /// Mark the current event as consumed, so we stop the callbacks
    pub fn consume_event(&mut self) {
        self.current_event_consumed = true;
    }

    /// Send a new event to the queue
    pub fn send<E: Send + Sync + 'static>(&mut self, evt: E) {
        self.events
            .push_front(Box::new(move |scene_graph, queue, state, to_remove| {
                let placeholder: Option<u64> = None;
                iter_call_event(evt, scene_graph, queue, state, to_remove, placeholder);
            }));
    }

    /// Send a new event to a unique node
    pub fn send_to<H: Into<UIRawHandler> + 'static, E: Send + Sync + 'static>(
        &mut self,
        handler: H,
        evt: E,
    ) {
        self.events
            .push_front(Box::new(move |scene_graph, queue, state, to_remove| {
                iter_call_event(
                    evt,
                    scene_graph,
                    queue,
                    state,
                    to_remove,
                    Some(handler.into().raw_id),
                );
            }));
    }
}
