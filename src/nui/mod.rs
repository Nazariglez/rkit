pub mod ctx;
pub mod layout;
pub mod node;
pub mod prelude;
pub mod style;

use std::cell::RefCell;

use ctx::NodeContext;
use rustc_hash::FxHashMap;
use style::Style;
use taffy::TaffyTree;

thread_local! {
    pub(super) static CACHE: RefCell<NuiCache> = {
        corelib::app::on_sys_post_update(|| {
            CACHE.with_borrow_mut(|cache| {
                cache.update();
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

/// Reuse caches between frames is there is no changes
struct NuiCache {
    /// Temporal layout id to identify non-named layouts
    cache_id: u64,
    /// Cache data
    layouts: FxHashMap<CacheId, (Vec<Style>, TaffyTree<NodeContext>)>,
    /// Auto clean the cache after N frames
    auto_reset: Option<usize>,
    /// Count the number of frames to reset the cache
    auto_reset_count: usize,
}

impl Default for NuiCache {
    fn default() -> Self {
        Self {
            cache_id: 0,
            layouts: FxHashMap::default(),
            auto_reset: Some(100_000),
            auto_reset_count: 0,
        }
    }
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

    pub fn add_cache(&mut self, layout: CacheId, styles: Vec<Style>, tree: TaffyTree<NodeContext>) {
        self.layouts.insert(layout, (styles, tree));
    }

    pub fn reset(&mut self) {
        self.layouts.clear();
    }

    fn update(&mut self) {
        // reset the anonymous id for the next frame
        self.cache_id = 0;

        // if auto reset is set then check if it needs to clean
        let Some(frames) = self.auto_reset else {
            return;
        };

        self.auto_reset_count += 1;
        if self.auto_reset_count >= frames {
            self.reset();
            self.auto_reset_count = 0;
            log::debug!("Cleaned UI Cache after {frames} frames");
        }
    }
}

/// Clean the UI Layout cache between used between frames.
/// Cleaning the cache will force the UI to compute a new layout again.
/// This is usefull if you're done with complex UIs and want to clean or reduce memory usage.
pub fn clean_ui_layout_cache() {
    CACHE.with_borrow_mut(|cache| cache.reset());
}

/// Set a number of frames after the cache will be clean/reset
/// Setting to none will disable the auto-reset feature
pub fn frames_to_clean_ui_cache(n: Option<usize>) {
    CACHE.with_borrow_mut(|cache| {
        cache.auto_reset = n;
    });
}
