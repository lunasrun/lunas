//! Edge-focused tests for `scope_id`: determinism/stability across repeated
//! calls, distinctness across different sources, format invariants, and the
//! integration contract with `scope_css` (same id used for both DOM stamping
//! and CSS rewriting must round-trip consistently).

use lunas_css::{scope_css, scope_id};

#[test]
fn deterministic_across_calls() {
    let src = "<template><div class=\"a\"></div></template>";
    let a = scope_id(src);
    let b = scope_id(src);
    let c = scope_id(src);
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn deterministic_across_many_repeated_calls() {
    let src = "component source text with some content";
    let first = scope_id(src);
    for _ in 0..100 {
        assert_eq!(scope_id(src), first);
    }
}

#[test]
fn distinct_for_different_sources() {
    let a = scope_id("component A source");
    let b = scope_id("component B source");
    assert_ne!(a, b);
}

#[test]
fn distinct_for_whitespace_only_difference() {
    // Even a single trailing space must (almost always) produce a different
    // hash — scope_id has no normalization step.
    let a = scope_id("<template></template>");
    let b = scope_id("<template></template> ");
    assert_ne!(a, b);
}

#[test]
fn distinct_for_case_difference() {
    let a = scope_id("Component");
    let b = scope_id("component");
    assert_ne!(a, b);
}

#[test]
fn empty_source_has_stable_id() {
    let a = scope_id("");
    let b = scope_id("");
    assert_eq!(a, b);
    assert_eq!(a, "data-lunas-811c9dc5");
}

#[test]
fn unicode_source_produces_valid_id() {
    let id = scope_id("日本語コンポーネント");
    assert!(id.starts_with("data-lunas-"));
    let hex = id.strip_prefix("data-lunas-").unwrap();
    assert_eq!(hex.len(), 8);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn long_source_produces_valid_fixed_length_id() {
    let long_src = "x".repeat(10_000);
    let id = scope_id(&long_src);
    let hex = id.strip_prefix("data-lunas-").unwrap();
    assert_eq!(hex.len(), 8);
}

#[test]
fn id_format_is_always_lowercase_hex_of_fixed_length() {
    let sources = ["", "a", "ab", "component X", "🎉", "\0\0\0"];
    for src in sources {
        let id = scope_id(src);
        assert!(id.starts_with("data-lunas-"), "{id}");
        let hex = &id["data-lunas-".len()..];
        assert_eq!(hex.len(), 8, "unexpected hex length for {src:?}: {id}");
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hex digits should be lowercase for {src:?}: {id}"
        );
    }
}

#[test]
fn scope_id_output_is_a_valid_scope_css_attribute() {
    // Round-trip: the id produced by scope_id must work as scope_attr for
    // scope_css without any adaptation (the integration contract described in
    // the README).
    let attr = scope_id("<template><div class=\"x\"></div></template>");
    let (out, diags) = scope_css(".x {}", &attr);
    assert!(diags.is_empty());
    assert_eq!(out, format!(".x[{attr}] {{}}"));
}

#[test]
fn different_components_yield_different_scoped_output() {
    let attr_a = scope_id("component A");
    let attr_b = scope_id("component B");
    assert_ne!(attr_a, attr_b);
    let (out_a, _) = scope_css(".x {}", &attr_a);
    let (out_b, _) = scope_css(".x {}", &attr_b);
    assert_ne!(out_a, out_b);
}

#[test]
fn near_duplicate_sources_rarely_collide() {
    // A basic avalanche sanity-check: flipping a single character anywhere in
    // a longer source should (almost certainly) change the hash.
    let base = "abcdefghijklmnopqrstuvwxyz0123456789";
    let base_id = scope_id(base);
    let mut collisions = 0;
    for i in 0..base.len() {
        let mut mutated: Vec<u8> = base.as_bytes().to_vec();
        mutated[i] = if mutated[i] == b'z' { b'y' } else { b'z' };
        let mutated = String::from_utf8(mutated).unwrap();
        if scope_id(&mutated) == base_id {
            collisions += 1;
        }
    }
    assert_eq!(
        collisions, 0,
        "expected no collisions from single-byte flips"
    );
}
