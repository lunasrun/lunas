//! `scope_css` must never panic, for any input. These tests throw a large
//! deterministic corpus of adversarial and pseudo-random strings at it and
//! assert each call returns (a panic would unwind and be caught here). They
//! also check two structural invariants that must hold for every input:
//!
//! * the transform is idempotent-safe in the sense that it never produces a
//!   string that fails on a second pass;
//! * every diagnostic range stays within the bounds of the input.

use lunas_css::scope_css;
use std::panic::{catch_unwind, AssertUnwindSafe};

const SCOPE: &str = "data-lunas-abcd1234";

fn assert_ok(input: &str) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let (out, diags) = scope_css(input, SCOPE);
        // Diagnostic ranges must land inside the input.
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
        // A second pass over the output must also not panic.
        let _ = scope_css(&out, SCOPE);
    }));
    assert!(result.is_ok(), "scope_css panicked on input: {input:?}");
}

#[test]
fn adversarial_literals_do_not_panic() {
    let cases = [
        "",
        "{",
        "}",
        "{}",
        "}{",
        "/*",
        "*/",
        "/* unterminated",
        "a {",
        "a { b",
        "a }",
        ".",
        "#",
        ":",
        "::",
        ":::",
        "[",
        "]",
        "[]",
        "[[[]]]",
        "((()))",
        "a[",
        "a[b",
        "a[b=",
        "a[b=\"",
        "a[b='",
        "a[b=\"c]",
        ":not(",
        ":not()",
        ":deep(",
        ":deep()",
        ":global(",
        ":global()",
        ":deep",
        ":global",
        "@",
        "@media",
        "@media {",
        "@media { a {",
        "@keyframes",
        "@keyframes {",
        "@keyframes x {",
        "@keyframes x { from {",
        "@import",
        "@import ;",
        "@font-face {",
        "@layer",
        "@layer;",
        "@-webkit-keyframes x {}",
        ",",
        ",,,",
        "a,,b {}",
        "  ,  { }",
        ">",
        "> >",
        "a > > b {}",
        "\\",
        "\\}",
        "a\\{b {}",
        "\0",
        "a\0b {}",
        "/* } { ; @ */",
        "\"unterminated string",
        "'x",
        "あ { color: 赤 }",
        "\u{1F600} {}",
        ".a { animation: }",
        ".a { animation }",
        ".a { : val }",
        "* { }",
        "&& {}",
        "@keyframes @ {}",
    ];
    for c in cases {
        assert_ok(c);
    }
}

#[test]
fn deeply_nested_does_not_panic() {
    let open_braces = "@media screen {".repeat(2000);
    let close_braces = "}".repeat(2000);
    assert_ok(&open_braces);
    assert_ok(&close_braces);
    assert_ok(&format!("{open_braces}.a{{}}{close_braces}"));

    let deep_parens = format!(".x:not({}) {{}}", "(".repeat(1000));
    assert_ok(&deep_parens);
}

/// A tiny deterministic LCG so the fuzz corpus is reproducible.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

#[test]
fn pseudo_random_fuzz_does_not_panic() {
    // Alphabet weighted toward characters that drive the scanner's states.
    let alphabet: &[char] = &[
        '{',
        '}',
        '(',
        ')',
        '[',
        ']',
        ':',
        ';',
        ',',
        '.',
        '#',
        '*',
        '>',
        '+',
        '~',
        '/',
        '*',
        '"',
        '\'',
        '\\',
        '@',
        '-',
        ' ',
        '\n',
        '\t',
        '\0',
        'a',
        'b',
        'c',
        'x',
        'k',
        'e',
        'y',
        'あ',
        '\u{1F600}',
    ];
    let mut rng = Lcg(0x0bad_c0de_dead_beef);
    for _ in 0..8000 {
        let len = (rng.next() % 60) as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            let idx = (rng.next() as usize) % alphabet.len();
            s.push(alphabet[idx]);
        }
        assert_ok(&s);
    }
}

#[test]
fn keyword_soup_fuzz_does_not_panic() {
    // Fuzz built from CSS keywords/tokens, which stresses the at-rule dispatch.
    let tokens: &[&str] = &[
        "@media",
        "@supports",
        "@keyframes",
        "@font-face",
        "@import",
        "@layer",
        "@charset",
        "@-webkit-keyframes",
        ":deep(",
        ":global(",
        ":not(",
        ":hover",
        "::before",
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        ";",
        ",",
        ".a",
        "#b",
        "> c",
        "animation:",
        "spin",
        "1s",
        "from",
        "to",
        "url(x)",
        "/* c */",
        "\"s\"",
        "\\",
        "  ",
    ];
    let mut rng = Lcg(0x1234_5678_9abc_def0);
    for _ in 0..8000 {
        let n = (rng.next() % 30) as usize;
        let mut s = String::new();
        for _ in 0..n {
            let idx = (rng.next() as usize) % tokens.len();
            s.push_str(tokens[idx]);
        }
        assert_ok(&s);
    }
}
