mod style;

use corelib::gfx;
use corelib::gfx::Color;
use corelib::math::{vec2, Vec2};
use draw::{create_draw_2d, Draw2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use std::ops::Rem;
use taffy::prelude::{auto, length, TaffyMaxContent};
use taffy::{AlignItems, FlexDirection, JustifyContent, Layout, NodeId, Size, Style, TaffyTree};

pub struct NuiLayout<'a> {
    draw: &'a mut Draw2D,
    // mouse_info: MouseInfo,
    // theme: Theme,
    // data: CustomData
}

impl<'a> NuiLayout<'a> {
    pub fn new(draw: &'a mut Draw2D) -> Self {
        Self { draw }
    }
    pub fn show<F: FnOnce(&mut NuiContext)>(self, cb: F) {
        // TODO use all the context here and do the thing
        let Self { draw } = self;
        layout(draw, cb);
    }
}

#[derive(Default)]
pub struct NodeInfo {
    pub style: Style,
}

pub trait NuiNode {
    fn style(&self) -> Style {
        Style::default()
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout) {}

    fn add_to_ctx_with<F: FnOnce(&mut NuiContext)>(self, ctx: &mut NuiContext, cb: F)
    where
        Self: Sized + 'static,
    {
        ctx.add_node_with(self, cb);
    }

    fn add_to_ctx(self, ctx: &mut NuiContext)
    where
        Self: Sized + 'static,
    {
        ctx.add_node(self);
    }
}

pub trait NuiWidget {
    fn ui(&self, ctx: &mut NuiContext);

    fn add_to_ctx(self, ctx: &mut NuiContext)
    where
        Self: Sized + 'static,
    {
        ctx.add_widget(self);
    }
}
pub struct NuiContext {
    nodes: FxHashMap<NodeId, Box<dyn NuiNode>>,
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

impl NuiContext {
    fn add_node_with<F: FnOnce(&mut Self), N: NuiNode + 'static>(&mut self, node: N, cb: F) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn add_node<N: NuiNode + 'static>(&mut self, node: N) -> NodeId {
        let style = node.style();
        let node_id = self.tree.new_leaf(style).unwrap();
        self.nodes.insert(node_id, Box::new(node));

        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
    }

    fn add_widget<W>(&mut self, widget: W)
    where
        W: NuiWidget + 'static,
    {
        widget.ui(self);
    }

    pub fn size(&self) -> Vec2 {
        self.size
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
pub struct Node<'a> {
    id: Option<&'static str>,
    style: NodeStyle,
    dr: Option<&'a dyn FnOnce()>,
}

impl<'a> NuiNode for Node<'a> {
    fn style(&self) -> Style {
        taffy_style_from(self.style)
    }

    fn render(&self, draw: &mut Draw2D, layout: Layout) {
        draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
            .color(self.style.color);
    }
}

impl<'a> Node<'a> {
    pub fn new(id: &'static str) -> Self {
        Self {
            id: Some(id),
            ..Default::default()
        }
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

fn layout<F: FnOnce(&mut NuiContext)>(draw: &mut Draw2D, cb: F) {
    let size = draw.size();
    let mut tree = TaffyTree::<()>::new();
    let root_id = tree
        .new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: length(size.x),
                height: length(size.y),
            },
            ..Default::default()
        })
        .unwrap();

    let mut ctx = NuiContext {
        nodes: Default::default(),
        node_stack: vec![root_id],
        tree,
        size,
    };

    cb(&mut ctx);

    let NuiContext {
        mut tree, nodes, ..
    } = ctx;

    tree.compute_layout(root_id, Size::MAX_CONTENT).unwrap();

    fn draw_node(
        node_id: NodeId,
        nodes: &FxHashMap<NodeId, Box<dyn NuiNode>>,
        tree: &mut TaffyTree<()>,
        draw: &mut Draw2D,
    ) {
        let layout = tree.layout(node_id).unwrap();
        println!("\n{node_id:?}:\n{layout:?}");
        draw.push_matrix(
            Transform2DBuilder::default()
                .set_translation(vec2(layout.location.x, layout.location.y))
                .build()
                .as_mat3(),
        );

        if let Some(node) = nodes.get(&node_id) {
            node.render(draw, *layout);
        }

        for child_id in tree.children(node_id).unwrap() {
            draw_node(child_id, nodes, tree, draw);
        }

        draw.pop_matrix();
    }

    println!("--------------------");
    draw_node(root_id, &nodes, &mut tree, draw);
    println!("++++++++++++++++++++");
}
