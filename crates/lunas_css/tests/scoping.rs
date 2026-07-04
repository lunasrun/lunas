//! Behavioural tests for the scoped-CSS transform.

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

// --- plain selectors -------------------------------------------------------

#[test]
fn type_selector() {
    assert_eq!(
        scope("div { color: red }"),
        "div[data-lunas-x] { color: red }"
    );
}

#[test]
fn class_and_id() {
    assert_eq!(scope(".btn {}"), ".btn[data-lunas-x] {}");
    assert_eq!(scope("#main {}"), "#main[data-lunas-x] {}");
}

#[test]
fn universal_selector() {
    assert_eq!(scope("* {}"), "*[data-lunas-x] {}");
}

#[test]
fn chained_simple_selectors() {
    assert_eq!(scope("a.b#c {}"), "a.b#c[data-lunas-x] {}");
}

// --- combinators -----------------------------------------------------------

#[test]
fn descendant_combinator() {
    assert_eq!(scope("ul li {}"), "ul[data-lunas-x] li[data-lunas-x] {}");
}

#[test]
fn child_combinator() {
    assert_eq!(scope("a > b {}"), "a[data-lunas-x] > b[data-lunas-x] {}");
}

#[test]
fn adjacent_and_general_sibling() {
    assert_eq!(scope("a + b {}"), "a[data-lunas-x] + b[data-lunas-x] {}");
    assert_eq!(scope("a ~ b {}"), "a[data-lunas-x] ~ b[data-lunas-x] {}");
}

#[test]
fn combinator_without_spaces() {
    assert_eq!(scope("a>b {}"), "a[data-lunas-x]>b[data-lunas-x] {}");
}

#[test]
fn multiple_descendants() {
    assert_eq!(
        scope("main section p {}"),
        "main[data-lunas-x] section[data-lunas-x] p[data-lunas-x] {}"
    );
}

// --- pseudo-classes / elements ---------------------------------------------

#[test]
fn pseudo_class_after_scope() {
    assert_eq!(scope(".btn:hover {}"), ".btn[data-lunas-x]:hover {}");
}

#[test]
fn pseudo_element_after_scope() {
    assert_eq!(scope("a::before {}"), "a[data-lunas-x]::before {}");
}

#[test]
fn bare_pseudo_element() {
    // `::before` alone → scope goes at the front.
    assert_eq!(scope("::before {}"), "[data-lunas-x]::before {}");
}

#[test]
fn functional_pseudo_not_split() {
    // `:not(.a, .b)` — the inner comma must not split the list, and the scope
    // attaches before the pseudo.
    assert_eq!(
        scope(".x:not(.a, .b) {}"),
        ".x[data-lunas-x]:not(.a, .b) {}"
    );
}

#[test]
fn nth_child() {
    assert_eq!(
        scope("li:nth-child(2n+1) {}"),
        "li[data-lunas-x]:nth-child(2n+1) {}"
    );
}

// --- attribute selectors ---------------------------------------------------

#[test]
fn attribute_selector() {
    assert_eq!(
        scope("input[type=text] {}"),
        "input[type=text][data-lunas-x] {}"
    );
}

#[test]
fn attribute_selector_with_bracket_in_string() {
    // A `]` inside the attribute value string must not close the attribute.
    let out = scope(r#"a[href="a]b"] {}"#);
    assert_eq!(out, r#"a[href="a]b"][data-lunas-x] {}"#);
}

#[test]
fn attribute_selector_with_comma_in_string() {
    let out = scope(r#"a[title="x, y"], b {}"#);
    assert_eq!(out, r#"a[title="x, y"][data-lunas-x], b[data-lunas-x] {}"#);
}

#[test]
fn attribute_before_pseudo() {
    assert_eq!(
        scope("input[disabled]:focus {}"),
        "input[disabled][data-lunas-x]:focus {}"
    );
}

// --- selector lists --------------------------------------------------------

#[test]
fn selector_list() {
    assert_eq!(scope("a, b {}"), "a[data-lunas-x], b[data-lunas-x] {}");
}

#[test]
fn selector_list_complex() {
    assert_eq!(
        scope("a, b > c {}"),
        "a[data-lunas-x], b[data-lunas-x] > c[data-lunas-x] {}"
    );
}

#[test]
fn selector_list_whitespace_preserved() {
    let out = scope("a ,\n  b {}");
    assert_eq!(out, "a[data-lunas-x] ,\n  b[data-lunas-x] {}");
}

// --- :deep / :global -------------------------------------------------------

#[test]
fn deep_leaves_descendants_unscoped() {
    assert_eq!(scope(".a :deep(.b) {}"), ".a[data-lunas-x] .b {}");
}

#[test]
fn deep_attached_to_compound() {
    // `.a:deep(.b)` → scope `.a`, then `.b` unscoped.
    assert_eq!(scope(".a:deep(.b) {}"), ".a[data-lunas-x] .b {}");
}

#[test]
fn deep_with_trailing_selector() {
    assert_eq!(scope(".a :deep(.b .c) {}"), ".a[data-lunas-x] .b .c {}");
}

#[test]
fn global_leaves_selector_unscoped() {
    assert_eq!(scope(":global(.modal) {}"), ".modal {}");
}

#[test]
fn global_with_descendants() {
    assert_eq!(scope(":global(.a .b) {}"), ".a .b {}");
}

#[test]
fn global_anywhere_makes_selector_fully_global() {
    assert_eq!(scope(":global(.a) .b {}"), ".a .b {}");
}

#[test]
fn global_in_list_only_affects_its_selector() {
    assert_eq!(scope(":global(.a), .b {}"), ".a, .b[data-lunas-x] {}");
}

#[test]
fn global_prefixed_ident_is_not_global() {
    // `:globalish` is a (bogus) pseudo-class, not our escape hatch.
    assert_eq!(scope("a:globalish {}"), "a[data-lunas-x]:globalish {}");
}

// --- nested at-rules -------------------------------------------------------

#[test]
fn media_recurses() {
    let out = scope("@media (min-width: 700px) { .a {} }");
    assert_eq!(out, "@media (min-width: 700px) { .a[data-lunas-x] {} }");
}

#[test]
fn supports_recurses() {
    let out = scope("@supports (display: grid) { a b {} }");
    assert_eq!(
        out,
        "@supports (display: grid) { a[data-lunas-x] b[data-lunas-x] {} }"
    );
}

#[test]
fn layer_block_recurses() {
    let out = scope("@layer base { .a {} }");
    assert_eq!(out, "@layer base { .a[data-lunas-x] {} }");
}

#[test]
fn layer_statement_passthrough() {
    assert_eq!(scope("@layer a, b;"), "@layer a, b;");
}

#[test]
fn nested_media_inside_media() {
    let out = scope("@media screen { @media (min-width: 1px) { .a {} } }");
    assert_eq!(
        out,
        "@media screen { @media (min-width: 1px) { .a[data-lunas-x] {} } }"
    );
}

// --- keyframes -------------------------------------------------------------

#[test]
fn keyframes_renamed_and_referenced() {
    let src = "@keyframes spin { from { opacity: 0 } to { opacity: 1 } } .x { animation: spin 1s }";
    let (out, diags) = scope_css(src, "data-lunas-ab12");
    assert!(diags.is_empty());
    assert!(out.contains("@keyframes spin-ab12 "), "{out}");
    assert!(out.contains("animation: spin-ab12 1s"), "{out}");
    // The keyframe selectors `from`/`to` are NOT scoped.
    assert!(out.contains("from { opacity: 0 }"), "{out}");
}

#[test]
fn animation_name_property_rewritten() {
    let src = "@keyframes pulse {} .x { animation-name: pulse }";
    let (out, _) = scope_css(src, "data-lunas-ab12");
    assert!(out.contains("animation-name: pulse-ab12"), "{out}");
}

#[test]
fn animation_shorthand_only_name_token_rewritten() {
    let src = "@keyframes fade {} .x { animation: 2s ease-in fade infinite }";
    let (out, _) = scope_css(src, "data-lunas-ab12");
    assert!(
        out.contains("animation: 2s ease-in fade-ab12 infinite"),
        "{out}"
    );
}

#[test]
fn unrelated_animation_name_untouched() {
    let src = "@keyframes spin {} .x { animation: wobble 1s }";
    let (out, _) = scope_css(src, "data-lunas-ab12");
    assert!(out.contains("animation: wobble 1s"), "{out}");
}

#[test]
fn keyframes_forward_reference() {
    // animation appears before the @keyframes rule — pass one collects the name.
    let src = ".x { animation: spin 1s } @keyframes spin {}";
    let (out, _) = scope_css(src, "data-lunas-ab12");
    assert!(out.contains("animation: spin-ab12 1s"), "{out}");
    assert!(out.contains("@keyframes spin-ab12"), "{out}");
}

#[test]
fn vendor_prefixed_keyframes() {
    let src = "@-webkit-keyframes spin {} .x { animation: spin 1s }";
    let (out, _) = scope_css(src, "data-lunas-ab12");
    assert!(out.contains("@-webkit-keyframes spin-ab12"), "{out}");
    assert!(out.contains("animation: spin-ab12 1s"), "{out}");
}

// --- pass-through at-rules -------------------------------------------------

#[test]
fn font_face_untouched() {
    let src = "@font-face { font-family: 'X'; src: url(x.woff) }";
    assert_eq!(scope(src), src);
}

#[test]
fn import_untouched() {
    let src = "@import url('theme.css');";
    assert_eq!(scope(src), src);
}

#[test]
fn charset_untouched() {
    let src = "@charset \"utf-8\";";
    assert_eq!(scope(src), src);
}

// --- comments --------------------------------------------------------------

#[test]
fn comment_before_selector() {
    let out = scope("/* c */ .a {}");
    assert_eq!(out, "/* c */ .a[data-lunas-x] {}");
}

#[test]
fn comment_inside_selector() {
    let out = scope("a /* x */ b {}");
    assert_eq!(out, "a[data-lunas-x] /* x */ b[data-lunas-x] {}");
}

#[test]
fn comment_with_braces_inside() {
    // Braces inside a comment must not be treated as block delimiters.
    let out = scope("/* } { */ .a {}");
    assert_eq!(out, "/* } { */ .a[data-lunas-x] {}");
}

#[test]
fn comment_in_declaration_block() {
    let out = scope(".a { /* comment */ color: red }");
    assert_eq!(out, ".a[data-lunas-x] { /* comment */ color: red }");
}

// --- unicode ---------------------------------------------------------------

#[test]
fn unicode_class_name() {
    let out = scope(".café {}");
    assert_eq!(out, ".café[data-lunas-x] {}");
}

#[test]
fn unicode_in_content_value() {
    let out = scope(".a { content: '→あ' }");
    assert_eq!(out, ".a[data-lunas-x] { content: '→あ' }");
}

#[test]
fn emoji_in_string() {
    let out = scope(r#".a[data-x="😀"] {}"#);
    assert_eq!(out, r#".a[data-x="😀"][data-lunas-x] {}"#);
}

// --- multiple rules & whitespace -------------------------------------------

#[test]
fn multiple_rules_preserve_formatting() {
    let src = ".a {\n  color: red;\n}\n\n.b {\n  color: blue;\n}\n";
    let out = scope(src);
    assert_eq!(
        out,
        ".a[data-lunas-x] {\n  color: red;\n}\n\n.b[data-lunas-x] {\n  color: blue;\n}\n"
    );
}

#[test]
fn empty_input() {
    assert_eq!(scope(""), "");
}

#[test]
fn whitespace_only() {
    assert_eq!(scope("   \n  "), "   \n  ");
}

#[test]
fn comments_only() {
    assert_eq!(scope("/* just a comment */"), "/* just a comment */");
}

// --- diagnostics on malformed input ----------------------------------------

#[test]
fn unterminated_block_reports_diagnostic() {
    let (out, diags) = scope_css(".a { color: red", S);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("unterminated"));
    // Output still contains the scoped selector.
    assert!(out.starts_with(".a[data-lunas-x] {"), "{out}");
    // The diagnostic range points into the original css.
    let r = diags[0].range;
    assert!(r.start().as_usize() < r.end().as_usize());
    assert!(r.end().as_usize() <= ".a { color: red".len());
}

#[test]
fn rule_without_brace_reports_diagnostic() {
    let (_out, diags) = scope_css(".a .b", S);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("missing `{`"));
}

// --- scope attribute variations --------------------------------------------

#[test]
fn different_scope_attr() {
    let (out, _) = scope_css(".a {}", "data-lunas-deadbeef");
    assert_eq!(out, ".a[data-lunas-deadbeef] {}");
}
