use corelib::input::{
    mouse_btns_down, mouse_btns_pressed, mouse_btns_released, mouse_position, KeyCode, MouseButton,
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

pub struct UIEventData<'a, T, S: 'static> {
    pub node: &'a mut T,
    pub transform: &'a mut Transform2D,
    pub state: &'a mut S,
    pub events: &'a mut UIEventQueue<S>,
}

pub struct UIManager<S: 'static> {
    scene_graph: SceneGraph<Node<S>>,
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
    start_click: FxHashSet<(UIRawHandler, MouseButton)>,
    clicked: FxHashSet<(UIRawHandler, MouseButton)>,
}

impl<S> Default for UIManager<S> {
    fn default() -> Self {
        Self::new(true)
    }
}
impl<S> UIManager<S> {
    pub fn new(enable_input: bool) -> Self {
        let node = Node {
            raw_id: 0,
            inner: Box::new(UIRoot),
            transform: Default::default(),
            matrix: Mat3::IDENTITY,
            inverse_matrix: Mat3::IDENTITY.inverse(),
            handlers: FxHashMap::default(),
        };

        Self {
            scene_graph: SceneGraph::new(node),
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
        }
    }

    fn set_camera(&mut self, cam: &dyn BaseCam2D) {
        self.size = cam.size();
        self.projection = cam.projection();
        self.inverse_projection = cam.inverse_projection();
        self.root_matrix = cam.transform();
        self.inverse_root_matrix = cam.inverse_transform();

        if self.enable_input {
            self.screen_mouse_pos = mouse_position();
        }
    }

    pub fn add<T: UIElement<S> + 'static>(
        &mut self,
        element: T,
        transform: Transform2D,
    ) -> UIHandler<T> {
        self.node_id += 1;
        let node = Node {
            raw_id: self.node_id,
            inner: Box::new(element),
            transform,
            matrix: Mat3::IDENTITY,
            inverse_matrix: Mat3::IDENTITY.inverse(),
            handlers: Default::default(),
        };

        let idx = self.scene_graph.attach_at_root(node);
        UIHandler {
            raw: UIRawHandler {
                raw_id: self.node_id,
                idx: Some(idx),
            },
            _t: PhantomData,
        }
    }

    pub fn add_to<H: Into<UIRawHandler>, T: UIElement<S> + 'static>(
        &mut self,
        parent: H,
        element: T,
        transform: Transform2D,
    ) -> Result<UIHandler<T>, String> {
        let parent_idx = parent
            .into()
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        self.node_id += 1;
        let node = Node {
            raw_id: self.node_id,
            inner: Box::new(element),
            transform,
            matrix: Mat3::IDENTITY,
            inverse_matrix: Mat3::IDENTITY.inverse(),
            handlers: Default::default(),
        };
        self.scene_graph
            .attach(parent_idx, node)
            .map(|idx| UIHandler {
                raw: UIRawHandler {
                    raw_id: self.node_id,
                    idx: Some(idx),
                },
                _t: PhantomData,
            })
            .map_err(|e| e.to_string())
    }

    fn dispatch_events(&mut self, state: &mut S) {
        while let Some(event_cb) = self.events.take_event() {
            event_cb(
                &mut self.scene_graph,
                &mut self.events,
                state,
                &mut self.to_remove_cb,
            );
        }
    }

    fn process_inputs(&mut self) {
        if !self.enable_input {
            return;
        }

        self.clicked.clear();
        std::mem::swap(&mut self.last_frame_hover, &mut self.hover);
        self.hover.clear();
        std::mem::swap(&mut self.last_frame_down, &mut self.down);
        self.down.clear();

        let down_btns = mouse_btns_down();
        let pressed_btns = mouse_btns_pressed();
        let released_btns = mouse_btns_released();
        for (_parent, node) in &self.scene_graph {
            let local = self.node_screen_to_local(node);
            let contains = self.node_contains_point(local, node);
            if contains {
                let raw = UIRawHandler {
                    raw_id: node.raw_id,
                    idx: None,
                };

                self.hover.insert(raw);
                down_btns.iter().for_each(|btn| {
                    self.down.insert((raw, btn));
                });

                pressed_btns.iter().for_each(|btn| {
                    let id = (raw, btn);
                    self.pressed.insert(id);
                    self.start_click.insert(id);
                });

                released_btns.iter().for_each(|btn| {
                    let id = (raw, btn);
                    self.released.insert(id);

                    if self.start_click.contains(&id) {
                        self.clicked.insert(id);
                    }
                });
            }
        }

        // clean any node that was pressed with this button
        released_btns.iter().for_each(|btn| {
            self.start_click.retain(|(_, b)| btn != *b);
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

    fn node_screen_to_local(&self, node: &Node<S>) -> Vec2 {
        node.screen_to_local(self.screen_mouse_pos, self.size, self.inverse_projection)
    }

    fn node_contains_point(&self, point: Vec2, node: &Node<S>) -> bool {
        let size = node.transform.size();
        point.x >= 0.0 && point.y >= 0.0 && point.x < size.x && point.y < size.y
    }

    pub fn update(&mut self, cam: &dyn BaseCam2D, state: &mut S) {
        self.dispatch_events(state);

        // update matrices
        self.set_camera(cam);

        // we got root.matrix from the camera not from the node
        self.scene_graph.root.transform.update();
        self.scene_graph.root.matrix = self.root_matrix;

        // update and calculate matrices for the scene-graph
        self.scene_graph.iter_mut().for_each(|(parent, node)| {
            node.inner
                .update(&mut node.transform, state, &mut self.events); // TODO: pass parent?
            let matrix = parent.matrix * node.transform.updated_mat3();
            if matrix != node.matrix {
                node.matrix = matrix;
                node.inverse_matrix = matrix.inverse();
            }
        });

        self.process_inputs();
    }

    pub fn render(&mut self, draw: &mut Draw2D, state: &mut S) {
        draw.push_matrix(self.scene_graph.root.transform.as_mat3());
        self.scene_graph
            .root
            .inner
            .render(&self.scene_graph.root.transform, draw, state);
        draw.pop_matrix();

        self.scene_graph.iter_mut().for_each(|(parent, node)| {
            draw.push_matrix(parent.transform.as_mat3() * node.transform.as_mat3());
            node.inner.render(&node.transform, draw, state);
            draw.pop_matrix();
        });
    }
    pub fn get<T>(&self, handler: UIHandler<T>) -> Option<(&T, &Transform2D)>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph.get(idx).map(|node| {
            (
                node.value.inner.downcast_ref::<T>().unwrap(),
                &node.value.transform,
            )
        })
    }

    pub fn get_mut<T>(&mut self, handler: UIHandler<T>) -> Option<(&mut T, &mut Transform2D)>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph.get_mut(idx).map(|node| {
            (
                node.value.inner.downcast_mut::<T>().unwrap(),
                &mut node.value.transform,
            )
        })
    }

    pub fn element<T>(&self, handler: UIHandler<T>) -> Option<&T>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph
            .get(idx)
            .map(|node| node.value.inner.downcast_ref::<T>().unwrap())
    }

    pub fn element_mut<T>(&mut self, handler: UIHandler<T>) -> Option<&mut T>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph
            .get_mut(idx)
            .map(|node| node.value.inner.downcast_mut::<T>().unwrap())
    }

    pub fn transform<T>(&self, handler: UIHandler<T>) -> Option<&Transform2D>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph.get(idx).map(|node| &node.value.transform)
    }

    pub fn transform_mut<T>(&mut self, handler: UIHandler<T>) -> Option<&mut Transform2D>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph
            .get_mut(idx)
            .map(|node| &mut node.value.transform)
    }

    pub fn remove<T: UIElement<S> + 'static>(
        &mut self,
        handler: UIHandler<T>,
    ) -> Result<(), String> {
        let idx = handler
            .raw
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;
        self.scene_graph.remove(idx);
        Ok(())
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
            .scene_graph
            .get_mut(handler_idx)
            .ok_or_else(|| "Invalid UIHandler".to_string())?
            .value
            .handlers;
        let k = TypeId::of::<E>();
        let cb: Box<EventHandlerFn<E, S>> = Box::new(move |ui_e, evt, transform, state, events| {
            let node = ui_e.downcast_mut::<T>().unwrap();
            let data = UIEventData {
                node,
                transform,
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
            if let Some(node) = self.scene_graph.get_mut(idx) {
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
        let node = match self.scene_graph.get(idx) {
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
        let node = match self.scene_graph.get(idx) {
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
        let node = match self.scene_graph.get(idx) {
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
        let node = match self.scene_graph.get(idx) {
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
        let node = match self.scene_graph.get(idx) {
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

pub struct Node<S> {
    raw_id: u64,
    inner: Box<dyn UIElement<S>>,
    transform: Transform2D,
    matrix: Mat3,
    inverse_matrix: Mat3,
    handlers: FxHashMap<TypeId, SmallVec<EventListener, 10>>,
}

impl<S> Node<S> {
    pub fn screen_to_local(
        &self,
        screen_pos: Vec2,
        screen_size: Vec2,
        inverse_projection: Mat4,
    ) -> Vec2 {
        // normalized coordinates
        let norm = screen_pos / screen_size;
        let mouse_pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        // projected position
        let pos = inverse_projection.project_point3(vec3(mouse_pos.x, mouse_pos.y, 1.0));

        // local position
        self.inverse_matrix.transform_point2(vec2(pos.x, pos.y))
    }
}

fn call_event<S, E>(
    node: &mut Node<S>,
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
                        cb(node.inner.as_mut(), evt, &mut node.transform, state, queue);
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
    scene_graph: &mut SceneGraph<Node<S>>,
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

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct UIHandler<T> {
    raw: UIRawHandler,
    _t: PhantomData<T>,
}

impl<T> Copy for UIHandler<T> {}

impl<T> Clone for UIHandler<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for UIHandler<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> UIHandler<T> {
    pub fn is_empty(&self) -> bool {
        self.raw.idx.is_none()
    }

    pub fn empty() -> Self {
        UIHandler {
            raw: Default::default(),
            _t: PhantomData,
        }
    }

    pub(super) fn raw_id(&self) -> u64 {
        self.raw.raw_id
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
