use corelib::math::{bvec2, vec2, Vec2};
use draw::{Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use taffy::prelude::length;
use taffy::{AvailableSpace, NodeId, Size, Style, TaffyTree};

use crate::nui::{CallRenderCallback, CtxId};

use super::{CacheId, NuiContext, CACHE};

const EMPTY_DATA: () = ();

pub trait Draw2DUiExt {
    fn ui<'draw>(&'draw mut self) -> NuiLayout<'_, 'draw, ()>;
    fn ui_with<'data, 'draw, T>(&'draw mut self, data: &'data T) -> NuiLayout<T>
    where
        'data: 'draw;
}

impl Draw2DUiExt for Draw2D {
    fn ui<'draw>(&'draw mut self) -> NuiLayout<'_, 'draw, ()> {
        self.ui_with(&())
    }

    fn ui_with<'data, 'draw, T>(&'draw mut self, data: &'data T) -> NuiLayout<T>
    where
        'data: 'draw,
    {
        NuiLayout::new(self, data)
    }
}

pub struct NuiLayout<'data, 'draw, T> {
    id: CacheId,
    draw: &'draw mut Draw2D,
    data: &'data T,
    size: Option<Vec2>,
    cache_disabled: bool,

    transform2d: Option<Transform2D>,
    // mouse_info: MouseInfo,
}

impl<'data, 'draw, T> NuiLayout<'data, 'draw, T> {
    fn new(draw: &'draw mut Draw2D, data: &'data T) -> Self {
        let id = CACHE.with_borrow_mut(|cache| cache.gen_id());
        Self {
            id,
            draw,
            data,
            size: None,
            cache_disabled: false,
            transform2d: None,
        }
    }

    pub fn id(mut self, id: &'static str) -> Self {
        self.id = CacheId::Named(id);
        self
    }

    pub fn disable_cache(mut self) -> Self {
        self.cache_disabled = true;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn show<F: FnOnce(&mut NuiContext<'data, T>)>(self, cb: F) {
        let NuiLayout {
            id: layout_id,
            cache_disabled,
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

        let mut ctx = NuiContext {
            temp_id: 0,
            data,
            nodes: FxHashMap::default(),
            node_stack: vec![root_id],
            tree,
            size,
            cache_styles: vec![],
        };

        cb(&mut ctx);

        let NuiContext {
            mut tree,
            mut nodes,
            cache_styles,
            ..
        } = ctx;

        CACHE.with_borrow_mut(|cache| {
            let is_valid_cache = !cache_disabled && cache.is_cache_valid(layout_id, &cache_styles);
            if !is_valid_cache {
                tree.compute_layout(
                    root_id,
                    Size {
                        width: AvailableSpace::Definite(size.x),
                        height: AvailableSpace::Definite(size.y),
                    },
                )
                .unwrap();
                cache.add_cache(layout_id, cache_styles, tree);
            }

            let use_transform = transform2d.is_some();
            if let Some(mut transform) = transform2d {
                transform.set_size(size);
                draw.push_matrix(transform.updated_mat3());
            }

            let tree = cache.layouts.get(&layout_id).as_ref().map(|(_, tree)| tree);
            if let Some(tree) = tree {
                draw_node(root_id, &mut nodes, tree, draw, data);
            }

            if use_transform {
                draw.pop_matrix();
            }
        });
    }

    // transform
    pub fn translate(mut self, pos: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_translation(pos);
        self
    }

    pub fn anchor(mut self, point: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_anchor(point);
        self
    }

    pub fn pivot(mut self, point: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_pivot(point);
        self
    }

    pub fn origin(self, point: Vec2) -> Self {
        self.anchor(point).pivot(point)
    }

    pub fn flip_x(mut self, flip: bool) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_flip(bvec2(flip, t.flip().y));
        self
    }

    pub fn flip_y(mut self, flip: bool) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_flip(bvec2(t.flip().x, flip));
        self
    }

    pub fn skew(mut self, skew: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_skew(skew);
        self
    }

    pub fn scale(mut self, scale: Vec2) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_scale(scale);
        self
    }

    pub fn rotation(mut self, rot: f32) -> Self {
        let t = self.transform2d.get_or_insert_with(Transform2D::default);
        t.set_rotation(rot);
        self
    }
}
fn draw_node<T>(
    node_id: NodeId,
    callbacks: &mut FxHashMap<CtxId, Box<dyn CallRenderCallback>>,
    tree: &TaffyTree<()>,
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
