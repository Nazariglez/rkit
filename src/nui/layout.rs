use crate::nui::{layout, NuiContext, NuiNodeType};
use bumpalo::Bump;
use corelib::math::{bvec2, vec2, Vec2};
use corelib::time;
use draw::{Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use taffy::prelude::length;
use taffy::{AvailableSpace, NodeId, Size, Style, TaffyTree};

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
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_translation(pos);
        self
    }

    pub fn anchor(mut self, point: Vec2) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_anchor(point);
        self
    }

    pub fn pivot(mut self, point: Vec2) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_pivot(point);
        self
    }

    pub fn origin(mut self, point: Vec2) -> Self {
        self.anchor(point).pivot(point)
    }

    pub fn flip_x(mut self, flip: bool) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_flip(bvec2(flip, t.flip().y));
        self
    }

    pub fn flip_y(mut self, flip: bool) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_flip(bvec2(t.flip().x, flip));
        self
    }

    pub fn skew(mut self, skew: Vec2) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_skew(skew);
        self
    }

    pub fn scale(mut self, scale: Vec2) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_scale(scale);
        self
    }

    pub fn rotation(mut self, rot: f32) -> Self {
        let t = self
            .transform2d
            .get_or_insert_with(|| Transform2D::default());
        t.set_rotation(rot);
        self
    }
}

pub(super) fn layout<D, F: FnOnce(&mut NuiContext<D>)>(layout: NuiLayout<D>, cb: F) {
    let NuiLayout {
        draw,
        data,
        size,
        cache_id,
        transform2d,
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

    let bump = Bump::new();
    let mut nodes = FxHashMap::default();

    let mut node_stack = Vec::with_capacity(200);
    node_stack.push(root_id);

    let mut ctx = NuiContext {
        data,
        bump: &bump,
        nodes: &mut nodes,
        node_stack,
        tree,
        size,
    };

    let now = time::now();
    cb(&mut ctx);
    println!("definition elapsed: {:?}", now.elapsed());

    let NuiContext {
        mut tree, nodes, ..
    } = ctx;

    let now = time::now();
    tree.compute_layout(
        root_id,
        Size {
            width: AvailableSpace::Definite(size.x),
            height: AvailableSpace::Definite(size.y),
        },
    )
    .unwrap();
    println!("layout elapsed {:?}", now.elapsed());

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

    let now = time::now();

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
    println!("draw elapsed {:?}", now.elapsed());
    // println!("++++++++++++++++++++");
}
