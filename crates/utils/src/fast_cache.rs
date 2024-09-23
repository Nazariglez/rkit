use hashbrown::hash_map::{Iter, IterMut};
use hashbrown::HashMap;
use rustc_hash::FxHasher;
use std::hash::{BuildHasherDefault, Hash};

type Cache<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

pub struct FastCache<K, V>
where
    K: Hash + Eq,
{
    inner: Cache<K, V>,
}

impl<K, V> Clone for FastCache<K, V>
where
    K: Hash + PartialEq + Eq + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> Default for FastCache<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self {
            inner: HashMap::with_hasher(Default::default()),
        }
    }
}

impl<K, V> FastCache<K, V>
where
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn contains_key(&self, k: &K) -> bool {
        self.inner.contains_key(k)
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.inner.insert(k, v)
    }

    #[inline]
    pub fn get(&self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    // Implementing iter for immutable iteration
    #[inline]
    pub fn iter(&self) -> Iter<K, V> {
        self.inner.iter()
    }

    // Implementing iter_mut for mutable iteration
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        self.inner.iter_mut()
    }
}
