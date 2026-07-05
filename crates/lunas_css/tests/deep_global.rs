//! Edge-focused tests for the `:deep()` / `:global()` escape hatches, and
//! their interaction with selector lists, combinators, and each other.
//! Complements the baseline cases in `tests/scoping.rs`.

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

// --- :deep() ------------------------------------------------------------------

#[test]
fn deep_in_middle_of_chain() {
    // Everything at and after `:deep(...)`'s compound is left unscoped,
    // including compounds that follow it in the chain.
    assert_eq!(
        scope(".a .b:deep(.c) .d {}"),
        ".a[data-lunas-x] .b[data-lunas-x] .c .d {}"
    );
}

#[test]
fn deep_then_child_combinator_unscoped() {
    assert_eq!(scope(".a:deep(.b) > .c {}"), ".a[data-lunas-x] .b > .c {}");
}

#[test]
fn deep_as_first_compound_no_leading_scope() {
    // `:deep(.a)` alone: "before" is empty, so nothing is scoped and no
    // separating space is introduced.
    assert_eq!(scope(":deep(.a) {}"), ".a {}");
}

#[test]
fn deep_with_empty_argument() {
    assert_eq!(scope(".a:deep() {}"), ".a[data-lunas-x] {}");
}

#[test]
fn deep_in_selector_list_scopes_only_before() {
    assert_eq!(
        scope(".a:deep(.b), .c {}"),
        ".a[data-lunas-x] .b, .c[data-lunas-x] {}"
    );
}

#[test]
fn deep_only_affects_its_own_list_entry() {
    assert_eq!(
        scope(".x, .a:deep(.b) {}"),
        ".x[data-lunas-x], .a[data-lunas-x] .b {}"
    );
}

#[test]
fn second_deep_is_left_as_literal_text() {
    // Only the first `:deep()` boundary is honored; once `deep_reached` is
    // set, remaining compounds (including a second `:deep(...)`) are emitted
    // completely untouched.
    assert_eq!(
        scope(".x :deep(.y):deep(.z) {}"),
        ".x[data-lunas-x] .y:deep(.z) {}"
    );
}

#[test]
fn deep_after_attribute_selector() {
    assert_eq!(
        scope(".a[data-x]:deep(.b) {}"),
        ".a[data-x][data-lunas-x] .b {}"
    );
}

#[test]
fn deep_with_multi_word_inner_selector() {
    assert_eq!(
        scope(".a :deep(.b .c > .d) {}"),
        ".a[data-lunas-x] .b .c > .d {}"
    );
}

#[test]
fn deep_without_parens_is_not_special() {
    // `:deep` with no following `(` is just an (invalid) pseudo-class name,
    // not our escape hatch — it gets scoped like any other pseudo.
    assert_eq!(scope(".a:deep {}"), ".a[data-lunas-x]:deep {}");
}

#[test]
fn deep_with_whitespace_before_paren_is_not_recognized() {
    // Whitespace between `:deep` and `(` splits the compound at the space
    // (whitespace always introduces a descendant combinator between units),
    // so `find_deep` never sees `:deep(` as one token here: `:deep` is just
    // scoped as an (invalid) pseudo-class on `.a`, and `(.b)` becomes its own
    // compound, scoped independently as a descendant.
    assert_eq!(
        scope(".a:deep (.b) {}"),
        ".a[data-lunas-x]:deep (.b)[data-lunas-x] {}"
    );
}

// --- :global() ------------------------------------------------------------------

#[test]
fn global_empty_argument() {
    assert_eq!(scope(":global() {}"), " {}");
}

#[test]
fn global_with_internal_comma_kept_as_one_unit() {
    // The comma lives inside `:global(...)`'s parens, so it's part of the
    // argument, not a selector-list separator; the whole thing unwraps to one
    // unscoped chunk.
    assert_eq!(scope(":global(.a, .b) {}"), ".a, .b {}");
}

#[test]
fn multiple_global_wrappers_all_unwrapped() {
    assert_eq!(scope(":global(.a):global(.b) {}"), ".a.b {}");
}

#[test]
fn global_with_combinator_inside() {
    assert_eq!(scope(":global(.a > .b) {}"), ".a > .b {}");
}

#[test]
fn global_mixed_with_deep_unwraps_deep_as_text() {
    // A top-level `:global` anywhere makes the *entire* selector global; any
    // `:deep(...)` present is not specially interpreted, it is just part of
    // the (fully unscoped) selector text outside the global wrapper.
    assert_eq!(scope(":global(.a) :deep(.b) {}"), ".a :deep(.b) {}");
}

#[test]
fn global_after_scoped_prefix_makes_whole_thing_global() {
    assert_eq!(scope(".a :global(.b) {}"), ".a .b {}");
}

#[test]
fn global_prefixed_ident_not_treated_as_global_pseudo() {
    // `:globalfoo(...)` is a longer ident, must not match `:global`.
    assert_eq!(
        scope("a:globalfoo(1) {}"),
        "a[data-lunas-x]:globalfoo(1) {}"
    );
}

#[test]
fn global_list_only_that_entry_is_global() {
    assert_eq!(
        scope(":global(.a), .b, :global(.c) {}"),
        ".a, .b[data-lunas-x], .c {}"
    );
}

#[test]
fn global_with_attribute_selector_inside() {
    assert_eq!(scope(r#":global(a[href^="/"]) {}"#), r#"a[href^="/"] {}"#);
}

#[test]
fn global_with_pseudo_class_inside() {
    assert_eq!(scope(":global(.a:hover) {}"), ".a:hover {}");
}

#[test]
fn global_with_leading_whitespace_in_arg_trimmed() {
    assert_eq!(scope(":global(  .a  ) {}"), ".a {}");
}

#[test]
fn global_with_nested_parens_in_arg() {
    assert_eq!(scope(":global(.a:not(.b)) {}"), ".a:not(.b) {}");
}
