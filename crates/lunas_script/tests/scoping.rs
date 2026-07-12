//! Deep coverage of `free_identifiers` lexical scoping — the reactivity input
//! that must report only the *free* variables an expression/function reads,
//! excluding names bound by params, locals, block scopes, catch clauses, named
//! fn/class expressions, and honoring shadowing at every nesting level.

use lunas_script::{
    free_identifiers, free_identifiers_with_spans, free_identifiers_with_spans_program,
    module_binding_references,
};

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
// `ScopedFreeCollector` opens a scope for a `for` / `for-of` / `for-in` header
// that covers the header AND body: the loop binding is not a free read. Only a
// `VarDecl` init/head binds — a bare-expression C-style init or a bare-pattern
// for-of/for-in head (`for (x of xs)`) assigns an existing binding and stays
// free.

#[test]
fn free_for_of_binding_scoped() {
    // `x` (loop binding) is scoped; only the genuine frees remain, in order.
    assert_eq!(
        free("(function(){ for (const x of items) { total += x } return total })"),
        ["items", "total", "total"]
    );
}

#[test]
fn free_for_in_key_scoped() {
    assert_eq!(
        free("(function(){ for (const k in obj) { use(k) } })"),
        ["obj", "use"]
    );
}

#[test]
fn free_c_style_for_header_binding_scoped() {
    // `i` from the header is scoped everywhere in the loop; `limit` and `sum`
    // are the only frees.
    assert_eq!(
        free("(function(){ for (let i = 0; i < limit; i++) { sum += i } })"),
        ["limit", "sum"]
    );
}

#[test]
fn free_c_style_for_expr_init_not_bound() {
    // A bare-expression init assigns an existing binding — `i` stays free.
    assert_eq!(
        free("(function(){ for (i = 0; i < limit; i++) { sum += i } })"),
        ["i", "i", "limit", "i", "sum", "i"]
    );
}

#[test]
fn free_for_of_destructured_binding_scoped() {
    // Destructured loop bindings (`{ id }`) are scoped; only `rows` is free.
    assert_eq!(
        free("(function(){ for (const { id } of rows) { use(id) } })"),
        ["rows", "use"]
    );
}

#[test]
fn free_for_of_array_destructure_binding_scoped() {
    assert_eq!(
        free("(function(){ for (const [a, b] of pairs) { use(a, b) } })"),
        ["pairs", "use"]
    );
}

#[test]
fn free_nested_loops_shadow() {
    // Both loop variables are scoped; only `outer`, `inner`, `sum` are free.
    assert_eq!(
        free("(function(){ for (const i of outer) { for (const j of inner) { sum += i + j } } })"),
        ["outer", "inner", "sum"]
    );
}

#[test]
fn free_loop_var_shadows_outer_name_read_outside_still_free() {
    // The loop var `x` shadows the outer `x` only inside the loop; the read of
    // `x` after the loop is a genuine free reference and must be reported.
    assert_eq!(
        free("(function(){ for (const x of xs) { use(x) } return x })"),
        ["xs", "use", "x"]
    );
}

// --- catch bindings ---
//
// `ScopedFreeCollector::visit_catch_clause` scopes the catch parameter
// (including destructuring) to the catch body, matching
// `module_binding_references`.

#[test]
fn free_catch_param_scoped() {
    assert_eq!(
        free("(function(){ try { risky() } catch (e) { report(e) } })"),
        ["risky", "report"]
    );
}

#[test]
fn free_catch_outer_same_name_only_outer_free() {
    // The `e` inside catch is bound; the `return e` outside the catch is free.
    assert_eq!(
        free("(function(){ try { f() } catch (e) { e } return e })"),
        ["f", "e"]
    );
}

#[test]
fn free_catch_destructured_param_scoped() {
    assert_eq!(
        free("(function(){ try { f() } catch ({ message }) { log(message) } })"),
        ["f", "log"]
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

// --- agreement with module_binding_references on for/catch forms ---
//
// `free_identifiers` and `module_binding_references` must scope `for` headers
// and `catch` params identically. For a program where every read targets a
// top-level binding, the set of names `free_identifiers_with_spans_program`
// reports must equal the set `module_binding_references` reports.

fn free_program_names(code: &str) -> Vec<String> {
    free_identifiers_with_spans_program(code)
        .expect("parse ok")
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn mref_names(code: &str) -> Vec<String> {
    module_binding_references(code)
        .expect("parse ok")
        .into_iter()
        .map(|r| r.name)
        .collect()
}

#[test]
fn free_agrees_with_mrefs_on_for_of() {
    // The loop binding `x` is scoped in BOTH analyses, so neither reports it.
    // (`module_binding_references` also skips declaration sites and filters to
    // top-level targets, so its list differs; the point is they agree that the
    // loop variable is bound, not free.)
    let code = "let items = []\nlet total = 0\nfor (const x of items) { total += x }";
    let frees = free_program_names(code);
    let mrefs = mref_names(code);
    assert!(!frees.contains(&"x".to_string()), "loop var must be scoped");
    assert!(!mrefs.contains(&"x".to_string()), "loop var must be scoped");
    // `items` and `total` are top-level bindings referenced by both.
    assert!(mrefs.contains(&"items".to_string()));
    assert!(mrefs.contains(&"total".to_string()));
}

#[test]
fn free_agrees_with_mrefs_on_catch() {
    let code = "let report = f\nfunction run(){ try { risky() } catch (e) { report(e) } }";
    // `e` scoped in both; only top-level `report` referenced (risky undeclared).
    assert_eq!(mref_names(code), ["report"]);
    // free_identifiers additionally reports `risky` (a free read, not top-level).
    let frees = free_program_names(code);
    assert!(frees.contains(&"report".to_string()));
    assert!(
        !frees.contains(&"e".to_string()),
        "catch param must be scoped"
    );
}
