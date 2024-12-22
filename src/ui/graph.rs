use crate::ui::{UIElement, UIRawHandler};
use corelib::math::{vec2, vec3, Mat3, Mat4, Vec2};
use draw::Transform2D;
use scene_graph::{NodeIndex, SceneGraph};
use std::marker::PhantomData;

pub struct UIGraph<S: 'static> {
    pub(super) scene_graph: SceneGraph<UINode<S>>,
    pub(super) removed: Vec<UIRawHandler>,
}

impl<S> UIGraph<S>
where
    S: 'static,
{
    pub fn new(root: UINode<S>) -> Self {
        Self {
            scene_graph: SceneGraph::new(root),
            removed: vec![],
        }
    }

    pub(super) fn root_transform_mut(&mut self) -> &mut Transform2D {
        self.scene_graph.root.inner.transform_mut()
    }

    pub(super) fn root_update_matrix(&mut self) {
        self.scene_graph.root.matrix = self.scene_graph.root.inner.transform_mut().updated_mat3();
    }

    pub fn add<T: UIElement<S> + 'static>(&mut self, element: T) -> UIHandler<T> {
        let node = UINode {
            idx: None,
            initialized_layout: false,
            inner: Box::new(element),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY,
            is_enabled: true,
            alpha: 1.0,
        };

        let idx = self.scene_graph.attach_at_root(node);
        self.scene_graph.get_mut(idx).unwrap().value.idx = Some(idx);

        UIHandler {
            raw: UIRawHandler { idx: Some(idx) },
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

        let node = UINode {
            idx: None,
            initialized_layout: false,
            inner: Box::new(element),
            matrix: Mat3::IDENTITY,
            root_inverse_matrix: Mat3::IDENTITY,
            is_enabled: true,
            alpha: 1.0,
        };
        let idx = self
            .scene_graph
            .attach(parent_idx, node)
            .map_err(|e| e.to_string())?;

        self.scene_graph.get_mut(idx).unwrap().value.idx = Some(idx);
        Ok(UIHandler {
            raw: UIRawHandler { idx: Some(idx) },
            _t: PhantomData,
        })
    }

    pub fn element<H>(&self, handler: H) -> Option<&dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
    {
        let idx = handler.into().idx?;
        self.scene_graph
            .get(idx)
            .map(|node| node.value.inner.as_ref())
    }

    pub fn element_mut<H>(&mut self, handler: H) -> Option<&mut dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
    {
        let idx = handler.into().idx?;
        self.scene_graph
            .get_mut(idx)
            .map(|node| node.value.inner.as_mut())
    }

    pub fn parent<H>(&self, handler: H) -> Option<&dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
    {
        let idx = handler.into().idx?;
        let parent_idx = self.scene_graph.parent(idx)?;
        self.scene_graph
            .get(parent_idx)
            .map(|node| node.value.inner.as_ref())
    }

    pub fn parent_mut<H>(&mut self, handler: H) -> Option<&mut dyn UIElement<S>>
    where
        H: Into<UIRawHandler>,
    {
        let idx = handler.into().idx?;
        let parent_idx = self.scene_graph.parent(idx)?;
        self.scene_graph
            .get_mut(parent_idx)
            .map(|node| node.value.inner.as_mut())
    }

    pub fn element_as<T>(&self, handler: UIHandler<T>) -> Option<&T>
    where
        T: UIElement<S> + 'static,
    {
        let idx = handler.raw.idx?;
        self.scene_graph
            .get(idx)
            .map(|node| node.value.inner.downcast_ref::<T>().unwrap())
    }

    pub fn element_mut_as<T>(&mut self, handler: UIHandler<T>) -> Option<&mut T>
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
        self.removed.push(handler.raw);
        Ok(())
    }
}

pub struct UINode<S> {
    pub(super) idx: Option<NodeIndex>,
    pub(super) initialized_layout: bool,
    pub(super) inner: Box<dyn UIElement<S>>,
    pub(super) matrix: Mat3,
    pub(super) root_inverse_matrix: Mat3,
    pub(super) is_enabled: bool,
    pub(super) alpha: f32,
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

#[derive(Eq, PartialEq)]
pub struct UIHandler<T> {
    pub(super) raw: UIRawHandler,
    pub(super) _t: PhantomData<T>,
}

impl<T> Default for UIHandler<T> {
    fn default() -> Self {
        Self {
            raw: UIRawHandler { idx: None },
            _t: PhantomData,
        }
    }
}

impl<T> Copy for UIHandler<T> {}

impl<T> Clone for UIHandler<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> UIHandler<T> {
    pub fn raw(self) -> UIRawHandler {
        self.raw
    }
}
