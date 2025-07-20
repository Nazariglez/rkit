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
pub const fn next_pot2(x: usize) -> usize {
    if x == 0 {
        return 1;
    }

    let mut n = x;
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;

    // Only perform this shift if we're on a 64-bit platform, if not this will overflow (as in wasm32)
    #[cfg(target_pointer_width = "64")]
    {
        n |= n >> 32;
    }

    n + 1
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
}
