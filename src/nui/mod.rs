pub mod layout;
mod node;
pub mod prelude;
pub mod style;

use bumpalo::Bump;
use corelib::math::Vec2;
use node::{NuiNode, NuiNodeType, NuiNodeWithData, NuiWidget};
use rustc_hash::FxHashMap;
use style::taffy_style_from;
use taffy::{NodeId, TaffyTree};

// TODO: cache global

pub struct NuiContext<'a, T> {
    data: &'a T,
    bump: &'a Bump,
    nodes: &'a mut FxHashMap<NodeId, NuiNodeType<'a, T>>,
    node_stack: Vec<NodeId>,
    tree: TaffyTree<()>,
    size: Vec2,
}

impl<'a, T> NuiContext<'a, T> {
    fn add_node_with<F: FnOnce(&mut Self), N: NuiNode + 'a>(&mut self, node: N, cb: F) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn add_node<N: NuiNode + 'a>(&mut self, node: N) -> NodeId {
        let obj = self.bump.alloc(node) as &dyn NuiNode;
        self.insert_node(NuiNodeType::Node(obj))
    }

    fn add_data_node_with<F: FnOnce(&mut Self), N: NuiNodeWithData<T> + 'a>(
        &mut self,
        node: N,
        cb: F,
    ) {
        let node_id = self.add_data_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn add_data_node<N: NuiNodeWithData<T> + 'a>(&mut self, node: N) -> NodeId {
        let obj = self.bump.alloc(node) as &dyn NuiNodeWithData<T>;
        self.insert_node(NuiNodeType::WithData(obj))
    }

    fn insert_node(&mut self, node: NuiNodeType<'a, T>) -> NodeId {
        let style = match &node {
            NuiNodeType::Node(n) => n.style(),
            NuiNodeType::WithData(n) => n.style(self.data),
        };

        let node_id = self.tree.new_leaf(taffy_style_from(&style.layout)).unwrap();
        self.nodes.insert(node_id, node);

        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
    }

    fn add_widget<W>(&mut self, widget: W)
    where
        W: NuiWidget<T> + 'static,
    {
        widget.ui(self);
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn data(&self) -> &T {
        self.data
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}
