use bumpalo::Bump;
use corelib::math::{bvec2, vec2, Rect, Vec2};
use draw::{Draw2D, Transform2D, Transform2DBuilder};
use rustc_hash::FxHashMap;
use taffy::prelude::length;
use taffy::{AvailableSpace, NodeId, Size, Style, TaffyTree, TraversePartialTree};

use crate::nui::ctx::{CtxId, NuiContext, OnDrawCb};

use super::node::{NodeInput, NodeState};
use super::{CacheId, NodeContext, CACHE};

pub trait Draw2DUiExt {
    fn ui<'draw, 'data, T>(&'draw mut self, data: &'data mut T) -> NuiLayout<'data, 'draw, T>
    where
        'data: 'draw;
}

impl Draw2DUiExt for Draw2D {
    fn ui<'draw, 'data, T>(&'draw mut self, data: &'data mut T) -> NuiLayout<'data, 'draw, T>
    where
        'data: 'draw,
    {
        NuiLayout::new(self, data)
    }
}

pub struct NuiLayout<'data, 'draw, T: 'data> {
    id: CacheId,
    draw: &'draw mut Draw2D,
    data: &'data mut T,
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
    fn new(draw: &'draw mut Draw2D, data: &'data mut T) -> Self {
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
        F: FnOnce(&mut NuiContext<'data, '_, T>),
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

        let mut tree = TaffyTree::new();
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

        let arena = Bump::new();
        let mut ctx = NuiContext {
            temp_id: 0,
            arena: &arena,
            data,
            callbacks: FxHashMap::default(),
            cached_styles: vec![],
            node_stack: vec![root_id],
            tree,
            size,
        };

        cb(&mut ctx);

        let NuiContext {
            mut tree,
            mut callbacks,
            cached_styles,
            data,
            ..
        } = ctx;

        CACHE.with_borrow_mut(|cache| {
            let skip_layout_compute = use_cache && cache.is_cache_valid(layout_id, &cached_styles);
            if !skip_layout_compute {
                tree.compute_layout(
                    root_id,
                    Size {
                        width: AvailableSpace::Definite(size.x),
                        height: AvailableSpace::Definite(size.y),
                    },
                )
                .unwrap();
                cache.add_cache(layout_id, cached_styles, tree);
            }

            let use_transform = transform2d.is_some();
            if let Some(mut transform) = transform2d {
                transform.set_size(size);
                draw.push_matrix(transform.updated_mat3());
            }

            let root_bounds = Rect::new(Vec2::ZERO, size);
            let mut cached_layout = cache.layouts.get_mut(&layout_id);
            let tree = cached_layout.as_mut().map(|(_, tree)| tree);
            if let Some(tree) = tree {
                draw_node(
                    root_id,
                    &mut callbacks,
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

enum NodeTree {
    Root(NodeId),
    Node { parent: NodeId, node_id: NodeId },
    StartChildrenOf(NodeId),
    EndChildrenOf(NodeId),
}

fn generate_node_tree(root: NodeId, tree: &TaffyTree<NodeContext>) -> Vec<NodeTree> {
    let mut list = Vec::with_capacity(tree.total_node_count());
    recursive_node(None, root, tree, &mut list);
    list
}

fn recursive_node(
    parent: Option<NodeId>,
    node_id: NodeId,
    tree: &TaffyTree<NodeContext>,
    list: &mut Vec<NodeTree>,
) {
    let id = match parent {
        Some(parent) => NodeTree::Node { parent, node_id },
        None => NodeTree::Root(node_id),
    };

    list.push(id);

    if tree.child_count(node_id) == 0 {
        return;
    }

    list.push(NodeTree::StartChildrenOf(node_id));

    tree.child_ids(node_id).for_each(|child_id| {
        recursive_node(Some(node_id), child_id, tree, list);
    });

    list.push(NodeTree::EndChildrenOf(node_id));
}

fn process_nodes<T>(
    root_id: NodeId,
    callbacks: &mut FxHashMap<CtxId, &mut OnDrawCb<T>>,
    tree: &mut TaffyTree<NodeContext>,
    draw: &mut Draw2D,
    data: &mut T,
    use_culling: bool,
    parent_bounds: Rect,
) {
    let nodes = generate_node_tree(root_id, tree);

    nodes.iter().for_each(|nt| {
        let (id, bounds, layout, skip) = match nt {
            NodeTree::Root(node_id) => {
                let layout = tree.layout(*node_id).unwrap();
                let bounds = Rect::new(
                    vec2(layout.location.x, layout.location.y),
                    vec2(layout.size.width, layout.size.height),
                );
                (*node_id, bounds, *layout)
            }
            NodeTree::Node { parent, node_id } => {
                let parent_layout = tree.layout(*parent).unwrap();
                let layout = tree.layout(*node_id).unwrap();
                let bounds = Rect::new(
                    vec2(
                        layout.location.x + parent_layout.location.x,
                        layout.location.y + parent_layout.location.y,
                    ),
                    vec2(layout.size.width, layout.size.height),
                );
                (*node_id, bounds, *layout)
            }
            _ => return,
        };

        let context = tree.get_node_context_mut(id).unwrap();
        context.state.size = bounds.size;
        context.state.position = bounds.origin;
        context.state.content_size = vec2(layout.content_size.width, layout.content_size.height);
    });
}

fn draw_node<T>(
    node_id: NodeId,
    callbacks: &mut FxHashMap<CtxId, &mut OnDrawCb<T>>,
    tree: &mut TaffyTree<NodeContext>,
    draw: &mut Draw2D,
    data: &mut T,
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
