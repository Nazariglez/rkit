/// Returns the User's data directory depending on the enviroment
/// The `web` build will return the basedir
#[inline(always)]
pub fn user_data_path(base: &str) -> Option<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        platform_dirs::AppDirs::new(Some(base), false).map(|d| d.data_dir)
    }

    #[cfg(target_arch = "wasm32")]
    {
        Some(std::path::Path::new(base).to_owned())
    }
}

/// Returns the next power of two number
#[inline(always)]
#[deprecated = "Just use 'n.next_power_of_two()' from the std"]
pub const fn next_pot2(x: usize) -> usize {
    x.next_power_of_two()
}

#[inline(always)]
pub const fn next_multiple_of(num: usize, base: usize) -> usize {
    debug_assert!(base > 0, "base must be > 0");
    match num.checked_next_multiple_of(base) {
        Some(v) => v,
        None => usize::MAX,
    }
}

#[inline(always)]
pub const fn closest_multiple_of(num: usize, base: usize) -> usize {
    debug_assert!(base > 0, "base must be > 0");

    let rem = num % base;
    if rem == 0 {
        return num;
    }

    let down = num - rem;
    let up = match down.checked_add(base) {
        Some(v) => v,
        None => return down,
    };

    if rem >= base - rem {
        return up;
    }

    down
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_next_multiply() {
        assert_eq!(next_multiple_of(144, 2), 146);

        assert_eq!(next_multiple_of(7, 8), 8);
        assert_eq!(next_multiple_of(8, 8), 8);
        assert_eq!(next_multiple_of(9, 8), 16);
        assert_eq!(next_multiple_of(15, 8), 16);
        assert_eq!(next_multiple_of(16, 8), 16);
        assert_eq!(next_multiple_of(17, 8), 24);

        assert_eq!(next_multiple_of(0, 10), 10);
        assert_eq!(next_multiple_of(9, 10), 10);
        assert_eq!(next_multiple_of(10, 10), 10);
        assert_eq!(next_multiple_of(13, 10), 20);
        assert_eq!(next_multiple_of(21, 10), 40);

        assert_eq!(next_multiple_of(8, 9), 9);
        assert_eq!(next_multiple_of(9, 9), 9);
        assert_eq!(next_multiple_of(10, 9), 18);
        assert_eq!(next_multiple_of(17, 9), 18);
        assert_eq!(next_multiple_of(19, 9), 36);
    }

    #[test]
    fn test_multiply_overflow_to_max() {
        let out = next_multiple_of(usize::MAX - 1, 3);
        assert_eq!(out, usize::MAX);
    }

    #[test]
    fn test_closest_multiply() {
        assert_eq!(closest_multiple_of(7, 8), 8);
        assert_eq!(closest_multiple_of(8, 8), 8);
        assert_eq!(closest_multiple_of(9, 8), 8);
        assert_eq!(closest_multiple_of(11, 8), 8);
        assert_eq!(closest_multiple_of(12, 8), 16);
        assert_eq!(closest_multiple_of(13, 8), 16);
        assert_eq!(closest_multiple_of(15, 8), 16);
        assert_eq!(closest_multiple_of(16, 8), 16);
        assert_eq!(closest_multiple_of(17, 8), 16);

        assert_eq!(closest_multiple_of(0, 10), 0);
        assert_eq!(closest_multiple_of(4, 10), 0);
        assert_eq!(closest_multiple_of(5, 10), 10);
        assert_eq!(closest_multiple_of(9, 10), 10);
        assert_eq!(closest_multiple_of(10, 10), 10);
        assert_eq!(closest_multiple_of(13, 10), 10);
        assert_eq!(closest_multiple_of(15, 10), 20);
        assert_eq!(closest_multiple_of(21, 10), 20);

        assert_eq!(closest_multiple_of(8, 9), 9);
        assert_eq!(closest_multiple_of(9, 9), 9);
        assert_eq!(closest_multiple_of(10, 9), 9);
        assert_eq!(closest_multiple_of(13, 9), 9);
        assert_eq!(closest_multiple_of(14, 9), 18);
        assert_eq!(closest_multiple_of(17, 9), 18);
        assert_eq!(closest_multiple_of(19, 9), 18);
    }

    #[test]
    fn test_closest_multiply_overflow() {
        let out = closest_multiple_of(usize::MAX - 1, 2);
        assert_eq!(out, usize::MAX - 1);
    }
}
