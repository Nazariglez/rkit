use corelib::input::{KeyCode, MouseButton};
use corelib::math::Vec2;
use draw::{BaseCam2D, Camera2D, Draw2D, Transform2D};
use rustc_hash::FxHashMap;
use scene_graph::{NodeIndex, SceneGraph};
use smallvec::SmallVec;
use std::any::TypeId;
use std::marker::PhantomData;

use crate::ui::element::{UIElement, UIRoot};
use crate::ui::events::{
    EventHandlerFn, EventHandlerFnOnce, EventListener, ListenerType, UIEventQueue,
};

pub struct UIManager<S: 'static> {
    scene_graph: SceneGraph<Node<S>>,
    events: UIEventQueue<S>,
    listener_id: usize,
    node_id: u64,

    // used to avoid allocations when `once` events expire
    to_remove_cb: Vec<usize>,
}

impl<S> Default for UIManager<S> {
    fn default() -> Self {
        Self::new()
    }
}
impl<S> UIManager<S> {
    pub fn new() -> Self {
        let mut container = UIRoot;
        let node = Node {
            raw_id: 0,
            inner: Box::new(container),
            transform: Default::default(),
            handlers: FxHashMap::default(),
        };

        Self {
            scene_graph: SceneGraph::new(node),
            events: UIEventQueue::new(),
            listener_id: 0,
            node_id: 0,

            to_remove_cb: vec![],
        }
    }

    pub fn set_camera(&mut self, cam: &dyn BaseCam2D) {
        // TODO: camera or projection???
    }

    // pub fn set_position(&mut self, pos: Vec2) {
    //     self.scene_graph.root.transform.set_translation(pos);
    // }
    //
    // pub fn set_size(&mut self, size: Vec2) {
    //     self.scene_graph.root.transform.set_translation(size);
    // }

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

    pub fn update(&mut self, state: &mut S) {
        self.scene_graph.root.transform.update();
        self.scene_graph.iter_mut().for_each(|(parent, node)| {
            node.inner
                .update(&mut node.transform, state, &mut self.events); // TODO: pass parent?
            node.transform.update();
        });
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
        F: FnMut(&mut T, &E, &mut S, &mut UIEventQueue<S>) + 'static,
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
        let cb: Box<EventHandlerFn<E, S>> = Box::new(move |t, e, s, q| {
            let tt = t.downcast_mut::<T>().unwrap();
            cb(tt, e, s, q);
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
}

pub struct Node<S> {
    raw_id: u64,
    inner: Box<dyn UIElement<S>>,
    transform: Transform2D,
    handlers: FxHashMap<TypeId, SmallVec<EventListener, 10>>,
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

#[derive(Copy, Clone)]
pub enum UIInput {
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
    Char(char),
    MouseBtnPressed(MouseButton),
    MouseBtnReleased(MouseButton),
    MouseMove { x: f32, y: f32 },
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

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Hash)]
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
