//! Edge-focused selector tests: combinators, attribute selectors, pseudo-
//! classes/elements, compound selectors, escapes, unicode, `:is()`/`:where()`.
//! Complements `tests/scoping.rs`, which covers the baseline/happy paths.

use lunas_css::scope_css;

const S: &str = "data-lunas-x";

/// Convenience: scope with the default attribute and assert no diagnostics.
fn scope(css: &str) -> String {
    let (out, diags) = scope_css(css, S);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics: {diags:?}\ninput: {css:?}"
    );
    out
}

// --- basic simple-selector variants -----------------------------------------

#[test]
fn tight_selector_list_no_whitespace() {
    assert_eq!(
        scope(".a,.b,.c{}"),
        ".a[data-lunas-x],.b[data-lunas-x],.c[data-lunas-x]{}"
    );
}

#[test]
fn no_whitespace_between_rules() {
    assert_eq!(
        scope(".a{color:red}.b{color:blue}"),
        ".a[data-lunas-x]{color:red}.b[data-lunas-x]{color:blue}"
    );
}

#[test]
fn namespace_universal_selector() {
    assert_eq!(scope("*|* {}"), "*|*[data-lunas-x] {}");
}

#[test]
fn namespaced_type_selector() {
    assert_eq!(scope("svg|circle {}"), "svg|circle[data-lunas-x] {}");
}

// --- combinators -------------------------------------------------------------

#[test]
fn all_combinators_in_one_chain() {
    assert_eq!(
        scope("a b > c + d ~ e {}"),
        "a[data-lunas-x] b[data-lunas-x] > c[data-lunas-x] + d[data-lunas-x] ~ e[data-lunas-x] {}"
    );
}

#[test]
fn combinator_no_space_before_only() {
    assert_eq!(scope("a> b {}"), "a[data-lunas-x]> b[data-lunas-x] {}");
}

#[test]
fn combinator_no_space_after_only() {
    assert_eq!(scope("a >b {}"), "a[data-lunas-x] >b[data-lunas-x] {}");
}

#[test]
fn combinator_tabs_and_newlines() {
    let out = scope("a\t>\nb {}");
    assert_eq!(out, "a[data-lunas-x]\t>\nb[data-lunas-x] {}");
}

#[test]
fn combinator_with_comment_between() {
    // A comment sits between the whitespace and the combinator symbol.
    let out = scope("a /* c */ > b {}");
    assert_eq!(out, "a[data-lunas-x] /* c */ > b[data-lunas-x] {}");
}

// --- attribute selectors -----------------------------------------------------

#[test]
fn attribute_equals_no_quotes() {
    assert_eq!(
        scope("input[type=text] {}"),
        "input[type=text][data-lunas-x] {}"
    );
}

#[test]
fn attribute_tilde_equals() {
    assert_eq!(
        scope(r#"a[data-x~="foo"] {}"#),
        r#"a[data-x~="foo"][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_pipe_equals() {
    assert_eq!(
        scope(r#"a[lang|="en"] {}"#),
        r#"a[lang|="en"][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_pipe_equals_no_quotes() {
    assert_eq!(scope("a[lang|=en] {}"), "a[lang|=en][data-lunas-x] {}");
}

#[test]
fn attribute_caret_equals_prefix() {
    assert_eq!(
        scope(r#"a[href^="https://"] {}"#),
        r#"a[href^="https://"][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_dollar_equals_suffix() {
    assert_eq!(
        scope(r#"a[href$=".pdf"] {}"#),
        r#"a[href$=".pdf"][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_star_equals_substring() {
    assert_eq!(
        scope(r#"a[class*="foo"] {}"#),
        r#"a[class*="foo"][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_case_insensitive_flag() {
    assert_eq!(
        scope(r#"a[href^="foo" i] {}"#),
        r#"a[href^="foo" i][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_case_sensitive_flag() {
    assert_eq!(
        scope(r#"a[href^="foo" s] {}"#),
        r#"a[href^="foo" s][data-lunas-x] {}"#
    );
}

#[test]
fn attribute_single_quoted_value() {
    assert_eq!(scope("a[title='x'] {}"), "a[title='x'][data-lunas-x] {}");
}

#[test]
fn attribute_bracket_in_single_quoted_value() {
    let out = scope(r#"a[href='a]b'] {}"#);
    assert_eq!(out, r#"a[href='a]b'][data-lunas-x] {}"#);
}

#[test]
fn attribute_equals_in_string_value() {
    let out = scope(r#"a[title="a=b"] {}"#);
    assert_eq!(out, r#"a[title="a=b"][data-lunas-x] {}"#);
}

#[test]
fn attribute_escaped_bracket_no_quotes() {
    // An escaped `]` inside an (unquoted, technically-invalid) attribute value
    // must not prematurely close the bracket.
    let out = scope(r"a[data-x=foo\]bar] {}");
    assert_eq!(out, r"a[data-x=foo\]bar][data-lunas-x] {}");
}

#[test]
fn multiple_attribute_selectors_chained() {
    assert_eq!(
        scope("input[type=text][disabled] {}"),
        "input[type=text][disabled][data-lunas-x] {}"
    );
}

#[test]
fn attribute_selector_before_combinator() {
    assert_eq!(
        scope("a[href] > b[title] {}"),
        "a[href][data-lunas-x] > b[title][data-lunas-x] {}"
    );
}

// --- pseudo-classes / pseudo-elements ----------------------------------------

#[test]
fn hover_pseudo_class() {
    assert_eq!(scope("a:hover {}"), "a[data-lunas-x]:hover {}");
}

#[test]
fn nth_child_formula() {
    assert_eq!(
        scope("li:nth-child(2n+1) {}"),
        "li[data-lunas-x]:nth-child(2n+1) {}"
    );
}

#[test]
fn nth_child_odd_keyword() {
    assert_eq!(
        scope("li:nth-child(odd) {}"),
        "li[data-lunas-x]:nth-child(odd) {}"
    );
}

#[test]
fn nth_of_type_odd() {
    assert_eq!(
        scope(".a:nth-of-type(odd) {}"),
        ".a[data-lunas-x]:nth-of-type(odd) {}"
    );
}

#[test]
fn nth_child_of_selector() {
    // `:nth-child(An+B of S)` — the comma-free functional form.
    assert_eq!(
        scope(".a:nth-child(2n+1 of .b) {}"),
        ".a[data-lunas-x]:nth-child(2n+1 of .b) {}"
    );
}

#[test]
fn not_with_multiple_args_not_split() {
    assert_eq!(
        scope(".x:not(.a, .b, .c) {}"),
        ".x[data-lunas-x]:not(.a, .b, .c) {}"
    );
}

#[test]
fn chained_not_pseudo_classes() {
    assert_eq!(
        scope("a:not(.a):not(.b) {}"),
        "a[data-lunas-x]:not(.a):not(.b) {}"
    );
}

#[test]
fn is_pseudo_class_commas_not_split() {
    assert_eq!(scope(".x:is(.a, .b) {}"), ".x[data-lunas-x]:is(.a, .b) {}");
}

#[test]
fn where_pseudo_class_commas_not_split() {
    assert_eq!(
        scope(".x:where(.a, .b) {}"),
        ".x[data-lunas-x]:where(.a, .b) {}"
    );
}

#[test]
fn has_pseudo_class_commas_not_split() {
    assert_eq!(
        scope(".x:has(.a, > .b) {}"),
        ".x[data-lunas-x]:has(.a, > .b) {}"
    );
}

#[test]
fn nested_functional_pseudo_commas_not_split() {
    // `:not(:is(.a, .b))` — nested parens, comma is two levels deep.
    assert_eq!(
        scope(".x:not(:is(.a, .b)) {}"),
        ".x[data-lunas-x]:not(:is(.a, .b)) {}"
    );
}

#[test]
fn double_colon_pseudo_element() {
    assert_eq!(scope("a::before {}"), "a[data-lunas-x]::before {}");
}

#[test]
fn selection_pseudo_element() {
    assert_eq!(scope(".a::selection {}"), ".a[data-lunas-x]::selection {}");
}

#[test]
fn bare_double_colon_pseudo_element() {
    assert_eq!(scope("::selection {}"), "[data-lunas-x]::selection {}");
}

#[test]
fn pseudo_class_then_pseudo_element() {
    assert_eq!(
        scope("a:hover::before {}"),
        "a[data-lunas-x]:hover::before {}"
    );
}

#[test]
fn compound_with_attr_and_pseudo() {
    assert_eq!(
        scope("input[type=checkbox]:checked {}"),
        "input[type=checkbox][data-lunas-x]:checked {}"
    );
}

#[test]
fn compound_type_class_id_attr_pseudo() {
    assert_eq!(
        scope("a.b#c[d]:hover {}"),
        "a.b#c[d][data-lunas-x]:hover {}"
    );
}

// --- pseudo-class formulas / functional args with combinators ---------------

#[test]
fn not_with_combinator_inside() {
    assert_eq!(scope(".x:not(> .a) {}"), ".x[data-lunas-x]:not(> .a) {}");
}

#[test]
fn language_pseudo_class() {
    assert_eq!(scope(":lang(en) {}"), "[data-lunas-x]:lang(en) {}");
}

#[test]
fn dir_pseudo_class() {
    assert_eq!(scope(":dir(rtl) {}"), "[data-lunas-x]:dir(rtl) {}");
}

// --- selector lists with mixed complexity ------------------------------------

#[test]
fn selector_list_with_pseudo_and_combinator() {
    assert_eq!(
        scope("a:hover, b > c {}"),
        "a[data-lunas-x]:hover, b[data-lunas-x] > c[data-lunas-x] {}"
    );
}

#[test]
fn selector_list_trailing_comma_whitespace() {
    let out = scope("a, b, {}");
    assert_eq!(out, "a[data-lunas-x], b[data-lunas-x], {}");
}

#[test]
fn selector_list_leading_comma() {
    let out = scope(", a {}");
    assert_eq!(out, ", a[data-lunas-x] {}");
}

#[test]
fn selector_list_many_entries() {
    assert_eq!(
        scope("a, b, c, d, e {}"),
        "a[data-lunas-x], b[data-lunas-x], c[data-lunas-x], d[data-lunas-x], e[data-lunas-x] {}"
    );
}

#[test]
fn comma_inside_attribute_value_not_split() {
    let out = scope(r#"a[title="x, y, z"], b {}"#);
    assert_eq!(
        out,
        r#"a[title="x, y, z"][data-lunas-x], b[data-lunas-x] {}"#
    );
}

// --- escapes -----------------------------------------------------------------

#[test]
fn escaped_colon_in_class_name() {
    // `a\:hover` is a literal class-ish identifier with an escaped colon, not
    // a pseudo-class; the scanner treats the escape as opaque, so `attach_scope`
    // still finds no top-level `:` after the escape sequence itself.
    let out = scope(r"a\:hover {}");
    assert_eq!(out, r"a\:hover[data-lunas-x] {}");
}

#[test]
fn escaped_dot_in_id() {
    let out = scope(r"#a\.b {}");
    assert_eq!(out, r"#a\.b[data-lunas-x] {}");
}

#[test]
fn escaped_space_in_class() {
    let out = scope(r".foo\ bar {}");
    assert_eq!(out, r".foo\ bar[data-lunas-x] {}");
}

#[test]
fn escaped_bracket_in_class_name() {
    let out = scope(r".a\[1\] {}");
    assert_eq!(out, r".a\[1\][data-lunas-x] {}");
}

// --- unicode identifiers ------------------------------------------------------

#[test]
fn unicode_class_japanese() {
    assert_eq!(scope(".日本語 {}"), ".日本語[data-lunas-x] {}");
}

#[test]
fn unicode_id_cyrillic() {
    assert_eq!(scope("#привет {}"), "#привет[data-lunas-x] {}");
}

#[test]
fn unicode_type_selector_combinator() {
    assert_eq!(
        scope("café > 日本語 {}"),
        "café[data-lunas-x] > 日本語[data-lunas-x] {}"
    );
}

// --- :is()/:where() combined with combinators inside -------------------------

#[test]
fn is_with_descendant_inside_arg() {
    assert_eq!(scope(".x:is(.a .b) {}"), ".x[data-lunas-x]:is(.a .b) {}");
}

#[test]
fn where_nested_in_not() {
    assert_eq!(
        scope(":not(:where(.a, .b)) {}"),
        "[data-lunas-x]:not(:where(.a, .b)) {}"
    );
}
