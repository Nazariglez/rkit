mod style;

use corelib::gfx::Color;
use corelib::math::{vec2, Vec2, bvec2, BVec2};
use draw::{create_draw_2d, Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::ops::Rem;
use taffy::prelude::{auto, length, TaffyMaxContent};
use taffy::{AlignItems, FlexDirection, JustifyContent, Layout, NodeId, Size, Style, TaffyTree};
use crate::macros::Drawable2D;

// TODO: cache global

pub trait Draw2DUiExt {
    fn ui(&mut self) -> NuiLayout<()>;
    fn ui_with<'a, T>(&'a mut self, data: &'a T) -> NuiLayout<T>;
}

impl Draw2DUiExt for Draw2D {
    fn ui(&mut self) -> NuiLayout<()> {
        self.ui_with(&())
    }

    fn ui_with<'a, T>(&'a mut self, data: &'a T) -> NuiLayout<T> {
        NuiLayout::new(self, data)
    }
}

pub struct NuiLayout<'a, T> {
    draw: &'a mut Draw2D,
    data: &'a T,
    size: Option<Vec2>,
    cache_id: Option<&'static str>,

    transform2d: Option<Transform2D>,
    // mouse_info: MouseInfo,
}

impl<'a, T> NuiLayout<'a, T> {
    fn new(draw: &'a mut Draw2D, data: &'a T) -> Self {
        Self {
            draw,
            data,
            size: None,
            cache_id: None,
            transform2d: None,
        }
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    pub fn cache(mut self, id: &'static str) -> Self {
        self.cache_id = Some(id);
        self
    }
    pub fn show<F: FnOnce(&mut NuiContext<T>)>(self, cb: F) {
        layout(self, cb);
    }

    // transform
    pub fn translate(mut self, pos: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_translation(pos);
        self
    }

    pub fn anchor(mut self, point: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_anchor(point);
        self
    }

    pub fn pivot(mut self, point: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_pivot(point);
        self
    }

    pub fn origin(mut self, point: Vec2) -> Self {
        self.anchor(point)
            .pivot(point)
    }

    pub fn flip_x(mut self, flip: bool) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_flip(bvec2(flip, t.flip().y));
        self
    }

    pub fn flip_y(mut self, flip: bool) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_flip(bvec2(t.flip().x, flip));
        self
    }

    pub fn skew(mut self, skew: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_skew(skew);
        self
    }

    pub fn scale(mut self, scale: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_scale(scale);
        self
    }

    pub fn rotation(mut self, rot: f32) -> Self {
        let t = self.transform2d.get_or_insert_with(|| Transform2D::default());
        t.set_rotation(rot);
        self
    }
}

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
        ctx.add_node_with(NuiNodeType::Node(Box::new(self)), cb);
    }

    fn add<D>(self, ctx: &mut NuiContext<D>)
    where
        Self: Sized + 'static,
    {
        ctx.add_node(NuiNodeType::Node(Box::new(self)));
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
        ctx.add_node_with(NuiNodeType::WithData(Box::new(self)), cb);
    }

    fn add(self, ctx: &mut NuiContext<T>)
    where
        Self: Sized + 'static,
    {
        ctx.add_node(NuiNodeType::WithData(Box::new(self)));
    }
}

pub enum NuiNodeType<D> {
    Node(Box<dyn NuiNode>),
    WithData(Box<dyn NuiNodeWithData<D>>),
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
    nodes: FxHashMap<NodeId, NuiNodeType<T>>,
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

impl<T> NuiContext<'_, T> {
    fn add_node_with<F: FnOnce(&mut Self)>(&mut self, node: NuiNodeType<T>, cb: F) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    fn add_node(&mut self, node: NuiNodeType<T>) -> NodeId {
        #[derive(Hash, Default)]
        struct SS {
            a1: i32,
            a2: i32,
            a3: i32,
            a4: i32,
            a5: i32,
            a6: i32,
            a7: i32,
            a8: i32,
            a9: i32,
            a10: i32,
            a11: i32,
            a12: i32,
            a13: i32,
            a14: i32,
            a15: i32,
            a16: i32,
            a17: i32,
            a18: i32,
            a19: i32,
            a20: i32,
            a21: i32,
            a22: i32,
            a23: i32,
            a24: i32,
            a25: i32,
            a26: i32,
            a27: i32,
            a28: i32,
            a29: i32,
            a30: i32,
            a31: i32,
            a32: i32,
            a33: i32,
            a34: i32,
            a35: i32,
            a36: i32,
            a37: i32,
            a38: i32,
            a39: i32,
            a40: i32,
            a41: i32,
            a42: i32,
            a43: i32,
            a44: i32,
            a45: i32,
            a46: i32,
            a47: i32,
            a48: i32,
            a49: i32,
            a50: i32,
            a51: i32,
            a52: i32,
            a53: i32,
            a54: i32,
            a55: i32,
            a56: i32,
            a57: i32,
            a58: i32,
            a59: i32,
            a60: i32,
            a61: i32,
            a62: i32,
            a63: i32,
            a64: i32,
            a65: i32,
            a66: i32,
            a67: i32,
            a68: i32,
            a69: i32,
            a70: i32,
            a71: i32,
            a72: i32,
            a73: i32,
            a74: i32,
            a75: i32,
            a76: i32,
            a77: i32,
            a78: i32,
            a79: i32,
            a80: i32,
            a81: i32,
            a82: i32,
            a83: i32,
            a84: i32,
            a85: i32,
            a86: i32,
            a87: i32,
            a88: i32,
            a89: i32,
            a90: i32,
        }
        let style = match &node {
            NuiNodeType::Node(n) => n.style(),
            NuiNodeType::WithData(n) => n.style(self.data),
        };

        // let mut hasher = FxHasher::default();
        // SS::default().hash(&mut hasher);
        // let hash = hasher.finish();
        // black_box(hash);

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

fn layout<D, F: FnOnce(&mut NuiContext<D>)>(
    layout: NuiLayout<D>,
    cb: F,
) {
    let NuiLayout {
        draw, data, size, cache_id, transform2d
    } = layout;
    let size = size.unwrap_or(draw.size());

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
        data,
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

    fn draw_node<T>(
        node_id: NodeId,
        nodes: &FxHashMap<NodeId, NuiNodeType<T>>,
        tree: &mut TaffyTree<()>,
        draw: &mut Draw2D,
        data: &T,
    ) {
        let layout = tree.layout(node_id).unwrap();
        // println!("\n{node_id:?}:\n{layout:?}");
        draw.push_matrix(
            Transform2DBuilder::default()
                .set_translation(vec2(layout.location.x, layout.location.y))
                .build()
                .as_mat3(),
        );

        if let Some(node) = nodes.get(&node_id) {
            match node {
                NuiNodeType::Node(node) => node.render(draw, *layout),
                NuiNodeType::WithData(node) => node.render(draw, *layout, data),
            }
        }

        for child_id in tree.children(node_id).unwrap() {
            draw_node(child_id, nodes, tree, draw, data);
        }

        draw.pop_matrix();
    }

    let use_transform = transform2d.is_some();
    if let Some(mut transform) = transform2d {
        transform.set_size(size);
        draw.push_matrix(transform.updated_mat3());
    }
    // println!("--------------------");
    draw_node(root_id, &nodes, &mut tree, draw, data);

    if use_transform {
        draw.pop_matrix();
    }
    // println!("++++++++++++++++++++");
}
