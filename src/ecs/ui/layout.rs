use crate::draw::{BaseCam2D, Draw2D};
use crate::math::{vec2, Vec2};
use bevy_ecs::prelude::*;
use corelib::math::{vec3, Mat3, Mat4};
use rustc_hash::FxHashMap;
use taffy::prelude::*;

use super::components::{UINode, UIRender};
use super::style::UIStyle;

#[derive(Clone, Copy, Debug)]
pub(super) enum UINodeGraph {
    Node(Entity),
    Begin(Entity),
    End(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct UICameraInfo {
    pub size: Vec2,
    pub projection: Mat4,
    pub inverse_projection: Mat4,
    pub transform: Mat3,
    pub inverse_transform: Mat3,
}

impl UICameraInfo {
    fn from_base(cam: &impl BaseCam2D) -> Self {
        Self {
            size: cam.size(),
            projection: cam.projection(),
            inverse_projection: cam.inverse_projection(),
            transform: cam.transform(),
            inverse_transform: cam.inverse_transform(),
        }
    }

    fn update_size(&mut self, size: Vec2) -> bool {
        if self.size == size {
            return false;
        }

        self.size = size;
        self.projection = Mat4::orthographic_rh(0.0, self.size.x, self.size.y, 0.0, 0.0, 1.0);
        self.inverse_projection = self.projection.inverse();
        self.transform = Mat3::IDENTITY;
        self.inverse_transform = Mat3::IDENTITY;

        true
    }

    pub fn screen_to_local(&self, screen_pos: Vec2, local_inverse_transform: Mat3) -> Vec2 {
        let norm = screen_pos / self.size;
        let mouse_pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        let pos = self
            .inverse_projection
            .project_point3(vec3(mouse_pos.x, mouse_pos.y, 1.0));

        local_inverse_transform.transform_point2(vec2(pos.x, pos.y))
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
    tree: TaffyTree<Entity>,
    root: NodeId,

    pub(super) graph: Vec<UINodeGraph>,
    pub(super) cam_info: UICameraInfo,
}

impl<T> Default for UILayout<T>
where
    T: Component,
{
    fn default() -> Self {
        let mut tree = TaffyTree::<Entity>::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        Self {
            _m: Default::default(),
            dirty_layout: true,
            dirty_graph: true,
            relations: FxHashMap::default(),
            tree,
            graph: vec![],

            cam_info: UICameraInfo {
                size: Vec2::ZERO,
                projection: Mat4::IDENTITY,
                inverse_projection: Mat4::IDENTITY,
                transform: Mat3::IDENTITY,
                inverse_transform: Mat3::IDENTITY,
            },

            root,
        }
    }
}

impl<T> UILayout<T>
where
    T: Component,
{
    fn update_root_size(&mut self) {
        self.tree
            .set_style(
                self.root,
                Style {
                    size: Size {
                        width: Dimension::Length(self.cam_info.size.x),
                        height: Dimension::Length(self.cam_info.size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    pub fn set_size(&mut self, size: Vec2) {
        let updated = self.cam_info.update_size(size);
        if updated {
            self.dirty_layout = true;
            self.update_root_size();
        }
    }

    pub fn set_camera(&mut self, cam: &impl BaseCam2D) {
        let info = UICameraInfo::from_base(cam);
        if info != self.cam_info {
            self.cam_info = info;
            self.dirty_layout = true;
            self.update_root_size();
        }
    }

    pub fn update(&mut self) -> bool {
        let needs_compute = self.dirty_graph || self.dirty_layout;
        if needs_compute {
            self.tree
                .compute_layout(
                    self.root,
                    Size {
                        width: AvailableSpace::Definite(self.cam_info.size.x),
                        height: AvailableSpace::Definite(self.cam_info.size.y),
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

    pub(super) fn add_raw_node(
        &mut self,
        entity: Entity,
        style: Style,
        parent: Option<NodeId>,
    ) -> NodeId {
        self.dirty_layout = true;
        self.dirty_graph = true;
        let node_id = self.tree.new_leaf_with_context(style, entity).unwrap();
        let parent_id = parent.unwrap_or(self.root);
        self.tree.add_child(parent_id, node_id).unwrap();
        self.relations.insert(entity, node_id);
        node_id
    }

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

    pub(super) fn remove_node(&mut self, entity: Entity) {
        if let Some(node_id) = self.relations.remove(&entity) {
            self.tree.remove(node_id).unwrap();
            self.dirty_graph = true;
        }
    }

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

    pub(super) fn set_node_style(&mut self, node: &UINode, style: &UIStyle) {
        self.tree.set_style(node.node_id, style.to_taffy()).unwrap();
        self.dirty_layout = true;
    }

    pub(super) fn set_node_layout(&self, node: &mut UINode) {
        let l = self.tree.layout(node.node_id).unwrap();
        node.size = vec2(l.size.width, l.size.height);
        node.position = vec2(l.location.x, l.location.y);
    }
}

fn process_graph(graph: &mut Vec<UINodeGraph>, node_id: NodeId, tree: &TaffyTree<Entity>) {
    match tree.get_node_context(node_id).cloned() {
        Some(e) => {
            graph.push(UINodeGraph::Begin(e));
            graph.push(UINodeGraph::Node(e));
            tree.child_ids(node_id)
                .for_each(|child_id| process_graph(graph, child_id, tree));
            graph.push(UINodeGraph::End(e));
        }
        _ => {
            tree.child_ids(node_id)
                .for_each(|child_id| process_graph(graph, child_id, tree));
        }
    }
}

pub fn draw_ui_layout<T>(draw: &mut Draw2D, world: &mut World)
where
    T: Component,
{
    world.resource_scope(|world: &mut World, layout: Mut<UILayout<T>>| {
        layout.graph.iter().for_each(|ng| {
            if let UINodeGraph::Node(entity) = ng {
                if let (Some(render), Some(node)) =
                    (world.get::<UIRender>(*entity), world.get::<UINode>(*entity))
                {
                    let last_alpha = draw.alpha();
                    draw.set_alpha(last_alpha * node.global_alpha);
                    draw.push_matrix(node.global_transform);
                    if draw.alpha() > 0.0 {
                        render.render(draw, world, *entity);
                    }
                    draw.pop_matrix();
                    draw.set_alpha(last_alpha);
                };
            }
        });
    });
}
