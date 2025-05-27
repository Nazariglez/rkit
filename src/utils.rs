#[doc(inline)]
pub use ::utils::drop_signal::*;
#[doc(inline)]
pub use ::utils::fast_cache::*;
#[doc(inline)]
pub use ::utils::helpers::*;
#[doc(inline)]
pub use ::utils::ring_buffer::*;
use arrayvec::ArrayVec;

pub mod local_pool {
    #[doc(inline)]
    pub use ::macros::init_local_pool;
    #[doc(inline)]
    pub use ::utils::local_pool::*;
}

use crate::random;

pub fn create_weighted_vec<T: Clone>(weights: &[(T, f32)], amount: usize) -> Vec<T> {
    // calculate the total weight
    let total: f32 = weights.iter().map(|(_, w)| *w).sum();
    let mut result = Vec::with_capacity(amount);

    if total > 0.0 {
        let mut remainders = Vec::with_capacity(weights.len());
        let mut remaining = amount;

        // calculate how many of each item to put in
        for (item, weight) in weights {
            let exact = (weight / total) * (amount as f32);
            let count = exact.floor() as usize;
            // add the whole-number part
            result.extend(std::iter::repeat_n(item.clone(), count));
            remaining -= count;
            // keep the leftover fraction for later
            remainders.push((item.clone(), exact - count as f32));
        }

        // fill the last spots with the items that had the biggest leftovers
        remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (item, _) in remainders.into_iter().take(remaining) {
            result.push(item);
        }
    }

    // randomize the vector
    random::shuffle(&mut result);
    result
}

pub fn create_const_weighted_vec<T: Clone, const N: usize>(weights: &[(T, f32)]) -> ArrayVec<T, N> {
    // sum of all weights
    let total: f32 = weights.iter().map(|(_, w)| *w).sum();
    let mut out = ArrayVec::<T, N>::new();
    let mut leftovers = ArrayVec::<(T, f32), N>::new();
    let mut remaining = N;

    if total > 0.0 {
        // for each (item, weight), take the integer share
        for (item, w) in weights {
            let exact = (w / total) * (N as f32);
            let cnt = exact.floor() as usize;
            for _ in 0..cnt {
                out.push(item.clone());
            }
            remaining = remaining.saturating_sub(cnt);
            // stash the fractional part
            leftovers.push((item.clone(), exact - cnt as f32));
        }

        // hand out the last `remaining` slots to the largest fractions
        leftovers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (item, _) in leftovers.into_iter().take(remaining) {
            out.push(item);
        }
    }

    //  shuffle in place
    random::shuffle(out.as_mut_slice());
    out
}

#[cfg(test)]
mod tests {
    use super::{create_const_weighted_vec, create_weighted_vec};

    /// count how many times 'x' appears in slice.
    fn count<T: PartialEq>(slice: &[T], x: &T) -> usize {
        slice.iter().filter(|&y| y == x).count()
    }

    #[test]
    fn dynamic_even_distribution() {
        let weights = &[("a", 1.0), ("b", 1.0)];
        let bag = create_weighted_vec(weights, 4);
        assert_eq!(bag.len(), 4);
        assert_eq!(count(&bag, &"a"), 2);
        assert_eq!(count(&bag, &"b"), 2);
    }

    #[test]
    fn dynamic_uneven_distribution() {
        let weights = &[("a", 1.0), ("b", 2.0)];
        let bag = create_weighted_vec(weights, 5);
        assert_eq!(bag.len(), 5);
        assert_eq!(count(&bag, &"a"), 2);
        assert_eq!(count(&bag, &"b"), 3);
    }

    #[test]
    fn dynamic_zero_weights_yield_empty() {
        let weights = &[("a", 0.0), ("b", 0.0)];
        let bag = create_weighted_vec(weights, 10);
        assert!(bag.is_empty());
    }

    #[test]
    fn dynamic_empty_input_yields_empty() {
        let weights: &[(&str, f32)] = &[];
        let bag = create_weighted_vec(weights, 10);
        assert!(bag.is_empty());
    }

    #[test]
    fn dynamic_zero_amount_yields_empty() {
        let weights = &[("a", 1.0), ("b", 2.0)];
        let bag = create_weighted_vec(weights, 0);
        assert!(bag.is_empty());
    }

    #[test]
    fn const_even_distribution() {
        let arr = create_const_weighted_vec::<_, 4>(&[("a", 1.0), ("b", 1.0)]);
        assert_eq!(arr.len(), 4);
        assert_eq!(count(&arr, &"a"), 2);
        assert_eq!(count(&arr, &"b"), 2);
    }

    #[test]
    fn const_uneven_distribution() {
        let arr = create_const_weighted_vec::<_, 5>(&[("a", 1.0), ("b", 2.0)]);
        assert_eq!(arr.len(), 5);
        assert_eq!(count(&arr, &"a"), 2);
        assert_eq!(count(&arr, &"b"), 3);
    }

    #[test]
    fn const_zero_weights_yield_empty() {
        let arr = create_const_weighted_vec::<_, 8>(&[("a", 0.0), ("b", 0.0)]);
        assert!(arr.is_empty());
    }

    #[test]
    fn const_empty_input_yields_empty() {
        let arr = create_const_weighted_vec::<&str, 8>(&[]);
        assert!(arr.is_empty());
    }
}
