//! A tiny inline FNV-1a hash, so scope ids need no external dependency and
//! stay identical across platforms and the wasm32 target.

/// 32-bit FNV-1a of `input`, rendered as 8 lowercase hex digits.
pub(crate) fn fnv1a_hex(input: &str) -> String {
    const OFFSET: u32 = 0x811c_9dc5;
    const PRIME: u32 = 0x0100_0193;
    let mut hash = OFFSET;
    for &b in input.as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_hex() {
        let a = fnv1a_hex("hello");
        let b = fnv1a_hex("hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 8);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn distinct_inputs_differ() {
        assert_ne!(fnv1a_hex("a"), fnv1a_hex("b"));
        assert_ne!(fnv1a_hex(""), fnv1a_hex("x"));
    }

    #[test]
    fn known_vector() {
        // FNV-1a 32-bit of the empty string is the offset basis.
        assert_eq!(fnv1a_hex(""), "811c9dc5");
    }
}
