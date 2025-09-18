/// Returns the User's data directory depending on the enviroment
/// The `web` build will return None
#[inline(always)]
pub fn user_data_path(base: &str) -> Option<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        platform_dirs::AppDirs::new(Some(base), false).map(|d| d.data_dir)
    }

    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

/// Returns the next power of two number
#[inline(always)]
#[deprecated = "Just use 'n.next_power_of_two()' from the std"]
pub const fn next_pot2(x: usize) -> usize {
    x.next_power_of_two()
}

#[inline(always)]
pub const fn next_multiply_of(num: usize, base: usize) -> usize {
    assert!(base > 0, "base must be > 0");
    let units = if num == 0 { 1 } else { num.div_ceil(base) };
    match units.checked_next_power_of_two() {
        Some(p2) => p2.saturating_mul(base),
        None => usize::MAX,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_next_pot2() {
        assert_eq!(next_pot2(0), 1);
        assert_eq!(next_pot2(1), 1);
        assert_eq!(next_pot2(2), 2);
        assert_eq!(next_pot2(3), 4);
        assert_eq!(next_pot2(5), 8);
        assert_eq!(next_pot2(15), 16);
        assert_eq!(next_pot2(16), 16);
        assert_eq!(next_pot2(17), 32);
        assert_eq!(next_pot2(1000), 1024);
    }

    #[test]
    fn next_multiply() {
        assert_eq!(next_multiply_of(7, 8), 8);
        assert_eq!(next_multiply_of(8, 8), 8);
        assert_eq!(next_multiply_of(9, 8), 16);
        assert_eq!(next_multiply_of(15, 8), 16);
        assert_eq!(next_multiply_of(16, 8), 16);
        assert_eq!(next_multiply_of(17, 8), 32);

        assert_eq!(next_multiply_of(0, 10), 10);
        assert_eq!(next_multiply_of(9, 10), 10);
        assert_eq!(next_multiply_of(10, 10), 10);
        assert_eq!(next_multiply_of(13, 10), 20);
        assert_eq!(next_multiply_of(21, 10), 40);

        assert_eq!(next_multiply_of(8, 9), 9);
        assert_eq!(next_multiply_of(9, 9), 9);
        assert_eq!(next_multiply_of(10, 9), 18);
        assert_eq!(next_multiply_of(17, 9), 18);
        assert_eq!(next_multiply_of(19, 9), 36);
    }

    #[test]
    fn test_multiply_overflow_to_max() {
        let out = next_multiply_of(usize::MAX - 1, 2);
        assert_eq!(out, usize::MAX);
    }
}
