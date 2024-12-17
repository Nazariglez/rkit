use crate::ui::events::EventListener;
use crate::ui::{UIElement, UIRawHandler};
use corelib::math::{vec2, vec3, Mat3, Mat4, Vec2};
use draw::Transform2D;
use rustc_hash::FxHashMap;
use scene_graph::SceneGraph;
use smallvec::SmallVec;
use std::any::TypeId;
use std::marker::PhantomData;

pub struct UIGraph<S: 'static> {
    pub(super) scene_graph: SceneGraph<UINode<S>>,
    node_id: u64,
}

impl<S> UIGraph<S>
where
    S: 'static,
{
    pub fn new(root: UINode<S>) -> Self {
        Self {
            scene_graph: SceneGraph::new(root),
            node_id: 0,
        }
    }

    pub(super) fn root_transform_mut(&mut self) -> &mut Transform2D {
        self.scene_graph.root.inner.transform_mut()
    }

    pub(super) fn root_update_matrix(&mut self) {
        self.scene_graph.root.matrix = self.scene_graph.root.inner.transform_mut().updated_mat3();
    }

    pub fn add<T: UIElement<S> + 'static>(&mut self, element: T) -> UIHandler<T> {
        self.node_id += 1;
        let node = UINode {
            raw_id: self.node_id,
            inner: Box::new(element),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY.inverse(),
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
    ) -> Result<UIHandler<T>, String> {
        let parent_idx = parent
            .into()
            .idx
            .ok_or_else(|| "Empty UIHandler".to_string())?;

        self.node_id += 1;
        let node = UINode {
            raw_id: self.node_id,
            inner: Box::new(element),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY.inverse(),
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
}

pub struct UINode<S> {
    pub(super) raw_id: u64,
    pub(super) inner: Box<dyn UIElement<S>>,
    pub(super) matrix: Mat3,
    pub(super) root_inverse_matrix: Mat3,
    pub(super) handlers: FxHashMap<TypeId, SmallVec<EventListener, 10>>,
}

impl<S> UINode<S> {
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
        self.root_inverse_matrix
            .transform_point2(vec2(pos.x, pos.y))
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct UIHandler<T> {
    pub(super) raw: UIRawHandler,
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
