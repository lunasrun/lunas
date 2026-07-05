//! Extended never-panic fuzz corpus for `scope_css`, complementing
//! `tests/robustness.rs`. This file uses different seeds, alphabets, and
//! generation strategies (grammar-guided selector/at-rule assembly, byte-level
//! mutation of valid CSS, and a much larger pure-random sweep at 10k+
//! iterations) to widen coverage of the input space without duplicating the
//! existing corpus.

use lunas_css::scope_css;
use std::panic::{catch_unwind, AssertUnwindSafe};

const SCOPE: &str = "data-lunas-fuzz9f8e";

fn assert_ok(input: &str) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let (out, diags) = scope_css(input, SCOPE);
        for d in &diags {
            assert!(
                d.range.end().as_usize() <= input.len(),
                "diagnostic range {:?} exceeds input len {} for {:?}",
                d.range,
                input.len(),
                input
            );
            assert!(d.range.start().as_usize() <= d.range.end().as_usize());
        }
        // Re-running on the transform's own output must also never panic —
        // exercises the transform being handed already-scoped CSS (e.g. if a
        // build pipeline accidentally double-processes a stylesheet).
        let (out2, _) = scope_css(&out, SCOPE);
        // ...and a third pass, since some rewrites (keyframe renaming) grow
        // the string each time a match is found in the accumulated suffixes.
        let _ = scope_css(&out2, SCOPE);
    }));
    assert!(result.is_ok(), "scope_css panicked on input: {input:?}");
}

/// Small deterministic xorshift RNG, distinct from `robustness.rs`'s LCG, so
/// the two corpora explore different sequences even with similar seeds.
struct Xorshift(u64);
impl Xorshift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}

#[test]
fn large_pure_random_byte_soup_10k_iterations() {
    // Alphabet includes raw bytes beyond the structural-token set used in
    // robustness.rs's pseudo_random_fuzz — more punctuation, digits, and a
    // couple of malformed-UTF8-adjacent (but still valid) code points.
    let alphabet: &[char] = &[
        '{', '}', '(', ')', '[', ']', ':', ';', ',', '.', '#', '*', '>', '+', '~', '/', '"', '\'',
        '\\', '@', '-', '_', '=', '^', '$', '|', '!', '%', '&', '?', ' ', '\n', '\t', '\r', '\0',
        '0', '1', '2', '9', 'n', 'a', 'z', 'A', 'Z', 'ص', 'ñ', '\u{200D}', '\u{FE0F}', '\u{2028}',
    ];
    let mut rng = Xorshift(0x9e37_79b9_7f4a_7c15);
    for _ in 0..10_000 {
        let len = rng.range(80);
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            let idx = rng.range(alphabet.len());
            s.push(alphabet[idx]);
        }
        assert_ok(&s);
    }
}

#[test]
fn grammar_guided_selector_fuzz_10k_iterations() {
    // Assembles pseudo-CSS from fragments that look like real selectors/rules
    // (rather than raw characters), so this corpus is more likely to hit
    // "almost valid" structures the character-soup fuzzers rarely produce.
    let selectors: &[&str] = &[
        "a",
        ".b",
        "#c",
        "*",
        "a.b#c",
        "[d]",
        "[d=e]",
        "[d=\"e,f\"]",
        "a:hover",
        "a::before",
        "a:not(.x, .y)",
        "a:is(.x, .y)",
        ":deep(.x)",
        ":global(.x)",
        "a:deep(.b)",
        "a:global(.b)",
        "a\\:esc",
        ".日本語",
        "svg|circle",
        "*|*",
    ];
    let combinators: &[&str] = &[" ", " > ", " + ", " ~ ", ">", "+", "~", "  ", ""];
    let at_preludes: &[&str] = &[
        "@media screen",
        "@media (min-width: 1px)",
        "@supports (display:grid)",
        "@layer x",
        "@container (min-width: 1px)",
        "@scope (.a)",
        "@keyframes spin",
        "@-webkit-keyframes spin",
        "@font-face",
        "@page",
        "@import url(x)",
        "@charset \"utf-8\"",
        "@weird-unknown-thing",
    ];
    let decls: &[&str] = &[
        "color: red",
        "animation: spin 1s",
        "animation-name: spin",
        "content: '→'",
        "background: url(a,b)",
        "",
        ":",
        "prop",
    ];
    let terminators: &[&str] = &["{", "}", ";", ""];

    let mut rng = Xorshift(0xc2b2_ae3d_27d4_eb4f);
    for _ in 0..10_000 {
        let mut s = String::new();
        let pieces = 1 + rng.range(12);
        for _ in 0..pieces {
            match rng.range(5) {
                0 => s.push_str(selectors[rng.range(selectors.len())]),
                1 => s.push_str(combinators[rng.range(combinators.len())]),
                2 => s.push_str(at_preludes[rng.range(at_preludes.len())]),
                3 => s.push_str(decls[rng.range(decls.len())]),
                _ => s.push_str(terminators[rng.range(terminators.len())]),
            }
        }
        assert_ok(&s);
    }
}

#[test]
fn mutated_valid_css_fuzz_5k_iterations() {
    // Start from a corpus of *valid* stylesheets and randomly delete/duplicate
    // bytes — a classic mutation-fuzzing strategy that tends to find
    // off-by-one boundary bugs that pure generation misses.
    let seeds: &[&str] = &[
        ".a { color: red } .b:hover { color: blue }",
        "@media screen { .a, .b > c { color: red } }",
        "@keyframes spin { from { opacity: 0 } to { opacity: 1 } } .x { animation: spin 1s }",
        ":global(.a) .b:deep(.c) {}",
        "a[href^=\"https://\"]:not(.external, .internal) {}",
        "@supports (display: grid) { @media (min-width: 1px) { .a {} } }",
        "/* comment */ .a /* mid */ { /* in-block */ color: red /* trailing */ }",
    ];
    let mut rng = Xorshift(0x1656_67b1_9e37_79f9);
    for seed in seeds {
        for _ in 0..700 {
            let bytes = seed.as_bytes();
            if bytes.is_empty() {
                continue;
            }
            let mut mutated: Vec<u8> = bytes.to_vec();
            let ops = 1 + rng.range(6);
            for _ in 0..ops {
                if mutated.is_empty() {
                    break;
                }
                match rng.range(4) {
                    0 => {
                        // Delete a byte at a random position.
                        let i = rng.range(mutated.len());
                        mutated.remove(i);
                    }
                    1 => {
                        // Duplicate a byte at a random position.
                        let i = rng.range(mutated.len());
                        let b = mutated[i];
                        mutated.insert(i, b);
                    }
                    2 => {
                        // Overwrite a byte with a structural character.
                        let structural: &[u8] = b"{}()[]:;,\"'\\@><+~ .#*";
                        let i = rng.range(mutated.len());
                        mutated[i] = structural[rng.range(structural.len())];
                    }
                    _ => {
                        // Truncate from the end.
                        let cut = rng.range(mutated.len() / 2 + 1);
                        mutated.truncate(mutated.len() - cut);
                    }
                }
            }
            // Mutation can produce invalid UTF-8; lossily repair it so the
            // fuzzer still explores *some* string, since scope_css only
            // accepts &str.
            let s = String::from_utf8_lossy(&mutated).into_owned();
            assert_ok(&s);
        }
    }
}

#[test]
fn adversarial_boundary_literals() {
    // Hand-picked inputs targeting specific boundary conditions in the
    // scanner/selector/rewrite modules that are easy to get subtly wrong.
    let cases = [
        // Scope attribute edge cases handled elsewhere, but paired with
        // pathological CSS here.
        ":not(:not(:not(:not(.a)))) {}",
        ":is(:where(:is(:where(.a))))  {}",
        "a:deep(:global(.b)) {}",
        ":global(:deep(.a)) {}",
        ".a:global(.b):deep(.c) {}",
        "@keyframes k {} @keyframes k {} @keyframes k {} .x { animation: k 1s, k 2s, k 3s }",
        "@media (min-width:1px){@media(min-width:2px){@media(min-width:3px){.a{}}}}",
        "a[b=\"\\\"\"] {}",
        "a[b='\\''] {}",
        ".a\\  {}",
        "@charset",
        "@charset;",
        "@charset \"a\" \"b\";",
        "@page:first{}",
        "@page :first{}",
        "*{}",
        "**{}",
        "a::before::after {}",
        "::before:hover {}",
        "a,,,,,b{}",
        ".a[",
        ".a[b",
        ".a[b=",
        ".a[b=c",
        ".a[b=c]",
        ".a[b=c] {",
        "\u{feff}.a {}",
        "a\u{200b}.b {}",
        "@media\u{a0}screen{.a{}}",
        &"a".repeat(5000),
        &format!("{}{{}}", ".a ".repeat(2000)),
        &format!(".a{{{}}}", "color:red;".repeat(2000)),
    ];
    for c in cases {
        assert_ok(c);
    }
}

#[test]
fn wide_scope_attribute_values_do_not_panic() {
    // scope_css takes an arbitrary scope_attr string too; malformed/adversarial
    // attrs must not cause panics regardless of the CSS input.
    let attrs = [
        "",
        "data-lunas-",
        "]",
        "[",
        "\"",
        "'",
        " ",
        "\n",
        "a b",
        "🎉",
    ];
    for attr in attrs {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = scope_css(
                ".a:hover { color: red } @keyframes k {} .b{animation:k 1s}",
                attr,
            );
        }));
        assert!(result.is_ok(), "panicked with scope_attr {attr:?}");
    }
}
