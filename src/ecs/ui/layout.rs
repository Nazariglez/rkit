use crate::{
    draw::{BaseCam2D, Camera2D, Draw2D},
    math::{Mat3, Mat4, Vec2, Vec3Swizzles, vec2, vec3},
};
use bevy_ecs::prelude::*;
use corelib::math::vec4;
use rustc_hash::FxHashMap;
use taffy::prelude::*;

use super::{
    components::{UINode, UIRender},
    ctx::{NodeContext, UINodeType, measure},
    style::UIStyle,
    widgets::{UIImage, UIText},
};

#[derive(Clone, Copy, Debug)]
pub(super) enum UINodeGraph {
    Node(Entity),
    Begin(Entity),
    End(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct UICameraInfo {
    pub cam_size: Vec2,
    pub top_left: Vec2,
    pub layout_size: Vec2,
    pub projection: Mat4,
    pub inverse_projection: Mat4,
    pub transform: Mat3,
    pub inverse_transform: Mat3,
}

impl UICameraInfo {
    /// Creates a new UICameraInfo from a Camera2D
    fn from_base(cam: &Camera2D) -> Self {
        let bounds = cam.bounds();
        Self {
            cam_size: cam.size(),
            top_left: bounds.origin,
            layout_size: bounds.size,
            projection: cam.projection(),
            inverse_projection: cam.inverse_projection(),
            transform: cam.transform(),
            inverse_transform: cam.inverse_transform(),
        }
    }

    /// Updates the camera size and recalculates related matrices
    /// Returns true if the size was changed
    fn update_size(&mut self, size: Vec2) -> bool {
        if self.layout_size == size {
            return false;
        }

        let can_update = size.x > 0.0 && size.y > 0.0;
        if !can_update {
            return false;
        }

        self.cam_size = size;
        self.layout_size = size;
        self.projection =
            Mat4::orthographic_rh(0.0, self.layout_size.x, self.layout_size.y, 0.0, 0.0, 1.0);
        self.inverse_projection = self.projection.inverse();
        self.transform = Mat3::IDENTITY;
        self.inverse_transform = Mat3::IDENTITY;

        true
    }

    /// Converts screen coordinates to local node coordinates
    pub fn screen_to_local(&self, screen_pos: Vec2, local_inverse_transform: Mat3) -> Vec2 {
        let norm = screen_pos / self.cam_size;
        let mouse_pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        let pos = self
            .inverse_projection
            .project_point3(vec3(mouse_pos.x, mouse_pos.y, 1.0));

        local_inverse_transform.transform_point2(pos.xy())
    }
}

#[derive(Debug, Resource)]
pub struct UILayout<T>
where
    T: Component,
{
    _m: std::marker::PhantomData<T>,
    dirty_layout: bool,
    dirty_graph: bool,
    relations: FxHashMap<Entity, NodeId>,
    tree: TaffyTree<NodeContext>,
    root: NodeId,
    pub(super) base_transform: Mat3,

    pub(super) graph: Vec<UINodeGraph>,
    pub(super) cam_info: UICameraInfo,
}

impl<T> Default for UILayout<T>
where
    T: Component,
{
    /// Creates a default UILayout with an empty tree and default camera info
    fn default() -> Self {
        let mut tree = TaffyTree::<NodeContext>::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        let container = tree.new_leaf(Style::default()).unwrap();
        tree.add_child(root, container).unwrap();

        Self {
            _m: Default::default(),
            dirty_layout: true,
            dirty_graph: true,
            relations: FxHashMap::default(),
            tree,
            graph: vec![],

            cam_info: UICameraInfo {
                cam_size: Vec2::ZERO,
                top_left: Vec2::ZERO,
                layout_size: Vec2::ZERO,
                projection: Mat4::IDENTITY,
                inverse_projection: Mat4::IDENTITY,
                transform: Mat3::IDENTITY,
                inverse_transform: Mat3::IDENTITY,
            },

            root,
            base_transform: Mat3::IDENTITY,
        }
    }
}

impl<T> UILayout<T>
where
    T: Component,
{
    /// Updates the root node style based on the current camera info
    fn update_root(&mut self) {
        self.tree
            .set_style(
                self.root,
                Style {
                    size: Size {
                        width: Dimension::Length(self.cam_info.layout_size.x),
                        height: Dimension::Length(self.cam_info.layout_size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        self.base_transform = Mat3::from_translation(self.cam_info.top_left);
    }

    /// Returns the parent entity of the given entity in the UI tree
    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.relations
            .get(&entity)
            .and_then(|id| self.tree.parent(*id))
            .and_then(|parent| self.tree.get_node_context(parent))
            .map(|parent_ctx| parent_ctx.entity)
    }

    /// Returns how many immediate children this entity has in the UI-tree.
    pub fn child_count(&self, entity: Entity) -> usize {
        if let Some(&node_id) = self.relations.get(&entity) {
            self.tree.child_ids(node_id).count()
        } else {
            0
        }
    }

    /// Returns true if this entity has any children.
    pub fn has_children(&self, entity: Entity) -> bool {
        if let Some(&node_id) = self.relations.get(&entity) {
            self.tree.child_ids(node_id).next().is_some()
        } else {
            false
        }
    }

    /// Returns true if this entity has no children.
    pub fn is_empty(&self, entity: Entity) -> bool {
        !self.has_children(entity)
    }

    /// Returns an iterator over the immediate children of `parent` in the UI tree.
    pub fn children(&self, parent: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.relations
            .get(&parent)
            .into_iter()
            .flat_map(move |&parent_id| self.tree.child_ids(parent_id))
            .filter_map(move |child_id| self.tree.get_node_context(child_id).map(|ctx| ctx.entity))
    }

    /// Converts screen coordinates to node-local coordinates
    #[inline]
    pub fn screen_to_node(&self, screen_pos: Vec2, node: &UINode) -> Vec2 {
        self.cam_info.screen_to_local(
            screen_pos,
            node.global_transform().inverse() * self.cam_info.inverse_transform,
        )
    }

    /// Converts node-local coordinates to screen coordinates
    pub fn node_to_screen(&self, point: Vec2, node: &UINode) -> Vec2 {
        let transform = self.cam_info.transform * node.global_transform();
        let half = self.cam_info.cam_size * 0.5;
        let pos = transform * vec3(point.x, point.y, 1.0);
        let pos = self.cam_info.projection * vec4(pos.x, pos.y, pos.z, 1.0);
        half + (half * vec2(pos.x, -pos.y))
    }

    /// Converts coordinates from one node's local space to another node's local space
    #[inline]
    pub fn node_to_node(&self, point: Vec2, from: &UINode, to: &UINode) -> Vec2 {
        self.screen_to_node(self.node_to_screen(point, from), to)
    }

    /// Returns the current layout size
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.cam_info.layout_size
    }

    /// Sets the layout size and updates the root node if necessary
    pub fn set_size(&mut self, size: Vec2) {
        let updated = self.cam_info.update_size(size);
        if updated {
            self.dirty_layout = true;
            self.update_root();
        }
    }

    /// Updates the camera info from a Camera2D and marks layout as dirty if needed
    pub fn set_camera(&mut self, cam: &Camera2D) {
        let cam_size = cam.size();
        let can_set = cam_size.x > 0.0 && cam_size.y > 0.0;
        if !can_set {
            return;
        }

        let info = UICameraInfo::from_base(cam);
        if info != self.cam_info {
            self.cam_info = info;
            self.dirty_layout = true;
            self.update_root();
        }
    }

    /// Updates the layout tree and graph if they're dirty
    /// Returns true if any computation was performed
    pub fn update(
        &mut self,
        images: Query<&UIImage, With<T>>,
        texts: Query<&UIText, With<T>>,
    ) -> bool {
        let needs_compute = self.dirty_graph || self.dirty_layout;
        if needs_compute {
            self.tree
                .compute_layout_with_measure(
                    self.root,
                    Size {
                        width: AvailableSpace::Definite(self.cam_info.layout_size.x),
                        height: AvailableSpace::Definite(self.cam_info.layout_size.y),
                    },
                    |known_dimensions, available_space, _node_id, ctx, _style| {
                        measure(known_dimensions, available_space, ctx, &images, &texts)
                    },
                )
                .unwrap();
        }

        if self.dirty_graph {
            self.graph.clear();
            process_graph(&mut self.graph, self.root, &self.tree);
        }

        self.dirty_layout = false;
        self.dirty_graph = false;
        needs_compute
    }

    /// Adds a new node to the layout tree with the given style and type
    pub(super) fn add_raw_node(
        &mut self,
        entity: Entity,
        style: Style,
        typ: UINodeType,
        parent: Option<NodeId>,
    ) -> NodeId {
        self.dirty_layout = true;
        self.dirty_graph = true;
        let node_id = self
            .tree
            .new_leaf_with_context(style, NodeContext { entity, typ })
            .unwrap();
        let parent_id = parent.unwrap_or(self.root);
        self.tree.add_child(parent_id, node_id).unwrap();
        self.relations.insert(entity, node_id);
        node_id
    }

    /// Adds a child node to a parent node, reparenting if necessary
    pub(super) fn add_child(&mut self, parent: Entity, child: Entity) {
        if let (Some(parent_id), Some(node_id)) =
            (self.relations.get(&parent), self.relations.get(&child))
        {
            if let Some(prev_parent) = self.tree.parent(*node_id) {
                self.tree.remove_child(prev_parent, *node_id).unwrap();
            }
            self.tree.add_child(*parent_id, *node_id).unwrap();
            self.dirty_graph = true;
        }
    }

    /// Removes a node and its children from the layout tree
    pub(super) fn remove_node(&mut self, entity: Entity) {
        if let Some(node_id) = self.relations.remove(&entity) {
            self.tree.remove(node_id).unwrap();
            self.dirty_graph = true;
        }
    }

    /// Returns all entities in the subtree starting from the given entity
    pub(super) fn tree_from_node(&self, entity: Entity) -> Vec<Entity> {
        debug_assert!(
            !self.dirty_graph,
            "The graph must be updated to get the right tree from a node"
        );
        let Some(start_idx) = self.graph.iter().position(|ng| match ng {
            UINodeGraph::Begin(e) => e == &entity,
            _ => false,
        }) else {
            return vec![];
        };

        let Some(end_idx) = self.graph.iter().position(|ng| match ng {
            UINodeGraph::End(e) => e == &entity,
            _ => false,
        }) else {
            return vec![];
        };

        self.graph[start_idx..=end_idx]
            .iter()
            .filter_map(|ng| match ng {
                UINodeGraph::Node(entity) => Some(*entity),
                _ => None,
            })
            .collect()
    }

    /// Updates the style of a node in the layout tree
    pub(super) fn set_node_style(&mut self, node: &UINode, style: &UIStyle) {
        self.tree
            .set_style(node.node_id, style.as_taffy_style())
            .unwrap();
        self.dirty_layout = true;
    }

    /// Updates a node's size and position based on the computed layout
    pub(super) fn set_node_layout(&self, node: &mut UINode) {
        let l = self.tree.layout(node.node_id).unwrap();
        node.size = vec2(l.size.width, l.size.height);
        node.position = vec2(l.location.x, l.location.y) + vec2(l.margin.left, l.margin.top);
    }
}

/// Recursively processes the layout tree to build a flat graph representation
fn process_graph(graph: &mut Vec<UINodeGraph>, node_id: NodeId, tree: &TaffyTree<NodeContext>) {
    match tree.get_node_context(node_id).cloned() {
        Some(ctx) => {
            graph.push(UINodeGraph::Begin(ctx.entity));
            graph.push(UINodeGraph::Node(ctx.entity));
            tree.child_ids(node_id)
                .for_each(|child_id| process_graph(graph, child_id, tree));
            graph.push(UINodeGraph::End(ctx.entity));
        }
        _ => {
            tree.child_ids(node_id)
                .for_each(|child_id| process_graph(graph, child_id, tree));
        }
    }
}

/// Draws the entire UI layout
pub fn draw_ui_layout<T>(draw: &mut Draw2D, world: &mut World)
where
    T: Component,
{
    draw_ui_layout_from::<T>(draw, world, None)
}

/// Draws a portion of the UI layout starting from a specific entity
pub fn draw_ui_layout_from<T>(draw: &mut Draw2D, world: &mut World, from: Option<Entity>)
where
    T: Component,
{
    world.resource_scope(|world: &mut World, layout: Mut<UILayout<T>>| {
        // is form is none draw all the graph
        let mut rendering = from.is_none();

        for ng in &layout.graph {
            match ng {
                UINodeGraph::Node(entity) => {
                    // skip the entitiy if is not inside the graph we want
                    if !rendering {
                        continue;
                    }

                    // if the node continas a render component then render
                    if let (Some(render), Some(node)) =
                        (world.get::<UIRender>(*entity), world.get::<UINode>(*entity))
                    {
                        // store current values
                        let last_alpha = draw.alpha();

                        // set layout's node values
                        draw.set_alpha(last_alpha * node.global_alpha);
                        draw.push_matrix(node.global_transform);

                        // draw if necessary
                        if draw.alpha() > 0.0 {
                            render.render(draw, world, *entity);
                        }

                        // restore old values
                        draw.pop_matrix();
                        draw.set_alpha(last_alpha);
                    };
                }
                UINodeGraph::Begin(entity) => {
                    // skip if we're already rendering
                    if rendering {
                        continue;
                    }

                    // start rendering if we reached the node we want
                    rendering = from.is_some_and(|e| &e == entity);
                }
                UINodeGraph::End(entity) => {
                    // skip if we're not rendering
                    if !rendering {
                        continue;
                    }

                    // stop rendering if we reached the end of the node's children
                    if let Some(e) = from {
                        if &e == entity {
                            // rendering = false;
                            break;
                        }
                    }
                }
            }
        }
    });
}
