//! Edge-focused tests for malformed CSS: never-panic, verbatim passthrough of
//! unparseable regions, and diagnostics with correct `TextRange`s. Complements
//! the never-panic fuzz corpus in `tests/robustness.rs` and the two basic
//! diagnostic tests in `tests/scoping.rs`.

use lunas_css::scope_css;

const S: &str = "data-lunas-x";

/// Every diagnostic range must be a valid, in-bounds, non-inverted range into
/// the original input.
fn assert_ranges_valid(input: &str, diags: &[lunas_span::Diagnostic]) {
    for d in diags {
        let start = d.range.start().as_usize();
        let end = d.range.end().as_usize();
        assert!(start <= end, "inverted range {start}..{end} for {input:?}");
        assert!(
            end <= input.len(),
            "range {start}..{end} exceeds input len {} for {input:?}",
            input.len()
        );
    }
}

#[test]
fn unterminated_declaration_block_diagnostic_range() {
    let src = ".a { color: red";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("unterminated block"));
    // Range covers from just after `{` to EOF.
    assert_eq!(diags[0].range.start().as_usize(), 4);
    assert_eq!(diags[0].range.end().as_usize(), src.len());
    // The selector is still scoped even though the block never closes.
    assert_eq!(out, ".a[data-lunas-x] { color: red");
}

#[test]
fn selector_with_no_brace_at_all_emitted_verbatim_unscoped() {
    // No `{` anywhere: the walker can't tell this is a selector prelude vs.
    // trailing junk, so it emits the raw text untouched (not scoped) and
    // reports "missing `{`".
    let src = ".a";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, ".a");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("missing `{`"));
    assert_eq!(diags[0].range.start().as_usize(), 0);
    assert_eq!(diags[0].range.end().as_usize(), 2);
}

#[test]
fn unterminated_comment_passthrough_no_diagnostic() {
    // An unterminated `/* ...` consumes to EOF as an inert comment; this is
    // not reported as a diagnostic (the tokenizer's `consume_comment` just
    // swallows to EOF silently).
    let src = "/* unterminated";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, src);
    assert!(diags.is_empty());
}

#[test]
fn unterminated_string_inside_declaration_block() {
    let src = ".a { \"unterminated string";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("unterminated block"));
    assert!(out.starts_with(".a[data-lunas-x] {"), "{out}");
    assert!(out.ends_with("unterminated string"), "{out}");
}

#[test]
fn unterminated_at_rule_block_diagnostic() {
    let src = "@media screen {";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("unterminated at-rule block"));
    assert_eq!(out, src);
}

#[test]
fn unterminated_nested_rule_inside_at_rule_reports_two_diagnostics() {
    // The @media block itself never closes, and neither does the qualified
    // rule inside it — both diagnostics are produced, with ranges relative to
    // the *original* source (base-shifted correctly for the nested one).
    let src = "@media screen { .a { color: red";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(diags.len(), 2);
    assert!(diags
        .iter()
        .any(|d| d.message.contains("unterminated at-rule block")));
    assert!(diags
        .iter()
        .any(|d| d.message.contains("unterminated block (missing")));
    assert!(out.contains(".a[data-lunas-x] { color: red"), "{out}");
}

#[test]
fn unterminated_keyframes_block_diagnostic() {
    let src = "@keyframes x { from {";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("unterminated @keyframes block"));
    // The keyframes name is still renamed even though the block is broken.
    assert!(out.starts_with("@keyframes x-x {"), "{out}");
}

#[test]
fn stray_closing_braces_before_a_valid_rule() {
    // Leading `}` characters become part of the "selector" prelude text for
    // the next rule scan; the transform never panics and always makes
    // progress. We assert only the never-panic + range-validity invariants
    // here since the exact recovery shape is an implementation detail.
    let src = "}}}{{{ .a {} ";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert!(!out.is_empty());
}

#[test]
fn only_closing_brace() {
    let src = "}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, "}");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("missing `{`"));
}

#[test]
fn extra_closing_brace_after_valid_rule() {
    let src = ".a { color: red; } }";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, ".a[data-lunas-x] { color: red; } }");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("missing `{`"));
}

#[test]
fn unclosed_attribute_selector_bracket() {
    let src = "a[href {}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    // Never panics; produces *some* best-effort output.
    let _ = out;
}

#[test]
fn unclosed_functional_pseudo_paren() {
    let src = ".x:not(.a {}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    let _ = out;
}

#[test]
fn multiple_unterminated_strings_in_sequence() {
    let src = "a[x=\"1] b[y='2] .c {}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    let _ = out;
}

#[test]
fn valid_rule_after_malformed_rule_still_scoped() {
    // Even when an earlier rule is malformed, a well-formed rule later in the
    // stream should still get scoped once the parser resynchronizes... though
    // per the walker's design, an unterminated rule consumes to EOF, so a
    // "valid rule after" only applies when the earlier issue is recoverable
    // (e.g. a stray `;`).
    let src = ".a; .b {}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, ".a; .b[data-lunas-x] {}");
    assert!(diags.is_empty());
}

#[test]
fn diagnostic_range_never_exceeds_input_for_empty_input() {
    let (out, diags) = scope_css("", S);
    assert_ranges_valid("", &diags);
    assert_eq!(out, "");
    assert!(diags.is_empty());
}

#[test]
fn never_panics_on_lone_at_sign() {
    let src = "@";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert_eq!(out, "@");
}

#[test]
fn never_panics_on_null_bytes_mixed_with_valid_css() {
    let src = ".a\0.b { color\0: red\0 }";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    let _ = out;
}

#[test]
fn never_panics_on_lone_high_surrogate_like_byte_sequences() {
    // Rust `&str` is always valid UTF-8, but adversarial-looking multi-byte
    // sequences (e.g. overlong-looking emoji clusters) must still not panic.
    let src = "🏳️‍🌈 { color: red } \u{200D}\u{FE0F} {}";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    let _ = out;
}

#[test]
fn deeply_nested_unclosed_parens_in_selector() {
    let src = format!(".x:not({}", "(".repeat(500));
    let (out, diags) = scope_css(&src, S);
    assert_ranges_valid(&src, &diags);
    let _ = out;
}

#[test]
fn deeply_nested_unclosed_media_blocks() {
    let src = "@media a {".repeat(500);
    let (out, diags) = scope_css(&src, S);
    assert_ranges_valid(&src, &diags);
    let _ = out;
}

#[test]
fn mixed_valid_and_invalid_rules_diagnostics_stay_in_bounds() {
    let src = ".a {} @media { .b { .c {} @keyframes { from {";
    let (out, diags) = scope_css(src, S);
    assert_ranges_valid(src, &diags);
    assert!(out.starts_with(".a[data-lunas-x] {}"), "{out}");
}
