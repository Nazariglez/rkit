#![cfg(feature = "random")]

use fastrand::Rng as RawRng;
use std::cell::RefCell;
use std::collections::Bound;
use std::ops::RangeBounds;

thread_local! {
    static LOCAL_RNG: RefCell<Rng> = RefCell::new(Rng::new());
}

// - Random API

/// Returns the current global seed
pub fn seed() -> u64 {
    LOCAL_RNG.with(|rng| rng.borrow().seed())
}

/// Set a new seed for the global RNG
pub fn set_seed(seed: u64) {
    LOCAL_RNG.with(|rng| rng.replace(Rng::with_seed(seed)));
}

/// Generate a random value for T
/// booleans will be true|false while floats will be a number between 0 and 1
pub fn gen<T: Generator>() -> T {
    LOCAL_RNG.with(|rng| rng.borrow_mut().gen())
}

/// Generate a random value between the range passed
pub fn range<T: RangeGenerator>(range: impl RangeBounds<T>) -> T {
    LOCAL_RNG.with(|rng| rng.borrow_mut().range(range))
}

/// Sort randomly a slice
pub fn shuffle<T>(slice: &mut [T]) {
    LOCAL_RNG.with(|rng| rng.borrow_mut().shuffle(slice))
}

/// Pick a value randomly
pub fn pick<I>(iter: I) -> Option<I::Item>
where
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
{
    LOCAL_RNG.with(|rng| rng.borrow_mut().pick(iter))
}

/// Random generator
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Rng {
    raw: RawRng,
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    /// New instance
    pub fn new() -> Self {
        Self { raw: RawRng::new() }
    }

    /// New instance using a seed
    pub fn with_seed(seed: u64) -> Self {
        Self {
            raw: RawRng::with_seed(seed),
        }
    }

    /// Generate a random value for T
    /// booleans will be true|false while floats will be a number between 0 and 1
    pub fn gen<T: Generator>(&mut self) -> T {
        T::gen(self)
    }

    /// Generate a random value between the range passed
    pub fn range<T: RangeGenerator>(&mut self, range: impl RangeBounds<T>) -> T {
        T::range(self, range)
    }

    /// Returns the current seed
    pub fn seed(&self) -> u64 {
        self.raw.get_seed()
    }

    /// Sort randomly a slice
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        self.raw.shuffle(slice)
    }

    /// Pick a value randomly
    pub fn pick<I>(&mut self, iter: I) -> Option<I::Item>
    where
        I: IntoIterator,
        I::IntoIter: ExactSizeIterator,
    {
        self.raw.choice(iter)
    }
}

pub trait Generator {
    fn gen(rng: &mut Rng) -> Self;
}

macro_rules! impl_generator {
    ($($t:ty, $method:ident),*) => {
        $(
            impl Generator for $t {
                fn gen(rng: &mut Rng) -> Self {
                    rng.raw.$method()
                }
            }
        )*
    };
}

impl_generator!(f32, f32, f64, f64, bool, bool);

pub trait RangeGenerator {
    fn range(rng: &mut Rng, range: impl RangeBounds<Self>) -> Self;
}

macro_rules! impl_range_generator {
    ($($t:ty, $method:ident),*) => {
        $(
            impl RangeGenerator for $t {
                fn range(rng: &mut Rng, range: impl RangeBounds<Self>) -> Self {
                    rng.raw.$method(range)
                }
            }
        )*
    };
}

// Usage example
impl_range_generator!(
    char, char, i8, i8, i16, i16, i32, i32, i64, i64, i128, i128, isize, isize, u8, u8, u16, u16,
    u32, u32, u64, u64, u128, u128, usize, usize
);

impl RangeGenerator for f32 {
    fn range(rng: &mut Rng, range: impl RangeBounds<Self>) -> Self {
        let min = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => 0.0,
        };
        let max = match range.end_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => 1.0,
        };
        min + rng.raw.f32() * max
    }
}

impl RangeGenerator for f64 {
    fn range(rng: &mut Rng, range: impl RangeBounds<Self>) -> Self {
        let min = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => 0.0,
        };
        let max = match range.end_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => 1.0,
        };
        min + rng.raw.f64() * max
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ops::Range;

    #[test]
    fn test_with_seed() {
        let seed = 42;
        let mut rng1 = Rng::with_seed(seed);
        let mut rng2 = Rng::with_seed(seed);

        assert_eq!(rng1.gen::<f32>(), rng2.gen::<f32>());
        assert_eq!(rng1.gen::<f64>(), rng2.gen::<f64>());
        assert_eq!(rng1.gen::<bool>(), rng2.gen::<bool>());
    }

    macro_rules! test_range {
        ($($rng:expr, $t:ty, $range:expr),*) => {
            $(
               let range: Range<$t> = $range;
                let number = $rng.range(range.clone());
                assert!(range.contains(&number), "Fail testing rng.range with type {}", std::any::type_name::<$t>());
            )*
        };
    }

    #[test]
    fn test_gen_range() {
        let mut rng = Rng::new();
        test_range!(rng, i8, 10..20);
        test_range!(rng, i16, 10..20);
        test_range!(rng, i32, 10..20);
        test_range!(rng, i64, 10..20);
        test_range!(rng, i128, 10..20);
        test_range!(rng, isize, 10..20);
        test_range!(rng, u8, 10..20);
        test_range!(rng, u16, 10..20);
        test_range!(rng, u32, 10..20);
        test_range!(rng, u64, 10..20);
        test_range!(rng, u128, 10..20);
        test_range!(rng, usize, 10..20);
    }

    #[test]
    fn test_shuffle() {
        let mut rng = Rng::new();
        let mut data = [1, 2, 3, 4, 5];
        rng.shuffle(&mut data);
        assert_ne!(data, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_pick() {
        let mut rng = Rng::new();
        let data = [10, 20, 30, 40, 50];
        let picked = rng.pick(data.iter());
        assert!(picked.is_some());
        assert!(data.contains(picked.unwrap()));
    }
}
