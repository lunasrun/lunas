//! Edge-focused tests for at-rule handling: conditional-group recursion
//! (`@media`/`@supports`/`@layer`/`@container`/`@scope`), vendor prefixes,
//! nesting, and pass-through at-rules (`@font-face`/`@import`/`@charset`/
//! `@page`/unknown). Complements the baseline cases in `tests/scoping.rs`.

use lunas_css::scope_css;

const S: &str = "data-lunas-x";

fn scope(css: &str) -> String {
    let (out, diags) = scope_css(css, S);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics: {diags:?}\ninput: {css:?}"
    );
    out
}

// --- @media ------------------------------------------------------------------

#[test]
fn media_with_and_condition() {
    let out = scope("@media screen and (min-width: 700px) { .a {} }");
    assert_eq!(
        out,
        "@media screen and (min-width: 700px) { .a[data-lunas-x] {} }"
    );
}

#[test]
fn media_with_multiple_rules_inside() {
    let out = scope("@media screen { .a {} .b {} }");
    assert_eq!(
        out,
        "@media screen { .a[data-lunas-x] {} .b[data-lunas-x] {} }"
    );
}

#[test]
fn media_with_selector_list_inside() {
    let out = scope("@media screen { a, b {} }");
    assert_eq!(out, "@media screen { a[data-lunas-x], b[data-lunas-x] {} }");
}

// --- @supports -----------------------------------------------------------------

#[test]
fn supports_with_not_condition() {
    let out = scope("@supports not (display: grid) { .a {} }");
    assert_eq!(out, "@supports not (display: grid) { .a[data-lunas-x] {} }");
}

#[test]
fn supports_nested_in_media() {
    let out = scope("@media screen and (min-width: 1px) { @supports (display:grid) { .a { } } }");
    assert_eq!(
        out,
        "@media screen and (min-width: 1px) { @supports (display:grid) { .a[data-lunas-x] { } } }"
    );
}

// --- @layer ----------------------------------------------------------------

#[test]
fn layer_statement_multi_name_passthrough() {
    assert_eq!(
        scope("@layer base, components, utilities;"),
        "@layer base, components, utilities;"
    );
}

#[test]
fn layer_statement_then_layer_block() {
    let src = "@layer base, components, utilities;\n@layer base { .a {} }";
    let out = scope(src);
    assert_eq!(
        out,
        "@layer base, components, utilities;\n@layer base { .a[data-lunas-x] {} }"
    );
}

#[test]
fn anonymous_layer_block_recurses() {
    let out = scope("@layer { .a {} }");
    assert_eq!(out, "@layer { .a[data-lunas-x] {} }");
}

#[test]
fn nested_layer_inside_layer() {
    let out = scope("@layer outer { @layer inner { .a {} } }");
    assert_eq!(out, "@layer outer { @layer inner { .a[data-lunas-x] {} } }");
}

// --- @container --------------------------------------------------------------

#[test]
fn container_named_query() {
    let out = scope("@container sidebar (min-width: 400px) { .a {} }");
    assert_eq!(
        out,
        "@container sidebar (min-width: 400px) { .a[data-lunas-x] {} }"
    );
}

#[test]
fn container_unnamed_query() {
    let out = scope("@container (min-width: 400px) { .a {} }");
    assert_eq!(out, "@container (min-width: 400px) { .a[data-lunas-x] {} }");
}

// --- @scope --------------------------------------------------------------------

#[test]
fn scope_at_rule_recurses() {
    let out = scope("@scope (.a) { .b {} }");
    assert_eq!(out, "@scope (.a) { .b[data-lunas-x] {} }");
}

#[test]
fn scope_at_rule_with_to_clause() {
    let out = scope("@scope (.a) to (.b) { .c {} }");
    assert_eq!(out, "@scope (.a) to (.b) { .c[data-lunas-x] {} }");
}

// --- vendor prefixes on conditional groups -----------------------------------

#[test]
fn vendor_prefixed_media_treated_as_unknown_group_name() {
    // `strip_vendor` recognizes `-webkit-keyframes` for @keyframes, but the
    // conditional-group check (`is_conditional_group`) also strips vendor
    // prefixes, so `@-webkit-media` still recurses like `@media`.
    let out = scope("@-webkit-media (min-width: 1px) { .a {} }");
    assert_eq!(
        out,
        "@-webkit-media (min-width: 1px) { .a[data-lunas-x] {} }"
    );
}

#[test]
fn moz_document_unknown_atrule_not_scoped() {
    // `@-moz-document` (an unknown at-rule after vendor-stripping) is NOT a
    // conditional group; its block is treated as declarations and is not
    // scoped even though it looks like it contains a rule.
    let out = scope("@-moz-document url(x) { .a {} }");
    assert_eq!(out, "@-moz-document url(x) { .a {} }");
}

// --- deep nesting --------------------------------------------------------------

#[test]
fn triple_nested_conditional_groups() {
    let out = scope("@media screen { @supports (display:grid) { @layer x { .a {} } } }");
    assert_eq!(
        out,
        "@media screen { @supports (display:grid) { @layer x { .a[data-lunas-x] {} } } }"
    );
}

// --- pass-through at-rules -----------------------------------------------------

#[test]
fn font_face_with_multiple_declarations_untouched() {
    let src = "@font-face { font-family: 'X'; src: url(x.woff) format('woff'); font-weight: 400 }";
    assert_eq!(scope(src), src);
}

#[test]
fn page_with_pseudo_class_untouched() {
    let src = "@page :first { margin: 1in }";
    assert_eq!(scope(src), src);
}

#[test]
fn page_without_pseudo_untouched() {
    let src = "@page { size: A4 }";
    assert_eq!(scope(src), src);
}

#[test]
fn multiple_page_rules_untouched() {
    let src = "@page :first { margin: 1in } @page { size: A4 }";
    assert_eq!(scope(src), src);
}

#[test]
fn import_with_media_query_untouched() {
    let src = "@import url('x.css') screen;";
    assert_eq!(scope(src), src);
}

#[test]
fn import_with_layer_function_untouched() {
    let src = "@import 'foo.css' layer(base);";
    assert_eq!(scope(src), src);
}

#[test]
fn charset_untouched_and_first() {
    let src = "@charset \"utf-8\";\n.a {}";
    let out = scope(src);
    assert_eq!(out, "@charset \"utf-8\";\n.a[data-lunas-x] {}");
}

#[test]
fn unknown_at_rule_with_block_not_scoped() {
    // An unknown at-rule's block is treated as declarations, not rules — even
    // though the content inside looks like a qualified rule, it must not be
    // scoped, matching the documented non-goal.
    let out = scope("@unknown-at-rule (foo) { .a { color: red } }");
    assert_eq!(out, "@unknown-at-rule (foo) { .a { color: red } }");
}

#[test]
fn unknown_at_rule_statement_form_untouched() {
    let src = "@my-custom-rule foo bar;";
    assert_eq!(scope(src), src);
}

#[test]
fn font_face_inside_media_not_scoped() {
    // `@font-face` nested inside a conditional group is still a declaration
    // block, not scoped, even though the walker recurses into the @media body.
    let out = scope("@media screen { @font-face { src: url(x) } }");
    assert_eq!(out, "@media screen { @font-face { src: url(x) } }");
}

// --- comments around at-rules --------------------------------------------------

#[test]
fn comment_before_at_rule() {
    let out = scope("/* c */ @media screen { .a {} }");
    assert_eq!(out, "/* c */ @media screen { .a[data-lunas-x] {} }");
}

#[test]
fn comment_inside_at_rule_prelude() {
    let out = scope("@media /* c */ screen { .a {} }");
    assert_eq!(out, "@media /* c */ screen { .a[data-lunas-x] {} }");
}
