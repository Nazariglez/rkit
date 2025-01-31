pub mod layout;
pub mod prelude;
pub mod style;

use bumpalo::Bump;
use corelib::gfx::Color;
use corelib::math::{bvec2, vec2, BVec2, Vec2};
use corelib::time;
use draw::{create_draw_2d, Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::ops::Rem;
use taffy::prelude::{auto, length, TaffyMaxContent};
use taffy::{
    AlignItems, AvailableSpace, FlexDirection, JustifyContent, Layout, NodeId, Size, Style,
    TaffyTree,
};

// TODO: cache global

#[derive(Default)]
pub struct NodeInfo {
    pub style: Style,
}

pub trait NuiNode {
    fn style(&self) -> Style {
        // TODO: visuals?
        Style::default()
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout) {}

    fn add_with_children<D, F: FnOnce(&mut NuiContext<D>)>(self, ctx: &mut NuiContext<D>, cb: F)
    where
        Self: Sized + 'static,
    {
        ctx.add_node_with(self, cb);
    }

    fn add<D>(self, ctx: &mut NuiContext<D>)
    where
        Self: Sized + 'static,
    {
        ctx.add_node(self);
    }
}

pub trait NuiNodeWithData<T> {
    fn style(&self, data: &T) -> Style {
        Style::default()
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout, data: &T) {}

    fn add_with_children<F: FnOnce(&mut NuiContext<T>)>(self, ctx: &mut NuiContext<T>, cb: F)
    where
        Self: Sized + 'static,
    {
        ctx.add_data_node_with(self, cb);
    }

    fn add(self, ctx: &mut NuiContext<T>)
    where
        Self: Sized + 'static,
    {
        ctx.add_data_node(self);
    }
}

pub enum NuiNodeType<'a, D> {
    Node(&'a dyn NuiNode),
    WithData(&'a dyn NuiNodeWithData<D>),
}

pub trait NuiWidget<T> {
    fn ui(self, ctx: &mut NuiContext<T>);

    fn add(self, ctx: &mut NuiContext<T>)
    where
        Self: Sized + 'static,
    {
        ctx.add_widget(self);
    }
}
pub struct NuiContext<'a, T> {
    data: &'a T,
    bump: &'a Bump,
    nodes: &'a mut FxHashMap<NodeId, NuiNodeType<'a, T>>,
    node_stack: Vec<NodeId>,
    tree: TaffyTree<()>,
    size: Vec2,
}

fn taffy_style_from(ns: NodeStyle) -> Style {
    let size = ns
        .size
        .map(|s| Size {
            width: length(s.x),
            height: length(s.y),
        })
        .unwrap_or(Size::auto());
    let gap = ns
        .gap
        .map(|s| Size {
            width: length(s.x),
            height: length(s.y),
        })
        .unwrap_or(Size::zero());
    Style {
        // flex_grow: 1.0,
        size,
        flex_direction: if ns.row {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        },
        align_items: ns.center_content.then_some(AlignItems::Center),
        justify_content: ns.center_content.then_some(JustifyContent::Center),
        gap,
        ..Default::default()
    }
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

        let node_id = self.tree.new_leaf(style).unwrap();
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

#[derive(Copy, Clone)]
pub struct NodeStyle {
    size: Option<Vec2>,
    gap: Option<Vec2>,
    color: Color,
    row: bool, // use enum
    center_content: bool,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            size: None,
            gap: None,
            color: Default::default(),
            row: false,
            center_content: false,
        }
    }
}

#[derive(Default)]
pub struct Node {
    style: NodeStyle,
}

impl NuiNode for Node {
    fn style(&self) -> Style {
        taffy_style_from(self.style)
    }

    fn render(&self, draw: &mut Draw2D, layout: Layout) {
        draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
            .color(self.style.color);
    }
}

impl Node {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style.color = color;
        self
    }

    pub fn top_bottom(mut self) -> Self {
        self.style.row = false;
        self
    }

    pub fn left_right(mut self) -> Self {
        self.style.row = true;
        self
    }

    pub fn content_horizontal_center(mut self) -> Self {
        self.style.center_content = true;
        self
    }

    pub fn content_gap(mut self, gap: Vec2) -> Self {
        self.style.gap = Some(gap);
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.style.size = Some(size);
        self
    }
}
