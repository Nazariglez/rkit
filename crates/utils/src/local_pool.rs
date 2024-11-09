use arrayvec::ArrayVec;
pub use paste::paste;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

// This code is hell... I am sure it can be done more directly in a better way
// but I want to have auto-attach on the pool when the element is dropped
// using one pool per thread and no locks... So, this seems handy to be like this
// even if it's ugly.

pub struct LocalPool<T, const N: usize> {
    _t: PhantomData<[T; N]>,
    on_take: fn() -> Option<LocalPoolObserver<T>>,
    on_drop: fn(T),
    len_fn: fn() -> usize,
}

impl<T, const N: usize> LocalPool<T, N> {
    pub const fn new(
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

// Define a macro to initialize thread-local pools
#[macro_export]
macro_rules! init_local_pool {
    ($name:ident, $n:expr, $t:ty, $init:expr) => {
        paste! {
            thread_local! {
                static [<INNER_ $name>]: std::cell::RefCell<InnerLocalPool<$t, $n>> = std::cell::RefCell::new(InnerLocalPool::new($init));
            }

            #[allow(non_snake_case)]
            fn [<on_take_ $name>]() -> Option<LocalPoolObserver<$t>> {
                [<INNER_ $name>].with(|pool| {
                    let mut pool = pool.borrow_mut();
                    pool.take()
                        .map(|t| LocalPoolObserver::new(t, [<on_drop_ $name>]))
                })
            }

            #[allow(non_snake_case)]
            fn [<on_drop_ $name>](t: $t) {
                [<INNER_ $name>].with(|pool| {
                    pool.borrow_mut().put_back(t);
                });
            }

            #[allow(non_snake_case)]
            fn [<len_ $name>]() -> usize {
                [<INNER_ $name>].with(|pool| {
                    pool.borrow().len()
                })
            }

            pub static $name: LocalPool<$t, $n> = LocalPool::new([<on_take_ $name>], [<on_drop_ $name>], [<len_ $name>]);
        }
    }
}
