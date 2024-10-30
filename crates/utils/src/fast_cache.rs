use lru::{Iter, IterMut, LruCache};
use rustc_hash::FxHasher;
use std::hash::{BuildHasherDefault, Hash};
use std::num::NonZeroUsize;

type Cache<K, V> = LruCache<K, V, BuildHasherDefault<FxHasher>>;

pub struct FastCache<K, V>
where
    K: Hash + Eq,
{
    inner: Cache<K, V>,
}

impl<K, V> FastCache<K, V>
where
    K: Hash + Eq,
{
    pub fn new(size: NonZeroUsize) -> Self {
        Self {
            inner: LruCache::with_hasher(size, Default::default()),
        }
    }

    #[inline]
    pub fn contains_key(&self, k: &K) -> bool {
        self.inner.contains(k)
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.inner.put(k, v)
    }

    #[inline]
    pub fn get(&mut self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    #[inline]
    pub fn get_or_insert<F: FnOnce() -> V>(&mut self, k: K, cb: F) -> &V {
        self.inner.get_or_insert(k, cb)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn iter(&self) -> Iter<K, V> {
        self.inner.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        self.inner.iter_mut()
    }
}
