pub mod ctx;
pub mod layout;
pub mod node;
pub mod prelude;
pub mod style;

use std::cell::RefCell;

use rustc_hash::FxHashMap;
use style::Style;
use taffy::TaffyTree;

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

/// Clean the UI Layout cache between used between frames.
/// Cleaning the cache will force the UI to compute a new layout again.
/// This is usefull if you're done with complex UIs and want to clean or reduce memory usage.
pub fn clean_ui_layout_cache() {
    CACHE.with_borrow_mut(|cache| cache.reset());
}
