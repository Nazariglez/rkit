use crate::draw::{Draw2D, Transform2D};
use crate::math::{vec2, Vec2};
use bevy_ecs::prelude::*;
use draw::BaseCam2D;
use taffy::prelude::*;

use super::components::{UINode, UIRender};

#[derive(Clone, Copy, Debug)]
enum UINodeGraph {
    Node(Entity),
    Begin(Entity),
    End(Entity),
}

#[derive(Debug, Clone, Copy)]
pub struct UILayoutId<T>
where
    T: Send + Sync,
{
    _m: std::marker::PhantomData<T>,
}

impl<T> Default for UILayoutId<T>
where
    T: Send + Sync,
{
    fn default() -> Self {
        Self {
            _m: Default::default(),
        }
    }
}

#[derive(Debug, Resource)]
pub struct UILayout<T>
where
    T: Send + Sync,
{
    id: UILayoutId<T>,
    dirty_layout: bool,
    dirty_graph: bool,
    tree: TaffyTree<Entity>,
    graph: Vec<UINodeGraph>,
    size: Vec2,
    root: NodeId,
}

impl<T> Default for UILayout<T>
where
    T: Send + Sync,
{
    fn default() -> Self {
        let mut tree = TaffyTree::<Entity>::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        Self {
            id: UILayoutId::default(),
            dirty_layout: true,
            dirty_graph: true,
            tree,
            graph: vec![],
            root,
            size: Vec2::ZERO,
        }
    }
}

impl<T> UILayout<T>
where
    T: Send + Sync,
{
    pub fn set_size(&mut self, size: Vec2) {
        if size == self.size {
            return;
        }

        self.size = size;
        self.tree
            .set_style(
                self.root,
                Style {
                    size: Size {
                        width: Dimension::Length(size.x),
                        height: Dimension::Length(size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        self.dirty_layout = true;
    }

    pub fn add_node(&mut self, entity: Entity, style: Style, parent: Option<NodeId>) -> NodeId {
        self.dirty_layout = true;
        self.dirty_graph = true;
        let node_id = self.tree.new_leaf_with_context(style, entity).unwrap();
        let parent_id = parent.unwrap_or(self.root);
        self.tree.add_child(parent_id, node_id).unwrap();
        node_id
    }

    pub fn update(&mut self, cam: Option<&dyn BaseCam2D>) -> bool {
        let needs_compute = self.dirty_graph || self.dirty_layout;
        if needs_compute {
            self.tree
                .compute_layout(
                    self.root,
                    Size {
                        width: AvailableSpace::Definite(self.size.x),
                        height: AvailableSpace::Definite(self.size.y),
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

    pub fn update_node(&self, node: &mut UINode) {
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
        _ => tree
            .child_ids(node_id)
            .for_each(|child_id| process_graph(graph, child_id, tree)),
    }
}

pub fn draw_ui<T>(draw: &mut Draw2D, world: &mut World)
where
    T: Send + Sync + 'static,
{
    world.resource_scope(|world: &mut World, layout: Mut<UILayout<T>>| {
        layout.graph.iter().for_each(|ng| match ng {
            UINodeGraph::Begin(entity) => {
                if let Some(node) = world.get::<UINode>(*entity) {
                    draw.push_matrix(
                        Transform2D::builder()
                            .set_size(node.size)
                            .set_translation(node.position)
                            .build()
                            .as_mat3(),
                    );
                }
            }
            UINodeGraph::Node(entity) => {
                if let Some(render) = world.get::<UIRender>(*entity) {
                    render.render(draw, world, *entity);
                };
            }
            UINodeGraph::End(_entity) => {
                if world.entity(*_entity).contains::<UINode>() {
                    draw.pop_matrix();
                }
            }
        });
    });
}
