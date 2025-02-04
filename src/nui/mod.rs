pub mod layout;
pub mod node;
pub mod prelude;
pub mod style;

use std::cell::RefCell;

use bumpalo::Bump;
use corelib::math::Vec2;
use draw::Draw2D;
use node::{Node, NuiWidget};
use rustc_hash::FxHashMap;
use style::{taffy_style_from, Style};
use taffy::{Layout, NodeId, TaffyTree};

thread_local! {
    pub(super) static CACHE: RefCell<NuiCache> = {
        corelib::app::on_sys_pre_update(|| {
            CACHE.with_borrow_mut(|cache| {
                cache.cache_id = 0;
            });
        });
        RefCell::new(NuiCache::default())
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum CacheId {
    Anonymous(u64),
    Named(&'static str),
}

#[derive(Default)]
struct NuiCache {
    cache_id: u64,
    layouts: FxHashMap<CacheId, (Vec<Style>, TaffyTree<()>)>,
}

impl NuiCache {
    pub fn gen_id(&mut self) -> CacheId {
        self.cache_id += 1;
        CacheId::Anonymous(self.cache_id)
    }

    pub fn is_cache_valid(&self, layout: CacheId, styles: &[Style]) -> bool {
        self.layouts
            .get(&layout)
            .is_some_and(|(s, _)| s.as_slice() == styles)
    }

    pub fn add_cache(&mut self, layout: CacheId, styles: Vec<Style>, tree: TaffyTree<()>) {
        self.layouts.insert(layout, (styles, tree));
    }

    pub fn reset(&mut self) {
        self.layouts.clear();
    }
}

pub fn clean_ui_layout_cache() {
    CACHE.with_borrow_mut(|cache| cache.reset());
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum CtxId {
    Temp(u64),
    Node(NodeId),
}

type DrawCb<'data, T> = dyn for<'draw> FnMut(&'draw mut Draw2D, Layout, &T) + 'data;

pub struct NuiContext<'data, 'arena, T>
where
    'data: 'arena,
    T: 'data,
{
    temp_id: u64,
    arena: &'arena Bump,
    data: &'data T,
    callbacks: FxHashMap<CtxId, &'arena mut DrawCb<'data, T>>,
    cached_styles: Vec<Style>,
    node_stack: Vec<NodeId>,
    tree: TaffyTree<()>,
    size: Vec2,
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

    fn on_draw<F: for<'draw> FnMut(&'draw mut Draw2D, Layout, &T) + 'data>(
        &mut self,
        temp_id: u64,
        cb: F,
    ) {
        let oref = self.arena.alloc(cb);
        self.callbacks.insert(CtxId::Temp(temp_id), oref);
    }

    fn add_node_with<F: FnOnce(&mut Self)>(&mut self, node: Node<'_, 'data, 'arena, T>, cb: F) {
        let node_id = self.add_node(node);
        self.node_stack.push(node_id);
        cb(self);
        self.node_stack.pop();
    }

    #[inline]
    fn add_node<'ctx>(&'ctx mut self, node: Node<'ctx, 'data, 'arena, T>) -> NodeId {
        self.insert_node(node)
    }

    fn insert_node<'ctx>(&'ctx mut self, mut node: Node<'ctx, 'data, 'arena, T>) -> NodeId {
        let style = node.style;

        let temp_id = CtxId::Temp(node.temp_id);
        let node_id = self.tree.new_leaf(taffy_style_from(&style.layout)).unwrap();
        match self.callbacks.entry(CtxId::Temp(node.temp_id)) {
            std::collections::hash_map::Entry::Occupied(e) => {
                let val = e.remove();
                self.callbacks.insert(CtxId::Node(node_id), val);
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                // TODO: this
            }
        };

        self.cached_styles.push(node.style);
        self.tree
            .add_child(*self.node_stack.last().unwrap(), node_id)
            .unwrap();

        node_id
    }

    #[inline]
    fn add_widget<W>(&mut self, widget: W)
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
}
