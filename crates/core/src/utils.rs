use heapless::{IndexMap, IndexSet};
use nohash_hasher::BuildNoHashHasher;

pub(crate) type EnumSet<T, const N: usize> = IndexSet<T, BuildNoHashHasher<T>, N>;
pub(crate) type EnumMap<K, V, const N: usize> = IndexMap<K, V, BuildNoHashHasher<K>, N>;

pub(crate) const fn next_pot2(x: usize) -> usize {
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
    if size_of::<usize>() > 4 {
        n |= n >> 32;
    }
    n + 1
}

// use values from environment or set a default
#[macro_export]
macro_rules! option_usize_env {
    ($s:expr, $d:expr) => {
        $crate::utils::parse_string_as_usize(option_env!($s), $d)
    };
}

pub const fn parse_string_as_usize(key: Option<&'static str>, default: usize) -> usize {
    match key {
        None => default, // Default value
        Some(num) => {
            if num.is_empty() {
                return default;
            }
            // str.parse::<usize>() is not a const fn yet
            // this trick will do it for now:
            // https://www.reddit.com/r/rust/comments/10ol38k/comment/j6fbjwj/?utm_source=reddit&utm_medium=web2x&context=3
            let mut res: usize = 0;
            let mut bytes = num.as_bytes();
            while let [byte, rest @ ..] = bytes {
                bytes = rest;
                if let b'0'..=b'9' = byte {
                    res *= 10;
                    res += (*byte - b'0') as usize;
                } else {
                    return default;
                }
            }
            res
        }
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
    fn test_parse_string_as_usize() {
        assert_eq!(parse_string_as_usize(Some("123"), 10), 123);
        assert_eq!(parse_string_as_usize(Some("0"), 10), 0);
        assert_eq!(parse_string_as_usize(Some("invalid"), 10), 10);
        assert_eq!(parse_string_as_usize(None, 10), 10);
        assert_eq!(parse_string_as_usize(Some(""), 10), 10);
        assert_eq!(parse_string_as_usize(Some("123abc"), 10), 10);
    }

    #[test]
    #[should_panic]
    fn test_parse_string_as_usize_overflow() {
        parse_string_as_usize(Some("99999999999999999999999999"), 0);
    }

    #[test]
    fn test_option_usize_env() {
        // Assuming there's no environment variable set, it should use the default.
        assert_eq!(option_usize_env!("NON_EXISTENT_ENV_VAR", 10), 10);
        // EXISTING_ENV_VAR is set on build.rs for test builds
        assert_eq!(option_usize_env!("EXISTING_ENV_VAR", 10), 123);
    }

    #[test]
    #[should_panic]
    fn test_option_usize_env_overflow() {
        option_usize_env!("OVERFLOW_ENV_VAR", 0);
    }
}
