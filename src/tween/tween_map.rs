use crate::tween::{ApplyState, Interpolable, Tween};
use rustc_hash::FxHashMap;

pub struct TweenMap<K, V>
where
    K: std::hash::Hash + Eq,
    V: Interpolable,
{
    inner: FxHashMap<K, Tween<V>>,
}

impl<K, V> Default for TweenMap<K, V>
where
    K: std::hash::Hash + Eq,
    V: Interpolable,
{
    fn default() -> Self {
        Self {
            inner: FxHashMap::default(),
        }
    }
}

impl<K, V> TweenMap<K, V>
where
    K: std::hash::Hash + Eq,
    V: Interpolable,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new Tween into the map
    pub fn insert(&mut self, key: K, tween: Tween<V>) -> Option<Tween<V>> {
        let tween = tween.start();
        self.inner.insert(key, tween)
    }

    /// Retrieves a mutable reference to a Tween by key
    pub fn get_mut(&mut self, key: &K) -> Option<&mut Tween<V>> {
        self.inner.get_mut(key)
    }

    /// Retrieves an immutable reference to a Tween by key
    pub fn get(&self, key: &K) -> Option<&Tween<V>> {
        self.inner.get(key)
    }

    /// Removes a Tween by key
    pub fn remove(&mut self, key: &K) -> Option<Tween<V>> {
        self.inner.remove(key)
    }

    /// Ticks all Tweens in the map
    pub fn tick_all(&mut self, delta: f32) {
        // Clean up ended tweens before ticking
        self.clean_ended();

        for tween in self.inner.values_mut() {
            tween.tick(delta);
        }
    }

    /// Applies the callback to a single Tween by key if it is started
    pub fn apply<F: FnOnce(V)>(&mut self, key: &K, cb: F) -> Option<ApplyState> {
        self.inner.get_mut(key).map(|tween| tween.apply(cb))
    }

    /// Removes all entries in the TweenMap
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Removes all ended Tweens from the map
    pub fn clean_ended(&mut self) {
        self.inner.retain(|_, tween| !tween.is_ended());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tween::{EaseFn, Tween, LINEAR};

    #[test]
    fn test_insert_and_get() {
        let mut map = TweenMap::new();
        let tween = Tween::new(0.0, 100.0, 1.0);
        map.insert("test", tween);

        assert!(map.get(&"test").is_some());
        assert_eq!(map.get(&"test").unwrap().value(), 0.0);
    }

    #[test]
    fn test_remove() {
        let mut map = TweenMap::new();
        map.insert("test", Tween::new(0.0, 100.0, 1.0));

        assert!(map.get(&"test").is_some());
        map.remove(&"test");
        assert!(map.get(&"test").is_none());
    }

    #[test]
    fn test_tick_all() {
        let mut map = TweenMap::new();
        map.insert("tween1", Tween::new(0.0, 100.0, 1.0));
        map.insert("tween2", Tween::new(50.0, 150.0, 1.0));

        map.tick_all(0.5);

        assert_eq!(map.get(&"tween1").unwrap().value(), 50.0);
        assert_eq!(map.get(&"tween2").unwrap().value(), 100.0);
    }

    #[test]
    fn test_apply() {
        let mut map = TweenMap::new();
        map.insert("test", Tween::new(0.0, 100.0, 1.0));

        let mut applied_value = 0.0;
        map.apply(&"test", |value| {
            applied_value = value;
        });

        assert_eq!(applied_value, 0.0);

        map.tick_all(1.0);
        map.apply(&"test", |value| {
            applied_value = value;
        });

        assert_eq!(applied_value, 100.0);
    }

    #[test]
    fn test_clear() {
        let mut map = TweenMap::new();
        map.insert("tween1", Tween::new(0.0, 100.0, 1.0));
        map.insert("tween2", Tween::new(50.0, 150.0, 1.0));

        assert!(map.get(&"tween1").is_some());
        assert!(map.get(&"tween2").is_some());

        map.clear();

        assert!(map.get(&"tween1").is_none());
        assert!(map.get(&"tween2").is_none());
    }

    #[test]
    fn test_clean_ended() {
        let mut map = TweenMap::new();
        map.insert("ended_tween", Tween::new(0.0, 100.0, 0.5));
        map.insert("ongoing_tween", Tween::new(0.0, 100.0, 1.0));

        map.tick_all(0.5);
        map.clean_ended();

        assert!(map.get(&"ended_tween").is_none());
        assert!(map.get(&"ongoing_tween").is_some());
    }

    #[test]
    fn test_tick_all_with_clean_ended() {
        let mut map = TweenMap::new();
        map.insert("tween1", Tween::new(0.0, 100.0, 1.0));
        map.insert("tween2", Tween::new(50.0, 150.0, 0.5)); // Ends sooner

        map.tick_all(0.5);

        assert!(map.get(&"tween1").is_some());
        // this one ends here but it's not removed until next fram
        assert!(map.get(&"tween2").is_some());

        map.tick_all(0.0);
        assert!(map.get(&"tween2").is_none());
    }
}
