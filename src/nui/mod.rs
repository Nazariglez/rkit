pub mod layout;
pub mod node;
pub mod prelude;
pub mod style;

use bumpalo::Bump;
use corelib::math::Vec2;
use draw::Draw2D;
use node::{Node, NuiWidget};
use rustc_hash::FxHashMap;
use style::taffy_style_from;
use taffy::{Layout, NodeId, TaffyTree};

// TODO: cache global
//
struct RenderCallback<T>
where
    T: FnOnce(&mut Draw2D, Layout),
{
    cb: Option<T>,
}

pub trait CallRenderCallback {
    fn call(&mut self, draw: &mut Draw2D, layout: Layout);
}

impl<T> CallRenderCallback for RenderCallback<T>
where
    T: FnOnce(&mut Draw2D, Layout),
{
    fn call(&mut self, draw: &mut Draw2D, layout: Layout) {
        if let Some(cb) = self.cb.take() {
            cb(draw, layout);
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum CtxId {
    Temp(u64),
    Node(NodeId),
}

pub struct NuiContext<'a, T> {
    temp_id: u64,
    data: &'a T,
    bump: &'a Bump,
    nodes: &'a mut FxHashMap<CtxId, &'a mut dyn CallRenderCallback>,
    node_stack: Vec<NodeId>,
    tree: TaffyTree<()>,
    size: Vec2,
}

impl<'a, T> NuiContext<'a, T> {
    fn on_render<F: FnOnce(&mut Draw2D, Layout) + 'a>(&mut self, temp_id: u64, cb: F) {
        let obj = self.bump.alloc(RenderCallback { cb: Some(cb) }) as &mut dyn CallRenderCallback;
        self.nodes.insert(CtxId::Temp(temp_id), obj);
    }

    fn add_node_with<F: FnOnce(&mut Self)>(&mut self, node: Node<T>, cb: F) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn add_node(&mut self, node: Node<T>) -> NodeId {
        self.insert_node(node)
    }

    fn insert_node(&mut self, mut node: Node<T>) -> NodeId {
        let style = node.style;

        let node_id = self.tree.new_leaf(taffy_style_from(&style.layout)).unwrap();
        match self.nodes.entry(CtxId::Temp(node.temp_id)) {
            std::collections::hash_map::Entry::Occupied(e) => {
                let val = e.remove();
                self.nodes.insert(CtxId::Node(node_id), val);
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                // TODO: this
            }
        };

        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
    }

    fn add_widget<W>(&'a mut self, widget: W)
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
