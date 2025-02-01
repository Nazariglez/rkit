use bumpalo::Bump;
use corelib::math::{bvec2, vec2, Vec2};
use corelib::time;
use draw::{Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use taffy::prelude::length;
use taffy::{AvailableSpace, NodeId, Size, Style, TaffyTree};

use crate::nui::{CallRenderCallback, CtxId, RenderCallback};

use super::NuiContext;

pub trait Draw2DUiExt {
    fn ui<'layout>(&'layout mut self) -> NuiLayout<()>;
    fn ui_with<'layout, T>(&'layout mut self, data: &'layout T) -> NuiLayout<T>;
}

impl Draw2DUiExt for Draw2D {
    fn ui<'layout>(&'layout mut self) -> NuiLayout<()> {
        self.ui_with(&())
    }

    fn ui_with<'layout, T>(&'layout mut self, data: &'layout T) -> NuiLayout<T> {
        NuiLayout::new(self, data)
    }
}

pub struct NuiLayout<'data, T> {
    draw: &'data mut Draw2D,
    data: &'data T,
    size: Option<Vec2>,

    transform2d: Option<Transform2D>,
    // mouse_info: MouseInfo,
}

impl<'data, T> NuiLayout<'data, T> {
    fn new(draw: &'data mut Draw2D, data: &'data T) -> Self {
        Self {
            draw,
            data,
            size: None,
            transform2d: None,
        }
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn show<'arena: 'data, F: FnOnce(&'data mut NuiContext<'data, 'arena, T>)>(self, cb: F) {
        // layout(self, cb);
        let NuiLayout {
            draw,
            data,
            size,
            transform2d,
        } = self;
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
            temp_id: 0,
            data,
            bump: &bump,
            nodes: &mut nodes,
            node_stack,
            tree,
            size,
        };

        let now = time::now();
        // cb(&mut ctx);
        println!("definition elapsed: {:?}", now.elapsed());

        let NuiContext {
            mut tree,
            mut nodes,
            ..
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
            callbacks: &mut FxHashMap<CtxId, &mut dyn CallRenderCallback>,
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

            if let Some(cbs) = callbacks.get_mut(&CtxId::Node(node_id)) {
                cbs.call(draw, *layout);
            }

            for child_id in tree.children(node_id).unwrap() {
                draw_node(child_id, callbacks, tree, draw, data);
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
        draw_node(root_id, &mut nodes, &mut tree, draw, data);

        if use_transform {
            draw.pop_matrix();
        }
        println!("draw elapsed {:?}", now.elapsed());
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

// pub(super) fn layout<'ctx, 'layout: 'ctx, D: 'ctx, F>(layout: NuiLayout<'layout, D>, cb: F)
// where
//     F: FnOnce(&'ctx mut NuiContext<'layout, 'ctx, D>),
// {
//     let NuiLayout {
//         draw,
//         data,
//         size,
//         transform2d,
//     } = layout;
//     let size = size.unwrap_or(draw.size());

//     let mut tree = TaffyTree::<()>::new();
//     let root_id = tree
//         .new_leaf(Style {
//             flex_grow: 1.0,
//             size: Size {
//                 width: length(size.x),
//                 height: length(size.y),
//             },
//             ..Default::default()
//         })
//         .unwrap();

//     let bump = Bump::new();
//     let mut nodes = FxHashMap::default();

//     let mut node_stack = Vec::with_capacity(200);
//     node_stack.push(root_id);

//     let mut ctx = NuiContext {
//         temp_id: 0,
//         data,
//         bump: &bump,
//         nodes: &mut nodes,
//         node_stack,
//         tree,
//         size,
//     };

//     let now = time::now();
//     cb(&mut ctx);
//     println!("definition elapsed: {:?}", now.elapsed());

//     let NuiContext {
//         mut tree,
//         mut nodes,
//         ..
//     } = ctx;

//     let now = time::now();
//     tree.compute_layout(
//         root_id,
//         Size {
//             width: AvailableSpace::Definite(size.x),
//             height: AvailableSpace::Definite(size.y),
//         },
//     )
//     .unwrap();
//     println!("layout elapsed {:?}", now.elapsed());

//     fn draw_node<T>(
//         node_id: NodeId,
//         callbacks: &mut FxHashMap<CtxId, &mut dyn CallRenderCallback>,
//         tree: &mut TaffyTree<()>,
//         draw: &mut Draw2D,
//         data: &T,
//     ) {
//         let layout = tree.layout(node_id).unwrap();
//         // println!("\n{node_id:?}:\n{layout:?}");
//         draw.push_matrix(
//             Transform2DBuilder::default()
//                 .set_translation(vec2(layout.location.x, layout.location.y))
//                 .build()
//                 .as_mat3(),
//         );

//         if let Some(cbs) = callbacks.get_mut(&CtxId::Node(node_id)) {
//             cbs.call(draw, *layout);
//         }

//         for child_id in tree.children(node_id).unwrap() {
//             draw_node(child_id, callbacks, tree, draw, data);
//         }

//         draw.pop_matrix();
//     }

//     let now = time::now();

//     let use_transform = transform2d.is_some();
//     if let Some(mut transform) = transform2d {
//         transform.set_size(size);
//         draw.push_matrix(transform.updated_mat3());
//     }
//     // println!("--------------------");
//     draw_node(root_id, &mut nodes, &mut tree, draw, data);

//     if use_transform {
//         draw.pop_matrix();
//     }
//     println!("draw elapsed {:?}", now.elapsed());
//     // println!("++++++++++++++++++++");
// }
