use super::node::{Node, NodeInput, NodeState, NuiWidget};
use bumpalo::Bump;
use corelib::math::Vec2;
use draw::Draw2D;

use std::{collections::hash_map::Entry, default};

use super::style::{taffy_style_from, Style};
use rustc_hash::FxHashMap;
use taffy::{Layout, NodeId, TaffyTree};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(super) enum CtxId {
    Temp(u64),
    Node(NodeId),
}

pub(super) type OnDrawCb<'data, T> = dyn for<'draw> Fn(&'draw mut Draw2D, Layout, &mut T) + 'data;

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
enum NodeType {
    #[default]
    Node,
    Text,
    Image,
    // Button
}

#[derive(Debug, Default)]
pub(super) struct NodeContext {
    pub skip: bool,
    pub typ: NodeType,
    pub state: NodeState,
    pub input: NodeInput,
}

// TODO: use ouroboros::self_referencing if possible so we can avoid the extra 'arena lifetime?

pub struct NuiContext<'data, 'arena, T>
where
    'data: 'arena,
    T: 'data,
{
    pub(super) temp_id: u64,
    pub(super) arena: &'arena Bump,
    pub(super) data: &'data mut T,
    pub(super) callbacks: FxHashMap<CtxId, &'arena mut OnDrawCb<'data, T>>,
    pub(super) cached_styles: Vec<Style>,
    pub(super) node_stack: Vec<NodeId>,
    pub(super) tree: TaffyTree<NodeContext>,
    pub(super) size: Vec2,
}

impl<'data, 'arena, T> NuiContext<'data, 'arena, T>
where
    'data: 'arena,
    T: 'data,
{
    #[inline]
    pub fn node<'ctx>(&'ctx mut self) -> Node<'ctx, 'data, 'arena, T> {
        Node::new(self)
    }

    #[inline]
    pub(super) fn on_draw<F: for<'draw> Fn(&'draw mut Draw2D, Layout, &mut T) + 'data>(
        &mut self,
        temp_id: u64,
        cb: F,
    ) {
        self.callbacks
            .insert(CtxId::Temp(temp_id), self.arena.alloc(cb));
    }

    #[inline]
    pub(super) fn add_node_with<F: FnOnce(&mut Self)>(
        &mut self,
        node: Node<'_, 'data, 'arena, T>,
        cb: F,
    ) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    pub(super) fn add_node<'ctx>(&'ctx mut self, node: Node<'ctx, 'data, 'arena, T>) -> NodeId {
        // process style to create the layout
        let node_id = self
            .tree
            .new_leaf_with_context(
                taffy_style_from(&node.style.layout),
                NodeContext {
                    input: NodeInput {
                        enabled: node.use_inputs,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        // if there is a callback assigned then replace it with the final id
        let temp_id = CtxId::Temp(node.temp_id);
        if let Entry::Occupied(e) = self.callbacks.entry(temp_id) {
            let val = e.remove();
            self.callbacks.insert(CtxId::Node(node_id), val);
        };

        // cache the style and add the node to the layout three
        self.cached_styles.push(node.style);
        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
    }

    #[inline]
    pub(super) fn add_widget<W>(&mut self, widget: W)
    where
        W: NuiWidget<T>,
    {
        widget.ui(self);
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }

    #[inline]
    pub fn data(&self) -> &T {
        self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
