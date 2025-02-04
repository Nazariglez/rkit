use corelib::math::{bvec2, vec2, Rect, Vec2};
use corelib::time;
use draw::{Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use taffy::prelude::length;
use taffy::{AvailableSpace, Layout, NodeId, Size, Style, TaffyTree};

use crate::nui::CtxId;

use super::{CacheId, DrawCb, NuiContext, CACHE};

pub trait Draw2DUiExt {
    fn ui(&mut self) -> NuiLayout<()>;
    fn ui_with<'draw, 'data, T>(&'draw mut self, data: &'data T) -> NuiLayout<'data, 'draw, T>
    where
        'data: 'draw;
}

impl Draw2DUiExt for Draw2D {
    fn ui(&mut self) -> NuiLayout<()> {
        self.ui_with(&())
    }

    fn ui_with<'draw, 'data, T>(&'draw mut self, data: &'data T) -> NuiLayout<'data, 'draw, T>
    where
        'data: 'draw,
    {
        NuiLayout::new(self, data)
    }
}

pub struct NuiLayout<'data, 'draw, T: 'data> {
    id: CacheId,
    draw: &'draw mut Draw2D,
    data: &'data T,
    size: Option<Vec2>,

    // reuse last frame layout if available
    use_cache: bool,

    // skip drawing nodes outside their parents
    use_culling: bool,

    transform2d: Option<Transform2D>,
    // mouse_info: MouseInfo,
}

impl<'data, 'draw, T> NuiLayout<'data, 'draw, T>
where
    T: 'data,
{
    fn new(draw: &'draw mut Draw2D, data: &'data T) -> Self {
        let id = CACHE.with_borrow_mut(|cache| cache.gen_id());
        Self {
            id,
            draw,
            data,
            size: None,
            use_cache: true,
            use_culling: true,
            transform2d: None,
        }
    }

    pub fn id(mut self, id: &'static str) -> Self {
        self.id = CacheId::Named(id);
        self
    }

    pub fn disable_cache(mut self) -> Self {
        self.use_cache = false;
        self
    }

    pub fn disable_culling(mut self) -> Self {
        self.use_culling = false;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn show<F>(self, cb: F)
    where
        F: FnOnce(&mut NuiContext<'data, T>),
    {
        let NuiLayout {
            id: layout_id,
            use_cache,
            use_culling,
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

        let now = time::now();
        cb(&mut ctx);
        // println!("define layout {:?}", now.elapsed());

        let NuiContext {
            mut tree,
            mut nodes,
            cache_styles,
            ..
        } = ctx;

        CACHE.with_borrow_mut(|cache| {
            let now = time::now();
            let skip_layout_compute = use_cache && cache.is_cache_valid(layout_id, &cache_styles);
            if !skip_layout_compute {
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

            let now = time::now();
            let use_transform = transform2d.is_some();
            if let Some(mut transform) = transform2d {
                transform.set_size(size);
                draw.push_matrix(transform.updated_mat3());
            }

            let root_bounds = Rect::new(Vec2::ZERO, size);
            let tree = cache.layouts.get(&layout_id).as_ref().map(|(_, tree)| tree);
            if let Some(tree) = tree {
                draw_node(
                    root_id,
                    &mut nodes,
                    tree,
                    draw,
                    data,
                    use_culling,
                    root_bounds,
                );
            }

            if use_transform {
                draw.pop_matrix();
            }
            // println!("render layout {:?}", now.elapsed());
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
    callbacks: &mut FxHashMap<CtxId, Box<DrawCb<T>>>,
    tree: &TaffyTree<()>,
    draw: &mut Draw2D,
    data: &T,
    use_culling: bool,
    parent_bounds: Rect,
) {
    let layout = tree.layout(node_id).unwrap();

    let bounds = Rect::new(
        vec2(layout.location.x, layout.location.y) + parent_bounds.origin,
        vec2(layout.size.width, layout.size.height),
    );

    let skip_draw = use_culling && !parent_bounds.intersects(&bounds);
    if skip_draw {
        return;
    }

    draw.push_matrix(
        Transform2DBuilder::default()
            .set_translation(vec2(layout.location.x, layout.location.y))
            .build()
            .as_mat3(),
    );

    if let Some(cbs) = callbacks.get_mut(&CtxId::Node(node_id)) {
        cbs(draw, *layout, data);
    }

    for child_id in tree.children(node_id).unwrap() {
        draw_node(child_id, callbacks, tree, draw, data, use_culling, bounds);
    }

    draw.pop_matrix();
}
