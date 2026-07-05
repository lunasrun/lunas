//! Edge-focused tests for `@keyframes` renaming and `animation` /
//! `animation-name` property rewriting, including forward references,
//! shorthand parsing, and known limitations. Complements the baseline cases
//! in `tests/scoping.rs`.

use lunas_css::scope_css;

const SCOPE: &str = "data-lunas-x";

fn out(src: &str) -> String {
    let (out, diags) = scope_css(src, SCOPE);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics: {diags:?}\ninput: {src:?}"
    );
    out
}

#[test]
fn percentage_keyframe_selectors_untouched_and_renamed() {
    let src = "@keyframes spin { 0% { opacity: 0 } 50% { opacity: .5 } 100% { opacity: 1 } } \
               .x { animation: spin 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes spin-x {"), "{o}");
    assert!(o.contains("0% { opacity: 0 }"), "{o}");
    assert!(o.contains("50% { opacity: .5 }"), "{o}");
    assert!(o.contains("100% { opacity: 1 }"), "{o}");
    assert!(o.contains("animation: spin-x 1s"), "{o}");
}

#[test]
fn multiple_animations_comma_separated_both_rewritten() {
    let src = "@keyframes a {} @keyframes b {} .x { animation: a 1s, b 2s }";
    let o = out(src);
    assert!(o.contains("@keyframes a-x {}"), "{o}");
    assert!(o.contains("@keyframes b-x {}"), "{o}");
    assert!(o.contains("animation: a-x 1s, b-x 2s"), "{o}");
}

#[test]
fn animation_property_uppercase_still_recognized() {
    let src = "@keyframes fade {} .x { ANIMATION: fade 1s }";
    let o = out(src);
    assert!(o.contains("ANIMATION: fade-x 1s"), "{o}");
}

#[test]
fn animation_name_property_mixed_case_recognized() {
    let src = "@keyframes fade {} .x { animation-Name: fade }";
    let o = out(src);
    assert!(o.contains("animation-Name: fade-x"), "{o}");
}

#[test]
fn animation_value_with_no_matching_keyframes_untouched() {
    let src = ".x { animation: spin 1s }";
    assert_eq!(out(src), ".x[data-lunas-x] { animation: spin 1s }");
}

#[test]
fn keyframe_name_is_whole_token_not_substring() {
    // `spin-thing` and `spin` are different idents; renaming must not treat
    // one as a substring of the other.
    let src = "@keyframes spin-thing {} .x { animation: spin 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes spin-thing-x {}"), "{o}");
    // `spin` (used in the animation value) has no matching @keyframes name,
    // so it is left untouched.
    assert!(o.contains("animation: spin 1s"), "{o}");
}

#[test]
fn animation_value_substring_of_keyframe_name_untouched() {
    let src = "@keyframes spin {} .x { animation: spin-thing 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes spin-x {}"), "{o}");
    assert!(o.contains("animation: spin-thing 1s"), "{o}");
}

#[test]
fn duplicate_keyframes_declaration_dedups_mapping() {
    let src = "@keyframes spin {} @keyframes spin {} .x { animation: spin 1s }";
    let o = out(src);
    assert!(
        o.contains("@keyframes spin-x {} @keyframes spin-x {}"),
        "{o}"
    );
    assert!(o.contains("animation: spin-x 1s"), "{o}");
}

#[test]
fn multiple_declarations_in_one_block_all_rewritten() {
    let src = "@keyframes spin {} .a { animation: spin 1s; color: red; animation-name: spin }";
    let o = out(src);
    assert!(
        o.contains("animation: spin-x 1s; color: red; animation-name: spin-x"),
        "{o}"
    );
}

#[test]
fn keyframe_name_inside_unrelated_property_value_untouched() {
    // `url(spin.png)` contains the substring "spin" but is not the
    // `animation`/`animation-name` property, so it must not be touched.
    let src = "@keyframes spin {} .a { background: url(spin.png) }";
    let o = out(src);
    assert!(o.contains("background: url(spin.png)"), "{o}");
}

#[test]
fn keyframe_name_matching_is_case_sensitive() {
    let src = "@keyframes spin {} .a { animation: SPIN 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes spin-x {}"), "{o}");
    // `SPIN` != `spin`, so it's left untouched.
    assert!(o.contains("animation: SPIN 1s"), "{o}");
}

#[test]
fn keyframes_nested_in_media_are_not_collected_by_pass_one() {
    // Known limitation: the keyframe-name collection pass only scans
    // top-level at-rules; a `@keyframes` nested inside `@media` is renamed
    // when walked structurally... but since pass one never recorded its name,
    // no rename mapping exists, so neither the `@keyframes` name nor any
    // `animation` reference to it changes.
    let src = "@media screen { @keyframes spin {} } .a { animation: spin 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes spin {}"), "{o}");
    assert!(o.contains("animation: spin 1s"), "{o}");
}

#[test]
fn keyframes_with_no_usage_still_renamed() {
    let src = "@keyframes spin { from { transform: none } to { transform: rotate(1turn) } }";
    let o = out(src);
    assert!(o.starts_with("@keyframes spin-x {"), "{o}");
    assert!(o.contains("from { transform: none }"), "{o}");
    assert!(o.contains("to { transform: rotate(1turn) }"), "{o}");
}

#[test]
fn animation_shorthand_multiple_tokens_only_name_rewritten() {
    let src = "@keyframes fade {} .x { animation: 2s cubic-bezier(0.1, 0.7, 1, 0.1) 1s infinite alternate fade }";
    let o = out(src);
    assert!(
        o.contains("animation: 2s cubic-bezier(0.1, 0.7, 1, 0.1) 1s infinite alternate fade-x"),
        "{o}"
    );
}

#[test]
fn keyframe_suffix_derived_from_scope_hash_tail() {
    let (o, _) = scope_css("@keyframes spin {}", "data-lunas-deadbeef");
    assert!(o.contains("@keyframes spin-deadbeef"), "{o}");
}

#[test]
fn keyframe_suffix_falls_back_when_attr_has_no_dash() {
    // No trailing hash segment after a `-`: the whole attribute (minus
    // non-ident bytes) is used as the suffix.
    let (o, _) = scope_css("@keyframes spin {}", "customattr");
    assert!(o.contains("@keyframes spin-customattr"), "{o}");
}

#[test]
fn vendor_prefixed_keyframes_multiple_prefixes() {
    let src = "@-moz-keyframes spin {} @-ms-keyframes spin {} .x { animation: spin 1s }";
    let o = out(src);
    assert!(o.contains("@-moz-keyframes spin-x {}"), "{o}");
    assert!(o.contains("@-ms-keyframes spin-x {}"), "{o}");
    assert!(o.contains("animation: spin-x 1s"), "{o}");
}

#[test]
fn keyframes_name_with_hyphen_and_digits() {
    let src = "@keyframes slide-2-left {} .x { animation: slide-2-left 1s }";
    let o = out(src);
    assert!(o.contains("@keyframes slide-2-left-x {}"), "{o}");
    assert!(o.contains("animation: slide-2-left-x 1s"), "{o}");
}

#[test]
fn animation_name_forward_and_backward_reference_both_work() {
    let src = ".a { animation-name: spin } @keyframes spin {} .b { animation-name: spin }";
    let o = out(src);
    assert!(
        o.contains(".a[data-lunas-x] { animation-name: spin-x }"),
        "{o}"
    );
    assert!(o.contains("@keyframes spin-x {}"), "{o}");
    assert!(
        o.contains(".b[data-lunas-x] { animation-name: spin-x }"),
        "{o}"
    );
}

#[test]
fn empty_keyframes_body_renamed() {
    assert_eq!(out("@keyframes empty {}"), "@keyframes empty-x {}");
}

#[test]
fn keyframes_with_comment_after_name() {
    let src = "@keyframes spin /* c */ { to { opacity: 1 } }";
    let o = out(src);
    assert!(o.starts_with("@keyframes spin-x /* c */ {"), "{o}");
}
