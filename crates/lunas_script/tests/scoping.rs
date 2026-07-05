//! Deep coverage of `free_identifiers` lexical scoping — the reactivity input
//! that must report only the *free* variables an expression/function reads,
//! excluding names bound by params, locals, block scopes, catch clauses, named
//! fn/class expressions, and honoring shadowing at every nesting level.

use lunas_script::{free_identifiers, free_identifiers_with_spans};

fn free(code: &str) -> Vec<String> {
    free_identifiers(code).expect("parse ok")
}

fn none() -> Vec<String> {
    Vec::<String>::new()
}

// --- arrow / function params ---

#[test]
fn free_single_arrow_param_bound() {
    assert_eq!(free("x => x"), none());
}

#[test]
fn free_arrow_param_outer_still_free() {
    assert_eq!(free("x => x + outer"), ["outer"]);
}

#[test]
fn free_multiple_params_all_bound() {
    assert_eq!(free("(a, b, c) => a + b + c"), none());
}

#[test]
fn free_rest_param_bound() {
    assert_eq!(free("(...xs) => xs.length + n"), ["n"]);
}

#[test]
fn free_function_expr_params_bound() {
    assert_eq!(free("(function(a, b){ return a + b + z })"), ["z"]);
}

// --- shadowing across nesting ---

#[test]
fn free_inner_param_shadows_outer_free_name_reported_once() {
    // Outer `a` is free; the inner arrow's `a` param shadows only inside.
    assert_eq!(free("a + (a => a)"), ["a"]);
}

#[test]
fn free_double_nested_shadow_collapses() {
    // Both levels bind `a`; no free `a` escapes.
    assert_eq!(free("a => (a => a)"), none());
}

#[test]
fn free_outer_visible_through_two_arrows() {
    assert_eq!(
        free("outer + (a => (b => a + b + outer))"),
        ["outer", "outer"]
    );
}

#[test]
fn free_block_let_shadows_outer() {
    // A block-scoped `n` shadows the outer read; only `dep` is free.
    assert_eq!(free("(function(){ let n = dep; return n })"), ["dep"]);
}

#[test]
fn free_inner_const_shadows_param_name() {
    // Param `v` then a block `const v`; nothing leaks, `seed` is free.
    assert_eq!(
        free("(function(v){ { const v = seed; return v } })"),
        ["seed"]
    );
}

#[test]
fn free_function_decl_in_block_shadows() {
    // A nested `function helper` binding is not free; `dep` inside it is.
    assert_eq!(
        free("(function(){ function helper(){ return dep } return helper() })"),
        ["dep"]
    );
}

#[test]
fn free_class_decl_in_block_bound() {
    assert_eq!(
        free("(function(){ class Local { m(){ return base } } return new Local() })"),
        ["base"]
    );
}

// --- for headers ---
//
// KNOWN LIMITATION: `ScopedFreeCollector` does not open a scope for a `for` /
// `for-of` / `for-in` header, so the loop binding leaks and is reported as a
// free read. These tests pin that current contract; if for-header scoping is
// added, update them (the loop variable should then disappear from the set).

#[test]
fn free_for_of_binding_leaks_current_behavior() {
    // `x` (loop binding) leaks in source order alongside the genuine frees.
    assert_eq!(
        free("(function(){ for (const x of items) { total += x } return total })"),
        ["x", "items", "total", "x", "total"]
    );
}

#[test]
fn free_for_in_binding_leaks_current_behavior() {
    assert_eq!(
        free("(function(){ for (const k in obj) { use(k) } })"),
        ["k", "obj", "use", "k"]
    );
}

#[test]
fn free_c_style_for_header_binding_leaks_current_behavior() {
    // `i` from the header is not scoped and leaks at every occurrence.
    assert_eq!(
        free("(function(){ for (let i = 0; i < limit; i++) { sum += i } })"),
        ["i", "i", "limit", "i", "sum", "i"]
    );
}

// --- catch bindings ---
//
// KNOWN LIMITATION: `ScopedFreeCollector` has no `visit_catch_clause`, so a
// catch parameter is NOT bound and leaks as a free read (unlike
// `module_binding_references`, which does scope catch params). Pin the current
// behavior.

#[test]
fn free_catch_param_leaks_current_behavior() {
    assert_eq!(
        free("(function(){ try { risky() } catch (e) { report(e) } })"),
        ["risky", "e", "report", "e"]
    );
}

#[test]
fn free_catch_outer_same_name_all_leak() {
    // Every `e` occurrence is reported (catch param not scoped).
    assert_eq!(
        free("(function(){ try { f() } catch (e) { e } return e })"),
        ["f", "e", "e", "e"]
    );
}

#[test]
fn free_catch_destructured_param_leaks_current_behavior() {
    assert_eq!(
        free("(function(){ try { f() } catch ({ message }) { log(message) } })"),
        ["f", "message", "log", "message"]
    );
}

// --- destructuring params ---

#[test]
fn free_object_destructure_param_bound() {
    assert_eq!(
        free("data.map(({ id }) => id + suffix)"),
        ["data", "suffix"]
    );
}

#[test]
fn free_array_destructure_param_bound() {
    assert_eq!(free("pairs.map(([a, b]) => a + b + c)"), ["pairs", "c"]);
}

#[test]
fn free_nested_destructure_param_bound() {
    assert_eq!(
        free("rows.map(({ user: { name } }) => name + tag)"),
        ["rows", "tag"]
    );
}

#[test]
fn free_rest_in_destructure_param_bound() {
    assert_eq!(
        free("xs.map(({ a, ...rest }) => a + rest.length + k)"),
        ["xs", "k"]
    );
}

// --- default params referencing earlier params / outer names ---

#[test]
fn free_default_param_reads_outer() {
    // The default value expression reads `fallback` (free); `b` param is bound.
    assert_eq!(free("xs.map((b = fallback) => b)"), ["xs", "fallback"]);
}

#[test]
fn free_default_reads_earlier_param() {
    // `a` in `b = a` is an earlier param; it is bound, so only `xs` is free.
    assert_eq!(free("xs.map((a, b = a) => a + b)"), ["xs"]);
}

// --- IIFE ---

#[test]
fn free_iife_locals_bound() {
    // The callee (arrow body) is visited before the argument, so `external`
    // (read in the body) precedes `seed` (the call argument).
    assert_eq!(
        free("((n) => { const twice = n * 2; return twice + external })(seed)"),
        ["external", "seed"]
    );
}

// --- named function / class expression self-reference ---

#[test]
fn free_named_fn_expr_self_ref_bound() {
    // `fact` refers to the fn-expr's own name — bound, not free.
    assert_eq!(
        free("(function fact(n){ return n <= 1 ? 1 : n * fact(n - 1) })"),
        none()
    );
}

#[test]
fn free_named_class_expr_self_ref_bound() {
    assert_eq!(
        free("(class Node { clone(){ return new Node() } })"),
        none()
    );
}

// --- class methods / fields ---

#[test]
fn free_class_method_reads_outer() {
    assert_eq!(
        free("(class C { greet(){ return prefix + name } })"),
        ["prefix", "name"]
    );
}

#[test]
fn free_class_method_param_bound() {
    assert_eq!(free("(class C { add(x){ return x + base } })"), ["base"]);
}

// --- hoisting: a name declared later in a block is still bound throughout ---

#[test]
fn free_hoisted_block_decl_bound_before_use() {
    // `helper` is used before its declaration in source order but is block-
    // scoped-visible, so it must not be reported free.
    assert_eq!(
        free("(function(){ const r = helper(); function helper(){ return dep } return r })"),
        ["dep"]
    );
}

// --- multiple free occurrences preserved in order ---

#[test]
fn free_preserves_order_and_repeats() {
    assert_eq!(free("a + b + a"), ["a", "b", "a"]);
}

// --- with-spans variant: shadowed occurrences excluded, ranges slice back ---

#[test]
fn free_spans_exclude_shadowed_and_slice_back() {
    let code = "total + rows.map(total => total * 2)";
    let ids = free_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    // Only the outer `total` and `rows` are free.
    assert_eq!(names, ["total", "rows"]);
    // The reported `total` is the outer occurrence at offset 0.
    assert_eq!(ids[0].1.start().raw(), 0);
    for (name, range) in &ids {
        assert_eq!(
            range.slice(code),
            Some(name.as_str()),
            "bad span for {name}"
        );
    }
}

#[test]
fn free_spans_nested_free_uses_distinct_ranges() {
    let code = "count + f(count)";
    let ids = free_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["count", "f", "count"]);
    assert_ne!(ids[0].1, ids[2].1);
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
}

#[test]
fn free_spans_unicode_offsets() {
    let code = "αβ + f(γ)";
    let ids = free_identifiers_with_spans(code).unwrap();
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
}

// --- deeply nested closures resolve outer frees ---

#[test]
fn free_deep_closure_resolves_outermost() {
    assert_eq!(free("a => b => c => d => a + b + c + d + e"), ["e"]);
}

#[test]
fn free_sibling_scopes_do_not_leak() {
    // `x` bound in the first arrow must not bind the second arrow's read.
    assert_eq!(free("(x => x) + (() => x)"), ["x"]);
}
