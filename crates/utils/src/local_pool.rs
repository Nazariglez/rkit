use arrayvec::ArrayVec;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A thread-local object pool for managing reusable items without locks.
///
/// The `LocalPool` is designed to manage objects in a thread-local context, providing automatic
/// reattachment to the pool when the `LocalPoolObserver` goes out of scope. This allows for efficient
/// reuse of objects without requiring heap allocation each time.
///
/// **Important**: The `LocalPool` struct is intended to be used with the provided macro `init_local_pool!`.
/// The macro generates the necessary thread-local instances and associated functions for managing the pool.
/// Direct instantiation of `LocalPool` is discouraged to avoid issues with manual management of thread-local storage.
///
/// # Example
///
/// ```rust,ignore
/// // Initialize the pool using the macro (example usage)
/// init_local_pool!(MY_POOL, 32, Vec<u8>, || Vec::with_capacity(100));
///
/// // Take an item from the pool
/// if let Some(mut item) = MY_POOL.take() {
///     item.push(42);
///     println!("Item: {:?}", *item);
/// }
///
/// // Check the length of the pool after taking an item
/// println!("Available items in pool: {}", MY_POOL.len());
/// ```
pub struct LocalPool<T, const N: usize> {
    _t: PhantomData<[T; N]>,
    on_take: fn() -> Option<LocalPoolObserver<T>>,
    on_drop: fn(T),
    len_fn: fn() -> usize,
}

impl<T, const N: usize> LocalPool<T, N> {
    #[doc(hidden)]
    pub const fn init(
        on_take: fn() -> Option<LocalPoolObserver<T>>,
        on_drop: fn(T),
        len_fn: fn() -> usize,
    ) -> Self {
        Self {
            _t: std::marker::PhantomData,
            on_take,
            on_drop,
            len_fn,
        }
    }
    pub fn take(&self) -> Option<LocalPoolObserver<T>> {
        (self.on_take)()
    }

    pub fn len(&self) -> usize {
        (self.len_fn)()
    }
}

pub struct LocalPoolObserver<T> {
    inner: Option<T>,
    on_drop: fn(T),
}

impl<T> LocalPoolObserver<T> {
    pub fn new(inner: T, on_drop: fn(T)) -> Self {
        Self {
            inner: Some(inner),
            on_drop,
        }
    }
}

impl<T> Deref for LocalPoolObserver<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T> DerefMut for LocalPoolObserver<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl<T> Drop for LocalPoolObserver<T> {
    fn drop(&mut self) {
        debug_assert!(self.inner.is_some(), "Pool object dropped twice?");
        if let Some(inner) = self.inner.take() {
            (self.on_drop)(inner);
        }
    }
}

pub struct InnerLocalPool<T, const N: usize> {
    pool: ArrayVec<T, N>,
}

impl<T, const N: usize> InnerLocalPool<T, N> {
    pub fn new<F>(initializer: F) -> Self
    where
        F: Fn() -> T,
    {
        let mut pool = ArrayVec::new();
        for _ in 0..N {
            pool.push(initializer());
        }
        Self { pool }
    }

    pub fn take(&mut self) -> Option<T> {
        self.pool.pop()
    }

    pub fn put_back(&mut self, item: T) {
        self.pool.push(item);
    }

    pub fn len(&self) -> usize {
        self.pool.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macros::init_local_pool;

    // Use the macro to initialize the pools used in the tests
    init_local_pool!(POOL_A, 4, Vec<u8>, || Vec::with_capacity(10));
    init_local_pool!(POOL_B, 3, String, || String::new());

    #[test]
    fn test_pool_a_initial_length() {
        assert_eq!(POOL_A.len(), 4);
    }

    #[test]
    fn test_take_and_return_pool_a() {
        // Take an item from POOL_A
        if let Some(mut item) = POOL_A.take() {
            item.push(42);
            assert_eq!(item.capacity(), 10);

            // After taking an item, the pool should have one less
            assert_eq!(POOL_A.len(), 3);
        } else {
            panic!("Expected an available item in POOL_A");
        }

        // Item should be returned after drop
        assert_eq!(POOL_A.len(), 4);
    }

    #[test]
    fn test_take_all_items_from_pool_b() {
        // Take all items from POOL_B
        let mut observers = Vec::new();
        for _ in 0..3 {
            let item = POOL_B.take().expect("Expected an available item in POOL_B");
            observers.push(item);
        }

        // All items are taken, POOL_B should be empty
        assert_eq!(POOL_B.len(), 0);

        // Taking another item should return None
        assert!(POOL_B.take().is_none());
    }

    #[test]
    fn test_return_all_items_to_pool_b() {
        {
            // Take all items from POOL_B
            let mut observers = Vec::new();
            for _ in 0..3 {
                let item = POOL_B.take().expect("Expected an available item in POOL_B");
                observers.push(item);
            }
        }

        // After dropping all items, the pool should be full again
        assert_eq!(POOL_B.len(), 3);
    }
}
