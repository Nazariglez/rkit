use corelib::gfx;
use corelib::gfx::Color;
use corelib::math::{vec2, Vec2};
use draw::{create_draw_2d, Draw2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use std::ops::Rem;
use taffy::prelude::{auto, length, TaffyMaxContent};
use taffy::{AlignItems, FlexDirection, JustifyContent, Layout, NodeId, Size, Style, TaffyTree};

pub trait NuiWidget {
    fn ui(self, ctx: &mut NuiContext)
    where
        Self: Sized,
    {
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout) {}
}

pub struct NuiContext {
    nodes: FxHashMap<NodeId, Box<dyn NuiWidget>>,
    node_stack: Vec<NodeId>,
    tree: TaffyTree<NodeStyle>,
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
    fn append_with<F: FnOnce(&mut Self)>(&mut self, node: Node, cb: F) {
        let node_id = self.append(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn append(&mut self, node: Node) -> NodeId {
        let style = taffy_style_from(node.style);
        let node_id = self.tree.new_leaf_with_context(style, node.style).unwrap();
        self.nodes.insert(node_id, Box::new(node));

        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
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
pub struct Node {
    id: Option<&'static str>,
    style: NodeStyle,
}

impl NuiWidget for Node {
    fn render(&self, draw: &mut Draw2D, layout: Layout) {
        draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
            .color(self.style.color);
    }
}

impl Node {
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

    pub fn show_with<F: FnOnce(&mut NuiContext)>(self, ctx: &mut NuiContext, cb: F) {
        ctx.append_with(self, cb);
    }

    pub fn show(self, ctx: &mut NuiContext) {
        ctx.append(self);
    }
}

pub fn layout<F: FnOnce(&mut NuiContext)>(size: Vec2, cb: F) {
    let mut tree = TaffyTree::<NodeStyle>::new();
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

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    fn draw_node(
        node_id: NodeId,
        nodes: &FxHashMap<NodeId, Box<dyn NuiWidget>>,
        tree: &mut TaffyTree<NodeStyle>,
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

        let color = match tree.get_node_context(node_id) {
            None => Color::TRANSPARENT,
            Some(c) => c.color,
        };

        if let Some(node) = nodes.get(&node_id) {
            node.render(draw, *layout);
        }
        // draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
        //     .color(color);

        for child_id in tree.children(node_id).unwrap() {
            draw_node(child_id, nodes, tree, draw);
        }

        draw.pop_matrix();
    }

    println!("--------------------");
    draw_node(root_id, &nodes, &mut tree, &mut draw);
    println!("++++++++++++++++++++");

    gfx::render_to_frame(&draw).unwrap();
}
