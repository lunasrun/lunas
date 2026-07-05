//! Deps matrix: cross-checks that the dependency list the *emitter* writes into
//! each `bind(c, [...], …)` (and `forBlock`/`ifBlock`/`setProp` drivers) matches
//! the dependency set the *resolver* computed for the same dynamic. This ties
//! the two phases together: the runtime dispatch mask must equal the resolved
//! `Deps` for representative components.
//!
//! Strategy: resolve a component to read the authoritative index of each
//! reactive var and the resolved `Deps` of each dynamic, then compile the same
//! source and assert the emitted dep-list literal (`[i, j]`) is exactly the
//! sorted resolved indices.

use lunas_compiler::{compile, resolve, DynamicKind, DynamicPart, ResolvedComponent};

fn resolved(src: &str) -> ResolvedComponent {
    let (c, diags) = resolve(src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected errors: {diags:?}"
    );
    c
}

fn emitted(src: &str) -> String {
    let (js, diags) = compile(src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected error diagnostics: {diags:?}"
    );
    js.expect("module emitted")
}

/// Renders a resolved `Deps` as the JS array literal the emitter produces
/// (`[0, 2]`).
fn dep_literal(indices: &[u32]) -> String {
    let mut s = String::from("[");
    for (i, d) in indices.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&d.to_string());
    }
    s.push(']');
    s
}

fn find_dynamic(c: &ResolvedComponent, pred: impl Fn(&DynamicPart) -> bool) -> &DynamicPart {
    c.dynamics.iter().find(|d| pred(d)).expect("dynamic part")
}

// --- text bind dep list matches resolved deps ------------------------------

#[test]
fn text_bind_dep_list_matches_resolved() {
    let src = "\
html:
    <p>${a + b}</p>
script:
    let a = 0
    let b = 0
    let unused = 0
    function f(){ a=1; b=2 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::Text);
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    // The text run's bind lists exactly the resolved indices.
    assert!(
        js.contains(&format!("bind(c, {want}, () => {{ t0.data")),
        "emitted dep list {want} must match resolved for text: {js}"
    );
}

// --- attribute bind dep list matches resolved deps -------------------------

#[test]
fn attribute_bind_dep_list_matches_resolved() {
    let src = "\
html:
    <div :title=\"a + b\"></div>
script:
    let a = 0
    let b = 0
    function f(){ a=1; b=2 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::Attribute("title".into()));
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    assert!(
        js.contains(&format!("bind(c, {want}, () => {{ e0.setAttribute")),
        "attr dep list {want} must match resolved: {js}"
    );
}

// --- :if condition dep list matches resolved deps --------------------------

#[test]
fn if_condition_dep_list_matches_resolved() {
    let src = "\
html:
    <div><span :if=\"open\">x</span></div>
script:
    let open = false
    let noise = 0
    function f(){ open = true; noise = 1 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::IfCondition);
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    assert!(
        js.contains(&format!("ifBlock(c, a0, {want}, () => (open.v)")),
        "ifBlock dep list {want} must match resolved: {js}"
    );
}

// --- :for iterable dep list matches resolved deps --------------------------

#[test]
fn for_iterable_dep_list_matches_resolved() {
    let src = "\
html:
    <ul><li :for=\"item of items\" :key=\"item\">${item}</li></ul>
script:
    let items = []
    let other = 0
    function f(){ items.push(1); other = 1 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::ForIterable);
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    assert!(
        js.contains(&format!("forBlock(c, a0, {want}, () => Array.from")),
        "forBlock dep list {want} must match resolved: {js}"
    );
}

// --- child reactive-prop driving bind dep list matches resolved deps -------

#[test]
fn child_prop_driving_bind_dep_list_matches_resolved() {
    let src = "\
@use Card from \"./Card.lunas\"
html:
    <div><Card :count=\"a + b\"/></div>
script:
    let a = 0
    let b = 0
    function f(){ a=1; b=2 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::Attribute("count".into()));
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    assert!(
        js.contains(&format!("bind(c, {want}, () => {{ ch0.setProp(\"count\"")),
        "setProp driving bind {want} must match resolved: {js}"
    );
}

// --- transitive-through-function deps agree end to end ---------------------

#[test]
fn transitive_function_deps_agree_across_phases() {
    let src = "\
html:
    <p>${total()}</p>
script:
    let price = 1
    let qty = 2
    let tax = 0
    function total(){ return price * qty + tax }
    function sp(){ price = 3 }
    function sq(){ qty = 4 }
    function st(){ tax = 1 }
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::Text);
    // Resolver should have found all three transitively.
    assert_eq!(part.deps.indices().len(), 3, "price, qty, tax");
    let want = dep_literal(part.deps.indices());
    let js = emitted(src);
    assert!(
        js.contains(&format!("bind(c, {want}, () => {{ t0.data")),
        "transitive dep list {want} must match resolved: {js}"
    );
}

// --- shadowed loop bindings are dropped from the emitted dep list ----------

#[test]
fn loop_shadowed_var_dropped_from_emitted_deps() {
    // Inside the item, a read of the loop binding `x` must NOT contribute a
    // reactive dep (it is item-coupled); a read of an outer reactive var still
    // does.
    let src = "\
html:
    <ul><li :for=\"x of xs\" :key=\"x\">${x + outer}</li></ul>
script:
    let xs = []
    let outer = 0
    function f(){ xs.push(1); outer = 1 }
";
    let c = resolved(src);
    // `xs` is index 0, `outer` is index 1 (both mutated, in decl order).
    assert_eq!(c.reactive_index("xs"), Some(0));
    assert_eq!(c.reactive_index("outer"), Some(1));
    let js = emitted(src);
    // The item text bind should depend on `outer` (index 1) only; `x` is a loop
    // binding, not a reactive var.
    assert!(
        js.contains("bind(c, [1], () => { t0.data = `${x + outer.v}`; });"),
        "loop binding excluded, outer var kept: {js}"
    );
}

// --- empty dep list for a purely-static dynamic ----------------------------

#[test]
fn static_interpolation_has_empty_deps_and_no_bind() {
    let src = "\
html:
    <p>${K}</p>
script:
    const K = 7
";
    let c = resolved(src);
    let part = find_dynamic(&c, |d| d.kind == DynamicKind::Text);
    assert!(part.deps.is_empty());
    let js = emitted(src);
    // No reactive deps -> a plain build-time statement, no bind wrapper.
    assert!(js.contains("t0.data = `${K}`;"), "{js}");
    assert!(!js.contains("bind("), "no bind for static dynamic: {js}");
}
