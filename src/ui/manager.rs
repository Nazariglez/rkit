use corelib::input::{
    is_mouse_moving, is_mouse_scrolling, mouse_btns_down, mouse_btns_pressed, mouse_btns_released,
    mouse_motion_delta, mouse_position, mouse_wheel_delta, KeyCode, MouseButton,
};
use corelib::math::{vec2, vec3, Mat3, Mat4, Vec2};
use draw::{BaseCam2D, Draw2D, Transform2D};
use rustc_hash::{FxHashMap, FxHashSet};
use scene_graph::{NodeIndex, SceneGraph};
use smallvec::SmallVec;
use std::any::TypeId;
use std::marker::PhantomData;

use crate::ui::element::{UIElement, UIRoot};
use crate::ui::events::{
    EventHandlerFn, EventHandlerFnOnce, EventListener, ListenerType, UIEventQueue,
};
use crate::ui::graph::{UIGraph, UIHandler, UINode};
use crate::ui::{UIInput, UINodeMetadata};

pub struct UIEventData<'a, T, S: 'static> {
    pub node: &'a mut T,
    pub state: &'a mut S,
    pub events: &'a mut UIEventQueue<S>,
}

pub struct UIManager<S: 'static> {
    graph: UIGraph<S>,
    events: UIEventQueue<S>,
    listener_id: usize,
    node_id: u64,

    // used to avoid allocations when `once` events expire
    to_remove_cb: Vec<usize>,

    // matrix
    projection: Mat4,
    inverse_projection: Mat4,
    root_matrix: Mat3,
    inverse_root_matrix: Mat3,
    size: Vec2,

    // input
    enable_input: bool,
    screen_mouse_pos: Vec2,
    last_frame_hover: FxHashSet<UIRawHandler>,
    hover: FxHashSet<UIRawHandler>,
    last_frame_down: FxHashSet<(UIRawHandler, MouseButton)>,
    down: FxHashSet<(UIRawHandler, MouseButton)>,
    pressed: FxHashSet<(UIRawHandler, MouseButton)>,
    released: FxHashSet<(UIRawHandler, MouseButton)>,
    start_click: FxHashMap<(UIRawHandler, MouseButton), Vec2>,
    clicked: FxHashSet<(UIRawHandler, MouseButton)>,
    scrolling: FxHashMap<UIRawHandler, Vec2>,
    dragging: FxHashSet<(UIRawHandler, MouseButton)>,
}

impl<S> Default for UIManager<S> {
    fn default() -> Self {
        Self::new(true)
    }
}
impl<S> UIManager<S> {
    pub fn new(enable_input: bool) -> Self {
        let node = UINode {
            raw_id: 0,
            inner: Box::new(UIRoot {
                transform: Transform2D::default(),
            }),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY.inverse(),
            handlers: FxHashMap::default(),
        };

        Self {
            graph: UIGraph::new(node),
            events: UIEventQueue::new(),
            listener_id: 0,
            node_id: 0,

            to_remove_cb: vec![],
            projection: Default::default(),
            inverse_projection: Default::default(),
            root_matrix: Default::default(),
            inverse_root_matrix: Default::default(),
            size: Vec2::ZERO,

            enable_input,
            screen_mouse_pos: Default::default(),
            last_frame_hover: Default::default(),
            hover: Default::default(),
            last_frame_down: Default::default(),
            down: Default::default(),
            pressed: Default::default(),
            released: Default::default(),
            start_click: Default::default(),
            clicked: Default::default(),
            scrolling: Default::default(),
            dragging: Default::default(),
        }
    }

    #[inline(always)]
    pub fn add<T: UIElement<S> + 'static>(&mut self, element: T) -> UIHandler<T> {
        self.graph.add(element)
    }

    #[inline(always)]
    pub fn add_to<H: Into<UIRawHandler>, T: UIElement<S> + 'static>(
        &mut self,
        parent: H,
        element: T,
    ) -> Result<UIHandler<T>, String> {
        self.graph.add_to(parent, element)
    }

    #[inline(always)]
    pub fn element<T>(&self, handler: UIHandler<T>) -> Option<&T>
    where
        T: UIElement<S> + 'static,
    {
        self.graph.element(handler)
    }

    #[inline(always)]
    pub fn element_mut<T>(&mut self, handler: UIHandler<T>) -> Option<&mut T>
    where
        T: UIElement<S> + 'static,
    {
        self.graph.element_mut(handler)
    }

    #[inline(always)]
    pub fn remove<T: UIElement<S> + 'static>(
        &mut self,
        handler: UIHandler<T>,
    ) -> Result<(), String> {
        self.graph.remove(handler)
    }

    fn set_camera(&mut self, cam: &dyn BaseCam2D) {
        self.size = cam.size();
        self.graph
            .root_transform_mut()
            .set_size(self.size)
            .set_translation(cam.bounds().min());
        self.projection = cam.projection();
        self.inverse_projection = cam.inverse_projection();
        self.root_matrix = cam.transform();
        self.inverse_root_matrix = cam.inverse_transform();

        if self.enable_input {
            self.screen_mouse_pos = mouse_position();
        }
    }

    fn dispatch_events(&mut self, state: &mut S) {
        while let Some(event_cb) = self.events.take_event() {
            event_cb(
                &mut self.graph.scene_graph, // TODO: check this
                &mut self.events,
                state,
                &mut self.to_remove_cb,
            );
        }
    }

    fn process_inputs(&mut self, state: &mut S) {
        if !self.enable_input {
            return;
        }

        self.scrolling.clear();
        self.clicked.clear();
        std::mem::swap(&mut self.last_frame_hover, &mut self.hover);
        self.hover.clear();
        std::mem::swap(&mut self.last_frame_down, &mut self.down);
        self.down.clear();

        // reverse the graph
        let mut graph: SmallVec<(&mut UINode<S>, &mut UINode<S>), 120> =
            self.graph.scene_graph.iter_mut().collect();
        graph.reverse();

        let down_btns = mouse_btns_down();
        let pressed_btns = mouse_btns_pressed();
        let released_btns = mouse_btns_released();
        let scroll = is_mouse_scrolling().then_some(mouse_wheel_delta());
        let moving = is_mouse_moving().then_some(mouse_motion_delta());
        for (parent, node) in graph {
            let parent_point =
                parent.screen_to_local(self.screen_mouse_pos, self.size, self.inverse_projection);
            let point =
                node.screen_to_local(self.screen_mouse_pos, self.size, self.inverse_projection);
            let contains = node.inner.input_box().contains(point);

            let raw = UIRawHandler {
                raw_id: node.raw_id,
                idx: None,
            };

            let metadata = UINodeMetadata {
                handler: raw,
                parent_handler: UIRawHandler {
                    raw_id: parent.raw_id,
                    idx: None,
                },
            };

            if contains {
                if !self.last_frame_hover.contains(&raw) {
                    node.inner
                        .input(UIInput::CursorEnter, state, &mut self.events, metadata);
                }

                self.hover.insert(raw);
                node.inner.input(
                    UIInput::Hover { pos: point },
                    state,
                    &mut self.events,
                    metadata,
                );

                down_btns.iter().for_each(|btn| {
                    self.down.insert((raw, btn));
                });

                pressed_btns.iter().for_each(|btn| {
                    let id = (raw, btn);
                    self.pressed.insert(id);
                    self.start_click.insert(id, parent_point);
                    node.inner.input(
                        UIInput::ButtonPressed(btn),
                        state,
                        &mut self.events,
                        metadata,
                    );
                });

                released_btns.iter().for_each(|btn| {
                    let id = (raw, btn);
                    self.released.insert(id);
                    node.inner.input(
                        UIInput::ButtonReleased(btn),
                        state,
                        &mut self.events,
                        metadata,
                    );

                    if self.start_click.contains_key(&id) {
                        self.clicked.insert(id);
                        node.inner.input(
                            UIInput::ButtonClick(btn),
                            state,
                            &mut self.events,
                            metadata,
                        );
                    }
                });

                if let Some(delta) = scroll {
                    self.scrolling.insert(raw, delta);
                    node.inner
                        .input(UIInput::Scroll { delta }, state, &mut self.events, metadata);
                }
            } else {
                if self.last_frame_hover.contains(&raw) {
                    node.inner
                        .input(UIInput::CursorLeave, state, &mut self.events, metadata);
                }
            }

            if let Some(drag_delta) = moving {
                self.start_click
                    .iter()
                    .filter(|((ui_raw, btn), pos)| *ui_raw == raw)
                    .for_each(|((raw, btn), pos)| {
                        let id = (*raw, *btn);
                        if !self.dragging.contains(&id) {
                            self.dragging.insert(id);
                            node.inner.input(
                                UIInput::DragStart {
                                    pos: *pos,
                                    btn: *btn,
                                },
                                state,
                                &mut self.events,
                                metadata,
                            );
                        }

                        node.inner.input(
                            UIInput::Dragging {
                                start_pos: *pos,
                                pos: parent_point,
                                frame_delta: drag_delta,
                                btn: *btn,
                            },
                            state,
                            &mut self.events,
                            metadata,
                        );
                    })
            }

            released_btns.iter().for_each(|btn| {
                node.inner.input(
                    UIInput::ButtonReleasedAnywhere(btn),
                    state,
                    &mut self.events,
                    metadata,
                );

                let id = (raw, btn);
                if self.dragging.contains(&id) {
                    node.inner.input(
                        UIInput::DragEnd {
                            pos: parent_point,
                            btn,
                        },
                        state,
                        &mut self.events,
                        metadata,
                    );

                    let _ = self.dragging.remove(&id);
                }
            });
        }

        // clean any node that was pressed with this button
        released_btns.iter().for_each(|btn| {
            self.start_click.retain(|(_, b), _| btn != *b);
            self.dragging.retain(|(_, b)| btn != *b);
        });
    }

    pub fn push_event<E>(&mut self, evt: E)
    where
        E: Send + Sync + 'static,
    {
        self.events.push(evt);
    }

    pub fn push_event_to<H, E>(&mut self, handler: H, evt: E)
    where
        H: Into<UIRawHandler> + 'static,
        E: Send + Sync + 'static,
    {
        self.events.push_to(handler, evt);
    }

    pub fn update(&mut self, cam: &dyn BaseCam2D, state: &mut S) {
        self.process_inputs(state);
        self.dispatch_events(state);

        // update matrices
        self.set_camera(cam);

        // update root element matrix
        self.graph.root_update_matrix();

        // update and calculate matrices for the scene-graph
        self.graph
            .scene_graph
            .iter_mut()
            .for_each(|(parent, node)| {
                let metadata = UINodeMetadata {
                    handler: UIRawHandler {
                        raw_id: node.raw_id,
                        idx: None,
                    },
                    parent_handler: UIRawHandler {
                        raw_id: parent.raw_id,
                        idx: None,
                    },
                };
                node.inner.update(state, &mut self.events, metadata);
                let matrix = parent.matrix * node.inner.transform_mut().updated_mat3();
                if matrix != node.matrix {
                    node.matrix = matrix;
                    node.root_inverse_matrix = (self.root_matrix * matrix).inverse();
                }
            });
    }

    pub fn render(&mut self, draw: &mut Draw2D, state: &mut S) {
        // draw.push_matrix(self.scene_graph.root.matrix);
        // self.scene_graph.root.inner.render(draw, state);
        // draw.pop_matrix();

        self.graph
            .scene_graph
            .iter_mut()
            .for_each(|(parent, node)| {
                let metadata = UINodeMetadata {
                    handler: UIRawHandler {
                        raw_id: node.raw_id,
                        idx: None,
                    },
                    parent_handler: UIRawHandler {
                        raw_id: parent.raw_id,
                        idx: None,
                    },
                };
                draw.push_matrix(node.matrix);
                node.inner.render(draw, state, metadata);
                draw.pop_matrix();
            });
    }

    pub fn on<T, E, F>(
        &mut self,
        handler: UIHandler<T>,
        mut cb: F,
    ) -> Result<UIListenerId<T, E>, String>
    where
        T: UIElement<S> + 'static,
        E: 'static,
        F: FnMut(&E, UIEventData<T, S>) + 'static,
    {
        let handler_idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        let handlers = &mut self
            .graph
            .scene_graph
            .get_mut(handler_idx)
            .ok_or_else(|| "Invalid UIHandler".to_string())?
            .value
            .handlers;
        let k = TypeId::of::<E>();
        let cb: Box<EventHandlerFn<E, S>> = Box::new(move |ui_e, evt, state, events| {
            let node = ui_e.downcast_mut::<T>().unwrap();
            let data = UIEventData {
                node,
                state,
                events,
            };
            cb(evt, data);
        });

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Mut(Box::new(cb)),
        };
        handlers.entry(k).or_default().push(listener);

        Ok(UIListenerId {
            id: self.listener_id,
            handler,
            _e: PhantomData,
        })
    }

    pub fn once<T, E, F>(
        &mut self,
        handler: UIHandler<T>,
        cb: F,
    ) -> Result<UIListenerId<T, E>, String>
    where
        T: UIElement<S> + 'static,
        E: 'static,
        F: FnOnce(&mut T, &E, &mut S, &mut UIEventQueue<S>) + 'static,
    {
        let handler_idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        let handlers = &mut self
            .graph
            .scene_graph
            .get_mut(handler_idx)
            .ok_or_else(|| "Invalid UIHandler".to_string())?
            .value
            .handlers;
        let k = TypeId::of::<E>();
        let cb: Box<EventHandlerFnOnce<E, S>> = Box::new(move |t, e, s, q| {
            let tt = t.downcast_mut::<T>().unwrap();
            cb(tt, e, s, q);
        });

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Once(Some(Box::new(cb))),
        };
        handlers.entry(k).or_default().push(listener);

        Ok(UIListenerId {
            id: self.listener_id,
            handler,
            _e: PhantomData,
        })
    }

    pub fn off<T, E>(&mut self, listener_id: UIListenerId<T, E>)
    where
        T: UIElement<S>,
        E: 'static,
        S: 'static,
    {
        // TODO: return Result?
        let idx = listener_id.handler.raw.idx;
        if let Some(idx) = idx {
            if let Some(node) = self.graph.scene_graph.get_mut(idx) {
                if let Some(listeners) = node.value.handlers.get_mut(&TypeId::of::<E>()) {
                    listeners.retain(|listener| listener.id != listener_id.id);
                }
            }
        }
    }

    pub fn trigger_event<T, E>(
        &mut self,
        handler: UIHandler<T>,
        evt: E,
        state: &mut S,
    ) -> Result<(), String>
    where
        T: UIElement<S> + 'static,
        E: Send + Sync + 'static,
    {
        let handler_idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;
        let node = &mut self
            .graph
            .scene_graph
            .get_mut(handler_idx)
            .ok_or_else(|| "Invalid UIHandler".to_string())?
            .value;
        call_event(node, &evt, state, &mut self.events, &mut self.to_remove_cb);
        Ok(())
    }

    pub fn cursor_hover<H>(&mut self, handler: H) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        let idx = match handler.into().idx {
            Some(idx) => idx,
            None => return false,
        };
        let node = match self.graph.scene_graph.get(idx) {
            Some(node) => node,
            None => return false,
        };

        let raw = UIRawHandler {
            raw_id: node.value.raw_id,
            idx: None,
        };

        self.hover.contains(&raw)
    }

    pub fn pressed_by<H>(&mut self, handler: H, btn: MouseButton) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        let idx = match handler.into().idx {
            Some(idx) => idx,
            None => return false,
        };
        let node = match self.graph.scene_graph.get(idx) {
            Some(node) => node,
            None => return false,
        };

        let raw = UIRawHandler {
            raw_id: node.value.raw_id,
            idx: None,
        };

        let id = (raw, btn);
        self.pressed.contains(&id)
    }

    pub fn pressed<H>(&mut self, handler: H) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        self.pressed_by(handler, MouseButton::Left)
    }

    pub fn released_by<H>(&mut self, handler: H, btn: MouseButton) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        let idx = match handler.into().idx {
            Some(idx) => idx,
            None => return false,
        };
        let node = match self.graph.scene_graph.get(idx) {
            Some(node) => node,
            None => return false,
        };

        let raw = UIRawHandler {
            raw_id: node.value.raw_id,
            idx: None,
        };

        let id = (raw, btn);
        self.released.contains(&id)
    }

    pub fn released<H>(&mut self, handler: H) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        self.released_by(handler, MouseButton::Left)
    }

    pub fn down_by<H>(&mut self, handler: H, btn: MouseButton) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        let idx = match handler.into().idx {
            Some(idx) => idx,
            None => return false,
        };
        let node = match self.graph.scene_graph.get(idx) {
            Some(node) => node,
            None => return false,
        };

        let raw = UIRawHandler {
            raw_id: node.value.raw_id,
            idx: None,
        };

        let id = (raw, btn);
        self.down.contains(&id)
    }

    pub fn down<H>(&mut self, handler: H) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        self.down_by(handler, MouseButton::Left)
    }

    pub fn clicked_by<H>(&mut self, handler: H, btn: MouseButton) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        let idx = match handler.into().idx {
            Some(idx) => idx,
            None => return false,
        };
        let node = match self.graph.scene_graph.get(idx) {
            Some(node) => node,
            None => return false,
        };

        let raw = UIRawHandler {
            raw_id: node.value.raw_id,
            idx: None,
        };

        let id = (raw, btn);
        self.clicked.contains(&id)
    }

    pub fn clicked<H>(&mut self, handler: H) -> bool
    where
        H: Into<UIRawHandler> + 'static,
    {
        self.clicked_by(handler, MouseButton::Left)
    }
}

fn call_event<S, E>(
    node: &mut UINode<S>,
    evt: &E,
    state: &mut S,
    queue: &mut UIEventQueue<S>,
    to_remove_cb: &mut Vec<usize>,
) where
    E: 'static,
    S: 'static,
{
    let k = TypeId::of::<E>();
    if let Some(listeners) = node.handlers.get_mut(&k) {
        for listener in listeners.iter_mut() {
            match &mut listener.typ {
                ListenerType::Once(opt_cb) => {
                    if let Some(cb) = opt_cb.take() {
                        let cb = cb.downcast::<Box<EventHandlerFnOnce<E, S>>>().unwrap();
                        cb(node.inner.as_mut(), evt, state, queue);
                    }
                    to_remove_cb.push(listener.id);
                }
                ListenerType::Mut(cb) => {
                    let cb = cb.downcast_mut::<Box<EventHandlerFn<E, S>>>();
                    if let Some(cb) = cb {
                        cb(node.inner.as_mut(), evt, state, queue);
                    }
                }
            }
        }

        listeners.retain(|listener| !to_remove_cb.contains(&listener.id));

        to_remove_cb.clear();
    }
}

pub(super) fn iter_call_event<E, S>(
    evt: E,
    scene_graph: &mut SceneGraph<UINode<S>>,
    queue: &mut UIEventQueue<S>,
    state: &mut S,
    to_remove_cb: &mut Vec<usize>,
    raw_id: Option<u64>,
) where
    E: Send + Sync + 'static,
    S: 'static,
{
    for (_parent, child) in scene_graph.iter_mut() {
        let do_break_after = match raw_id {
            Some(id) => {
                if id == child.raw_id {
                    // do not keep iterating, break after process this event
                    true
                } else {
                    // skip this node
                    continue;
                }
            }
            None => false,
        };
        // if the event is marked as consumed, then do not execute the rest of callbacks
        if queue.current_event_consumed {
            break;
        }

        call_event(child, &evt, state, queue, to_remove_cb);

        if do_break_after {
            break;
        }
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Hash, Debug)]
pub struct UIRawHandler {
    pub(super) raw_id: u64,
    pub(super) idx: Option<NodeIndex>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct UIListenerId<T, E> {
    id: usize,
    handler: UIHandler<T>,
    _e: PhantomData<E>,
}

impl<T, E> Copy for UIListenerId<T, E> {}

impl<T, E> Clone for UIListenerId<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> From<UIHandler<T>> for UIRawHandler {
    fn from(value: UIHandler<T>) -> Self {
        value.raw
    }
}
