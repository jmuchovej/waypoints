//! Fractional indexing for ordered collections.
//!
//! Generates lexicographically sortable strings that support arbitrary
//! insertion between existing positions without reindexing the whole list.
//!
//! Keys use a base-36 alphabet (`0-9a-z`). The default key length is 4
//! characters, giving ~1.68 million positions per parent — more than enough
//! for any realistic landmark. When two keys are adjacent at their current
//! precision, the midpoint extends to a longer key.

const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const BASE: u64 = 36;
const DEFAULT_LEN: usize = 4;

fn digit_value(c: u8) -> u64 {
    DIGITS
        .iter()
        .position(|&d| d == c)
        .expect("invalid fractional index character") as u64
}

fn encode(mut val: u64, len: usize) -> String {
    let mut buf = vec![b'0'; len];
    for i in (0..len).rev() {
        buf[i] = DIGITS[(val % BASE) as usize];
        val /= BASE;
    }
    String::from_utf8(buf).unwrap()
}

fn decode(s: &str, len: usize) -> u64 {
    let bytes = s.as_bytes();
    let mut val: u64 = 0;
    for i in 0..len {
        let d = if i < bytes.len() {
            digit_value(bytes[i])
        } else {
            0
        };
        val = val * BASE + d;
    }
    val
}

fn pow_base(exp: usize) -> u64 {
    BASE.pow(exp as u32)
}

/// Generate `n` evenly-spaced fractional index keys.
///
/// Returns keys of 4 characters (`0-9a-z`), uniformly distributed across
/// the key space (~1.68M positions). Use this for initial seeding or full
/// reordering.
///
/// ```
/// use waypoints_domain::fractional_index::generate_n;
///
/// let keys = generate_n(3);
/// assert_eq!(keys.len(), 3);
/// assert!(keys[0] < keys[1]);
/// assert!(keys[1] < keys[2]);
/// ```
pub fn generate_n(n: usize) -> Vec<String> {
    if n == 0 {
        return vec![];
    }
    let max = pow_base(DEFAULT_LEN);
    (0..n)
        .map(|i| {
            let val = max * (i as u64 + 1) / (n as u64 + 1);
            encode(val, DEFAULT_LEN)
        })
        .collect()
}

/// Generate a single key between `a` and `b`.
///
/// - `a = None` means "before everything" (lower bound = 0)
/// - `b = None` means "after everything" (upper bound = max)
/// - Both `None` returns the midpoint of the key space
///
/// If the two keys are adjacent at their current precision, the result
/// will be one character longer.
///
/// ```
/// use waypoints_domain::fractional_index::key_between;
///
/// let mid = key_between(Some("i000"), Some("i002"));
/// assert_eq!(mid, "i001");
///
/// let before = key_between(None, Some("1000"));
/// assert!(before < "1000".to_string());
///
/// let after = key_between(Some("zzzz"), None);
/// assert!(after > "zzzz".to_string());
/// ```
pub fn key_between(a: Option<&str>, b: Option<&str>) -> String {
    let len = match (a, b) {
        (Some(a), Some(b)) => a.len().max(b.len()).max(DEFAULT_LEN),
        (Some(a), None) => a.len().max(DEFAULT_LEN),
        (None, Some(b)) => b.len().max(DEFAULT_LEN),
        (None, None) => DEFAULT_LEN,
    };

    let na = a.map(|s| decode(s, len)).unwrap_or(0);
    let nb = b.map(|s| decode(s, len)).unwrap_or_else(|| pow_base(len));

    debug_assert!(na < nb, "a ({na}) must be less than b ({nb})");

    let mid = na + (nb - na) / 2;
    if mid > na {
        return encode(mid, len);
    }

    // Adjacent at this precision — extend by one character for more room.
    let ext = len + 1;
    let na_ext = na * BASE;
    let nb_ext = nb * BASE;
    let mid_ext = na_ext + (nb_ext - na_ext) / 2;
    encode(mid_ext, ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_n_produces_sorted_keys() {
        let keys = generate_n(10);
        assert_eq!(keys.len(), 10);
        for w in keys.windows(2) {
            assert!(w[0] < w[1], "{} should be < {}", w[0], w[1]);
        }
    }

    #[test]
    fn generate_n_uniform_length() {
        let keys = generate_n(5);
        for k in &keys {
            assert_eq!(k.len(), DEFAULT_LEN);
        }
    }

    #[test]
    fn generate_n_zero() {
        assert!(generate_n(0).is_empty());
    }

    #[test]
    fn key_between_both_none() {
        let mid = key_between(None, None);
        assert_eq!(mid.len(), DEFAULT_LEN);
        assert!(mid.as_str() > "0000");
        assert!(mid.as_str() < "zzzz");
    }

    #[test]
    fn key_between_exact_midpoint() {
        let mid = key_between(Some("i000"), Some("i002"));
        assert_eq!(mid, "i001");
    }

    #[test]
    fn key_between_adjacent_extends() {
        let a = "i000";
        let b = "i001";
        let mid = key_between(Some(a), Some(b));
        assert_eq!(mid.len(), 5);
        assert!(mid > a.to_string());
        assert!(mid < b.to_string());
    }

    #[test]
    fn key_between_before_first() {
        let b = "1000";
        let mid = key_between(None, Some(b));
        assert!(mid < b.to_string());
        assert!(mid > "0000".to_string());
    }

    #[test]
    fn key_between_after_last() {
        let a = "zzzz";
        let mid = key_between(Some(a), None);
        assert!(mid > a.to_string());
    }

    #[test]
    fn key_between_with_generated_keys() {
        let keys = generate_n(5);
        for w in keys.windows(2) {
            let mid = key_between(Some(&w[0]), Some(&w[1]));
            assert!(mid > w[0], "{mid} should be > {}", w[0]);
            assert!(mid < w[1], "{mid} should be < {}", w[1]);
        }
    }

    #[test]
    fn key_between_before_generated() {
        let keys = generate_n(3);
        let before = key_between(None, Some(&keys[0]));
        assert!(before < keys[0]);
    }

    #[test]
    fn key_between_after_generated() {
        let keys = generate_n(3);
        let after = key_between(Some(&keys[2]), None);
        assert!(after > keys[2]);
    }

    #[test]
    fn repeated_splits_stay_ordered() {
        let mut key = key_between(None, None);
        let end = "zzzz".to_string();
        for _ in 0..20 {
            let next = key_between(Some(&key), Some(&end));
            assert!(next > key, "{next} should be > {key}");
            assert!(next < end, "{next} should be < {end}");
            key = next;
        }
    }

    #[test]
    fn repeated_left_splits_stay_ordered() {
        let start = "0000".to_string();
        let mut key = key_between(None, None);
        for _ in 0..20 {
            let prev = key_between(Some(&start), Some(&key));
            assert!(prev > start, "{prev} should be > {start}");
            assert!(prev < key, "{prev} should be < {key}");
            key = prev;
        }
    }
}
