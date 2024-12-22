use corelib::input::{
    is_mouse_moving, is_mouse_scrolling, mouse_btns_down, mouse_btns_pressed, mouse_btns_released,
    mouse_motion_delta, mouse_position, mouse_wheel_delta, MouseButton,
};
use corelib::math::{Mat3, Mat4, Rect, Vec2};
use draw::{BaseCam2D, Draw2D, Transform2D};
use heapless::{FnvIndexMap, FnvIndexSet};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use scene_graph::NodeIndex;
use smallvec::SmallVec;
use std::any::TypeId;
use std::marker::PhantomData;
use strum::EnumCount;
use strum_macros::{EnumCount, EnumIter, FromRepr};
use utils::helpers::next_pot2;

use crate::ui::element::{UIElement, UIRoot};
use crate::ui::events::{EventListener, ListenerType};
use crate::ui::graph::{UIGraph, UIHandler, UINode};
use crate::ui::{UIEvents, UIInput, UINodeMetadata};

/// Defines the actions to take after an input event is triggered.
///
/// The variants of this enum are sorted by priority, with higher values indicating more restrictive actions.
/// The priority order ensures that more restrictive actions override less restrictive ones during event processing.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum UIControl {
    /// Continue normal event propagation.
    ///
    /// - All siblings, parents, and ancestors will process the event unless explicitly stopped by other logic.
    #[default]
    Continue = 0,

    /// Skip processing of sibling elements.
    ///
    /// - Sibling nodes of the current element in the same parent container will not process the event.
    SkipSiblings = 1,

    /// Skip processing for overlapping nodes.
    ///
    /// - Prevents events from being processed by other nodes that overlap spatially with the current element.
    SkipOverlap = 2,

    /// Fully consume the event.
    ///
    /// - The event will not propagate further in any phase.
    Consume = 3,
}

impl From<()> for UIControl {
    fn from(_value: ()) -> Self {
        Self::default()
    }
}

pub(super) type ListenerStorage =
    FxHashMap<TypeId, FxHashMap<UIRawHandler, SmallVec<EventListener, 5>>>;
pub(super) type EventHandlerFn<E, S> =
    dyn FnMut(&E, &UIRawHandler, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> UIControl;
pub(super) type EventHandlerFnOnce<E, S> =
    dyn FnOnce(&E, &UIRawHandler, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> UIControl;
pub(super) type EventGlobalHandlerFn<E, S> =
    dyn FnMut(&E, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> UIControl;
pub(super) type EventGlobalHandlerFnOnce<E, S> =
    dyn FnOnce(&E, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> UIControl;

pub struct UIManager<S: 'static> {
    graph: UIGraph<S>,
    events: UIEvents<S>,
    listener_id: usize,

    listeners: ListenerStorage,
    temp_iter_handlers: Vec<NodeIterInfo>,

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
            idx: Some(NodeIndex::Root),
            first_relayout: false,
            inner: Box::new(UIRoot {
                transform: Transform2D::default(),
            }),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY,
            is_visible: true,
            handlers: FxHashMap::default(),
        };

        Self {
            graph: UIGraph::new(node),
            events: UIEvents::default(),
            listener_id: 0,

            listeners: FxHashMap::with_capacity_and_hasher(10, FxBuildHasher),
            temp_iter_handlers: vec![],

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
    pub fn element_as<T>(&self, handler: UIHandler<T>) -> Option<&T>
    where
        T: UIElement<S> + 'static,
    {
        self.graph.element_as(handler)
    }

    #[inline(always)]
    pub fn element_mut_as<T>(&mut self, handler: UIHandler<T>) -> Option<&mut T>
    where
        T: UIElement<S> + 'static,
    {
        self.graph.element_mut_as(handler)
    }

    #[inline(always)]
    pub fn element<H>(&self, handler: H) -> Option<&dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
    {
        self.graph.element(handler)
    }

    #[inline(always)]
    pub fn element_mut<H>(&mut self, handler: H) -> Option<&mut dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
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
            .set_size(cam.bounds().size)
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
        // clean removed in last frame
        self.graph.removed.iter().for_each(|raw| {
            self.listeners.iter_mut().for_each(|(_, listeners)| {
                let _ = listeners.remove(raw);
            });
        });
        self.graph.removed.clear();

        if self.events.is_empty() {
            return;
        }

        // iter through events in reverse order
        self.temp_iter_handlers.clear();
        self.temp_iter_handlers
            .extend(
                self.graph
                    .scene_graph
                    .iter()
                    .map(|(parent, node)| NodeIterInfo {
                        parent_handler: Some(UIRawHandler { idx: parent.idx }),
                        node_handler: UIRawHandler { idx: node.idx },
                    }),
            );
        self.temp_iter_handlers.reverse();

        while let Some(evt_cb) = self.events.take_event() {
            evt_cb(
                &self.temp_iter_handlers,
                &mut self.listeners,
                &mut self.graph,
                &mut self.events,
                state,
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

        let mut control = ProcessControl::default();

        let down_btns = mouse_btns_down();
        let pressed_btns = mouse_btns_pressed();
        let released_btns = mouse_btns_released();
        let scroll = is_mouse_scrolling().then_some(mouse_wheel_delta());
        let moving = is_mouse_moving().then_some(mouse_motion_delta());
        for (parent, node) in graph {
            let can_process = node.is_visible && node.inner.input_enabled();
            if !can_process {
                continue;
            }

            let parent_point =
                parent.screen_to_local(self.screen_mouse_pos, self.size, self.inverse_projection);
            let point =
                node.screen_to_local(self.screen_mouse_pos, self.size, self.inverse_projection);
            let contains = node.inner.input_box().contains(point);

            let raw = UIRawHandler { idx: node.idx };
            let parent_raw = UIRawHandler { idx: parent.idx };

            let metadata = UINodeMetadata {
                handler: raw,
                parent_handler: UIRawHandler { idx: parent.idx },
            };

            if contains {
                if !self.last_frame_hover.contains(&raw) {
                    if control.can_trigger(ControlEvents::Enter, parent_raw, contains) {
                        let crtl = node.inner.input(
                            UIInput::CursorEnter,
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::Enter, crtl, parent_raw);
                    }
                }

                if control.can_trigger(ControlEvents::Hover, parent_raw, contains) {
                    self.hover.insert(raw);
                    let crtl = node.inner.input(
                        UIInput::Hover { pos: point },
                        state,
                        &mut self.events,
                        metadata,
                    );
                    control.store_control(ControlEvents::Hover, crtl, parent_raw);
                }

                down_btns.iter().for_each(|btn| {
                    if control.can_trigger(ControlEvents::Down(btn), parent_raw, contains) {
                        self.down.insert((raw, btn));
                        let crtl = node.inner.input(
                            UIInput::ButtonDown(btn),
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::Down(btn), crtl, parent_raw);
                    }
                });

                pressed_btns.iter().for_each(|btn| {
                    if control.can_trigger(ControlEvents::Pressed(btn), parent_raw, contains) {
                        let id = (raw, btn);
                        self.pressed.insert(id);
                        self.start_click.insert(id, parent_point);
                        let crtl = node.inner.input(
                            UIInput::ButtonPressed(btn),
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::Pressed(btn), crtl, parent_raw);
                    }
                });

                released_btns.iter().for_each(|btn| {
                    let id = (raw, btn);
                    if control.can_trigger(ControlEvents::Released(btn), parent_raw, contains) {
                        self.released.insert(id);
                        let crtl = node.inner.input(
                            UIInput::ButtonReleased(btn),
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::Released(btn), crtl, parent_raw);
                    }

                    if control.can_trigger(ControlEvents::Click(btn), parent_raw, contains) {
                        if self.start_click.contains_key(&id) {
                            self.clicked.insert(id);
                            let crtl = node.inner.input(
                                UIInput::ButtonClick(btn),
                                state,
                                &mut self.events,
                                metadata,
                            );
                            control.store_control(ControlEvents::Click(btn), crtl, parent_raw);
                        }
                    }
                });

                if let Some(delta) = scroll {
                    if control.can_trigger(ControlEvents::Scroll, parent_raw, contains) {
                        self.scrolling.insert(raw, delta);
                        let crtl = node.inner.input(
                            UIInput::Scroll { delta },
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::Scroll, crtl, parent_raw);
                    }
                }
            } else if self.last_frame_hover.contains(&raw) {
                if control.can_trigger(ControlEvents::Leave, parent_raw, contains) {
                    let crtl =
                        node.inner
                            .input(UIInput::CursorLeave, state, &mut self.events, metadata);
                    control.store_control(ControlEvents::Leave, crtl, parent_raw);
                }
            }

            if let Some(drag_delta) = moving {
                self.start_click
                    .iter()
                    .filter(|((ui_raw, _btn), _pos)| *ui_raw == raw)
                    .for_each(|(&(raw, btn), pos)| {
                        let id = (raw, btn);
                        if !self.dragging.contains(&id) {
                            if control.can_trigger(
                                ControlEvents::DragStart(btn),
                                parent_raw,
                                contains,
                            ) {
                                self.dragging.insert(id);
                                let crtl = node.inner.input(
                                    UIInput::DragStart { pos: *pos, btn },
                                    state,
                                    &mut self.events,
                                    metadata,
                                );
                                control.store_control(
                                    ControlEvents::DragStart(btn),
                                    crtl,
                                    parent_raw,
                                );
                            }
                        }

                        if control.can_trigger(ControlEvents::Dragging(btn), parent_raw, contains) {
                            let crtl = node.inner.input(
                                UIInput::Dragging {
                                    start_pos: *pos,
                                    pos: parent_point,
                                    frame_delta: drag_delta,
                                    btn,
                                },
                                state,
                                &mut self.events,
                                metadata,
                            );
                            control.store_control(ControlEvents::Dragging(btn), crtl, parent_raw);
                        }
                    });
            }

            released_btns.iter().for_each(|btn| {
                if control.can_trigger(ControlEvents::ReleasedAnywhere(btn), parent_raw, contains) {
                    let crtl = node.inner.input(
                        UIInput::ButtonReleasedAnywhere(btn),
                        state,
                        &mut self.events,
                        metadata,
                    );
                    control.store_control(ControlEvents::ReleasedAnywhere(btn), crtl, parent_raw);
                }

                let id = (raw, btn);
                if self.dragging.contains(&id) {
                    if control.can_trigger(ControlEvents::DragEnd(btn), parent_raw, contains) {
                        let crtl = node.inner.input(
                            UIInput::DragEnd {
                                pos: parent_point,
                                btn,
                            },
                            state,
                            &mut self.events,
                            metadata,
                        );
                        control.store_control(ControlEvents::DragEnd(btn), crtl, parent_raw);

                        let _ = self.dragging.remove(&id);
                    }
                }
            });
        }

        // TODO: consume events on input callback
        // TODO: global events to be consumed outside of the ui system?

        // clean any node that was pressed with this button
        released_btns.iter().for_each(|btn| {
            self.start_click.retain(|(_, b), _| btn != *b);
            self.dragging.retain(|(_, b)| btn != *b);
        });
    }

    pub fn send_event<E>(&mut self, evt: E)
    where
        E: Send + Sync + 'static,
    {
        self.events.send(evt);
    }

    pub fn send_event_to<H, E>(&mut self, handler: H, evt: E)
    where
        H: Into<UIRawHandler> + 'static,
        E: Send + Sync + 'static,
    {
        self.events.send_to(handler, evt);
    }

    pub fn update(&mut self, cam: &dyn BaseCam2D, state: &mut S) {
        // TODO: this could be done in the update iteration if we move process inputs to the end of the frame?
        // process visible state
        self.graph
            .scene_graph
            .iter_mut()
            .for_each(|(parent, child)| process_visibility(parent, child));

        // process inputs and dispatch events
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
            .filter(|(parent, child)| child.is_visible)
            .for_each(|(parent, node)| {
                let metadata = UINodeMetadata {
                    handler: UIRawHandler { idx: node.idx },
                    parent_handler: UIRawHandler { idx: parent.idx },
                };
                node.inner.update(state, &mut self.events, metadata);
                let matrix = parent.matrix * node.inner.transform_mut().updated_mat3();
                if matrix != node.matrix || !node.first_relayout {
                    node.first_relayout = true;
                    node.matrix = matrix;
                    node.root_inverse_matrix = (self.root_matrix * matrix).inverse();
                    node.inner.relayout(
                        state,
                        &mut self.events,
                        Rect::new(Vec2::ZERO, parent.inner.transform().size()),
                    );
                }
            });
    }

    pub fn render(&mut self, draw: &mut Draw2D, state: &mut S) {
        self.graph
            .scene_graph
            .iter_mut()
            .filter(|(parent, child)| child.is_visible)
            .for_each(|(parent, node)| {
                let metadata = UINodeMetadata {
                    handler: UIRawHandler { idx: node.idx },
                    parent_handler: UIRawHandler { idx: parent.idx },
                };
                draw.push_matrix(node.matrix);
                node.inner.render(draw, state, metadata);
                draw.pop_matrix();
            });
    }

    pub fn listen<E, F, R>(&mut self, mut cb: F) -> Result<UIListenerId<(), E>, String>
    where
        E: 'static,
        R: Into<UIControl> + 'static,
        F: FnMut(&E, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> R + 'static,
    {
        let k = TypeId::of::<E>();
        let handlers = self
            .listeners
            .entry(k)
            .or_insert_with(|| FxHashMap::with_capacity_and_hasher(5, FxBuildHasher));

        let cb: Box<EventGlobalHandlerFn<E, S>> = Box::new(move |e, g, s, q| cb(e, g, s, q).into());

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Mut(Box::new(cb)),
        };

        let raw: UIRawHandler = UIRawHandler { idx: None };
        handlers.entry(raw).or_default().push(listener);

        Ok(UIListenerId {
            id: self.listener_id,
            handler: UIHandler::<()>::default(),
            _e: Default::default(),
        })
    }

    pub fn on<T, E, F, R>(
        &mut self,
        handler: UIHandler<T>,
        mut cb: F,
    ) -> Result<UIListenerId<T, E>, String>
    where
        T: UIElement<S> + 'static,
        E: 'static,
        R: Into<UIControl> + 'static,
        F: FnMut(&E, &UIRawHandler, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> R + 'static,
    {
        let idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        if !self.graph.scene_graph.contains(idx) {
            return Err("Invalid UIHandler".to_string());
        }

        let k = TypeId::of::<E>();
        let handlers = self
            .listeners
            .entry(k)
            .or_insert_with(|| FxHashMap::with_capacity_and_hasher(5, FxBuildHasher));

        let cb: Box<EventHandlerFn<E, S>> = Box::new(move |e, h, g, s, q| cb(e, h, g, s, q).into());

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Mut(Box::new(cb)),
        };

        let raw: UIRawHandler = handler.raw();
        handlers.entry(raw).or_default().push(listener);

        Ok(UIListenerId {
            id: self.listener_id,
            handler,
            _e: PhantomData,
        })
    }

    pub fn listen_once<E, F, R>(&mut self, mut cb: F) -> Result<UIListenerId<(), E>, String>
    where
        E: 'static,
        R: Into<UIControl> + 'static,
        F: FnOnce(&E, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> R + 'static,
    {
        let k = TypeId::of::<E>();
        let handlers = self
            .listeners
            .entry(k)
            .or_insert_with(|| FxHashMap::with_capacity_and_hasher(5, FxBuildHasher));

        let cb: Box<EventGlobalHandlerFnOnce<E, S>> =
            Box::new(move |e, g, s, q| cb(e, g, s, q).into());

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Once(Some(Box::new(cb))),
        };

        let raw: UIRawHandler = UIRawHandler { idx: None };
        handlers.entry(raw).or_default().push(listener);

        Ok(UIListenerId {
            id: self.listener_id,
            handler: UIHandler::<()>::default(),
            _e: Default::default(),
        })
    }

    pub fn once<T, E, F, R>(
        &mut self,
        handler: UIHandler<T>,
        cb: F,
    ) -> Result<UIListenerId<T, E>, String>
    where
        T: UIElement<S> + 'static,
        E: 'static,
        R: Into<UIControl> + 'static,
        F: FnOnce(&E, &UIRawHandler, &mut UIGraph<S>, &mut S, &mut UIEvents<S>) -> R + 'static,
    {
        let idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        if !self.graph.scene_graph.contains(idx) {
            return Err("Invalid UIHandler".to_string());
        }

        let k = TypeId::of::<E>();
        let handlers = self
            .listeners
            .entry(k)
            .or_insert_with(|| FxHashMap::with_capacity_and_hasher(5, FxBuildHasher));

        let cb: Box<EventHandlerFnOnce<E, S>> =
            Box::new(move |e, h, g, s, q| cb(e, h, g, s, q).into());

        self.listener_id += 1;
        let listener = EventListener {
            id: self.listener_id,
            typ: ListenerType::Once(Some(Box::new(cb))),
        };

        let raw: UIRawHandler = handler.raw();
        handlers.entry(raw).or_default().push(listener);

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
        if let Some(idx) = listener_id.handler.raw.idx {
            if let Some(node) = self.graph.scene_graph.get_mut(idx) {
                if let Some(listeners) = node.value.handlers.get_mut(&TypeId::of::<E>()) {
                    listeners.retain(|listener| listener.id != listener_id.id);
                }
            }
        }
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
            idx: node.value.idx,
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
            idx: node.value.idx,
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
            idx: node.value.idx,
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
            idx: node.value.idx,
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
            idx: node.value.idx,
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

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub struct UIRawHandler {
    pub(super) idx: Option<NodeIndex>,
}

impl UIRawHandler {
    pub fn typed<T>(self) -> UIHandler<T> {
        UIHandler::from(self)
    }

    pub fn is_empty(&self) -> bool {
        self.idx.is_none()
    }
}

impl<T> From<UIRawHandler> for UIHandler<T> {
    fn from(value: UIRawHandler) -> Self {
        Self {
            raw: value,
            _t: PhantomData,
        }
    }
}

#[derive(Eq, PartialEq)]
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

pub(super) struct NodeIterInfo {
    pub parent_handler: Option<UIRawHandler>,
    pub node_handler: UIRawHandler,
}

#[inline]
fn process_visibility<S: 'static>(parent: &mut UINode<S>, child: &mut UINode<S>) {
    child.is_visible = parent.is_visible && child.inner.visible();
}

#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, EnumCount)]
enum ControlEvents {
    Hover,
    Enter,
    Leave,
    Click(MouseButton),
    Down(MouseButton),
    Pressed(MouseButton),
    Released(MouseButton),
    ReleasedAnywhere(MouseButton),
    Scroll,
    DragStart(MouseButton),
    Dragging(MouseButton),
    DragEnd(MouseButton),
}

impl ControlEvents {
    pub const fn pot2_count() -> usize {
        let base_count = Self::COUNT;
        let mouse_count = MouseButton::COUNT;
        let variants_using_mouse = 8;
        let count = mouse_count * variants_using_mouse + base_count;
        next_pot2(count)
    }
}

#[derive(Default)]
struct ProcessControl {
    consume: SmallVec<ControlEvents, 10>,
    skip_siblings: SmallVec<(ControlEvents, UIRawHandler), 50>,
    skip_overlap: FnvIndexSet<ControlEvents, { ControlEvents::pot2_count() }>,
}

impl ProcessControl {
    pub fn store_control(&mut self, evt: ControlEvents, control: UIControl, parent: UIRawHandler) {
        match control {
            UIControl::Continue => {}
            UIControl::SkipSiblings => self.skip_siblings(evt, parent),
            UIControl::SkipOverlap => self.skip_overlap(evt),
            UIControl::Consume => self.consume(evt),
        }
    }
    pub fn consume(&mut self, evt: ControlEvents) {
        self.consume.push(evt);
        if matches!(evt, ControlEvents::Hover) {
            self.skip_overlap.insert(ControlEvents::Enter);
            self.skip_overlap.insert(ControlEvents::Leave);
        }
    }

    pub fn skip_siblings(&mut self, evt: ControlEvents, parent: UIRawHandler) {
        self.skip_siblings.push((evt, parent));
    }

    pub fn skip_overlap(&mut self, evt: ControlEvents) {
        let res = self.skip_overlap.insert(evt);
        debug_assert!(res.is_ok());
        match res {
            Ok(_) => {
                if matches!(evt, ControlEvents::Hover) {
                    self.skip_overlap.insert(ControlEvents::Enter);
                    self.skip_overlap.insert(ControlEvents::Leave);
                }
            }
            Err(e) => {
                log::error!("Cannot set {e:?} into ProcessControl::skip_overlap");
            }
        }
    }

    pub fn can_trigger(
        &mut self,
        evt: ControlEvents,
        parent: UIRawHandler,
        contains: bool,
    ) -> bool {
        let is_consumed = self.consume.contains(&evt);
        if is_consumed {
            return false;
        }

        let is_overlapping = contains && self.skip_overlap.contains(&evt);
        if is_overlapping {
            return false;
        }

        let skip_sibling = !self.skip_siblings.contains(&(evt, parent));
        skip_sibling
    }
}
