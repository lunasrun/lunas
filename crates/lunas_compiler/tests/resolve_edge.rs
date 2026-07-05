//! Edge-case coverage for the resolution layer (`resolve`): reactive-var
//! numbering (index == bit), dependency-set computation per dynamic (transitive
//! through function calls, cycle-safe), handler write-sets, props/imports
//! tables, `Deps::mask_u128`, and never-panic-on-malformed guarantees.
//!
//! These complement `resolve.rs` and `reactivity.rs` with a broader, more
//! adversarial matrix; they assert structural properties of the resolved model
//! rather than emitted JS.

use lunas_compiler::{resolve, DynamicKind, DynamicPart, ResolvedComponent};

fn ok(src: &str) -> ResolvedComponent {
    let (c, diags) = resolve(src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected errors: {diags:?}"
    );
    c
}

fn idx(c: &ResolvedComponent, name: &str) -> u32 {
    c.reactive_index(name).expect("reactive var")
}

/// The dependency indices of the first dynamic part matching `pred`.
fn deps_of(c: &ResolvedComponent, pred: impl Fn(&DynamicPart) -> bool) -> Vec<u32> {
    c.dynamics
        .iter()
        .find(|d| pred(d))
        .expect("a matching dynamic part")
        .deps
        .indices()
        .to_vec()
}

// --- reactive-var numbering: index == bit position -------------------------

#[test]
fn index_equals_bit_via_mask() {
    // Reactive index i must equal bit i in a single-var dep mask.
    let c = ok("\
html:
    <p>${a}${b}${d}</p>
script:
    let a = 0
    let b = 0
    let c = 0
    let d = 0
    function f(){ a=1; b=2; d=3 }
");
    // c is never mutated -> not reactive; a,b,d numbered 0,1,2 in decl order.
    assert_eq!(idx(&c, "a"), 0);
    assert_eq!(idx(&c, "b"), 1);
    assert_eq!(idx(&c, "d"), 2);
    assert_eq!(c.reactive_index("c"), None);
    // Each single-var text bind's mask is exactly 1<<index.
    for (name, expect) in [("a", 0b001u128), ("b", 0b010), ("d", 0b100)] {
        let part = c
            .dynamics
            .iter()
            .find(|p| p.kind == DynamicKind::Text && p.expr == name)
            .unwrap();
        assert_eq!(part.deps.mask_u128(), Some(expect), "{name}");
        assert_eq!(part.deps.mask_u128(), Some(1u128 << idx(&c, name)));
    }
}

#[test]
fn numbering_is_declaration_order_not_mutation_order() {
    // Mutated in reverse order, but numbered in declaration order.
    let c = ok("\
html:
    <p>${first}${second}</p>
script:
    let first = 0
    let second = 0
    function f(){ second = 1; first = 2 }
");
    assert_eq!(idx(&c, "first"), 0);
    assert_eq!(idx(&c, "second"), 1);
}

#[test]
fn declared_but_never_mutated_is_not_reactive() {
    let c = ok("\
html:
    <p>${k}</p>
script:
    const k = 1
");
    assert!(!c.is_reactive("k"));
    assert!(c.reactive_vars.is_empty());
}

#[test]
fn top_level_reassignment_makes_reactive() {
    let c = ok("\
html:
    <p>${x}</p>
script:
    let x = 0
    x = x + 1
");
    assert!(c.is_reactive("x"));
    assert_eq!(idx(&c, "x"), 0);
}

#[test]
fn compound_and_increment_operators_mark_reactive() {
    for op in ["v += 1", "v -= 1", "v *= 2", "v++", "--v", "v **= 2"] {
        let src = format!(
            "html:\n    <p>${{v}}</p>\nscript:\n    let v = 1\n    function f(){{ {op} }}\n"
        );
        let c = ok(&src);
        assert!(c.is_reactive("v"), "op `{op}` should make v reactive");
    }
}

// --- props reactive + numbered after script vars ---------------------------

#[test]
fn props_numbered_after_script_vars_in_input_order() {
    let c = ok("\
@input a:number = 0
@input b:number = 0
html:
    <p>${s}${a}${b}</p>
script:
    let s = 0
    function f(){ s = 1 }
");
    // s (script) is 0; props a,b follow in @input order.
    assert_eq!(idx(&c, "s"), 0);
    assert_eq!(idx(&c, "a"), 1);
    assert_eq!(idx(&c, "b"), 2);
    assert!(c.is_reactive("a") && c.is_reactive("b"));
}

#[test]
fn prop_also_declared_in_script_is_not_double_numbered() {
    // `start` is both an @input prop and re-bound in script; it gets ONE index.
    let c = ok("\
@input start:number = 0
html:
    <p>${count}${start}</p>
script:
    let count = start
    function f(){ count = 1 }
");
    // count is reactive var 0. start is a prop -> reactive, appended after.
    assert_eq!(idx(&c, "count"), 0);
    assert_eq!(idx(&c, "start"), 1);
    // Exactly two reactive vars, no duplicate `start`.
    let starts = c.reactive_vars.iter().filter(|v| v.name == "start").count();
    assert_eq!(starts, 1);
}

#[test]
fn prop_that_is_script_reactive_var_keeps_single_index() {
    // A prop whose name IS a script-mutated binding: not appended again.
    let c = ok("\
@input n:number = 0
html:
    <p>${n}</p>
script:
    let n = 0
    function f(){ n = 1 }
");
    // `n` is reactive from script (index 0); the prop must not add a second.
    assert_eq!(c.reactive_vars.len(), 1);
    assert_eq!(idx(&c, "n"), 0);
}

#[test]
fn prop_read_in_template_has_prop_index_as_dep() {
    let c = ok("\
@input label:string = \"\"
html:
    <p>${label}</p>
");
    let deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    assert_eq!(deps, vec![idx(&c, "label")]);
}

// --- imports / props tables ------------------------------------------------

#[test]
fn imports_table_preserves_source_order_and_paths() {
    let c = ok("\
@use First from \"./First.lunas\"
@use Second from \"./dir/Second.lunas\"
html:
    <p>hi</p>
");
    assert_eq!(c.imports.len(), 2);
    assert_eq!(c.imports[0].component_name, "First");
    assert_eq!(c.imports[0].path, "./First.lunas");
    assert_eq!(c.imports[1].component_name, "Second");
    assert_eq!(c.imports[1].path, "./dir/Second.lunas");
}

#[test]
fn props_table_preserves_source_order() {
    let c = ok("\
@input alpha:number = 1
@input beta:string = \"x\"
html:
    <p>hi</p>
");
    assert_eq!(c.props.len(), 2);
    assert_eq!(c.props[0].name, "alpha");
    assert_eq!(c.props[1].name, "beta");
}

// --- two-way write-back makes an otherwise-unmutated var reactive ----------

#[test]
fn two_way_binding_makes_lvalue_reactive_without_script_mutation() {
    // The script never assigns `name`, but `::value="name"` writes it back.
    let c = ok("\
html:
    <input ::value=\"name\">
script:
    let name = \"a\"
");
    assert!(
        c.is_reactive("name"),
        "two-way write-back should number `name` reactive"
    );
}

#[test]
fn two_way_member_lvalue_makes_root_reactive() {
    // `::value="o.k"` deep-writes `o`, so its root becomes reactive.
    let c = ok("\
html:
    <input ::value=\"o.k\">
script:
    let o = {}
");
    assert!(c.is_reactive("o"));
}

#[test]
fn ref_binding_makes_target_reactive() {
    // `:ref="el"` assigns the element into `el`, a mutation.
    let c = ok("\
html:
    <input :ref=\"el\">
script:
    let el
");
    assert!(c.is_reactive("el"));
}

// --- transitive dependency computation -------------------------------------

#[test]
fn transitive_reads_through_two_hops() {
    let c = ok("\
html:
    <p>${outer()}</p>
script:
    let a = 1
    let b = 2
    function outer(){ return inner() + a }
    function inner(){ return b }
    function seta(){ a = 9 }
    function setb(){ b = 9 }
");
    let mut deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    deps.sort_unstable();
    let mut expected = vec![idx(&c, "a"), idx(&c, "b")];
    expected.sort_unstable();
    assert_eq!(deps, expected, "reads through outer->inner reach a and b");
}

#[test]
fn cyclic_function_reads_are_cycle_safe() {
    // Mutually recursive read graph must not loop forever and must still find
    // every reactive var read along the cycle.
    let c = ok("\
html:
    <p>${ping()}</p>
script:
    let a = 1
    let b = 2
    function ping(){ return a + pong() }
    function pong(){ return b + ping() }
    function sa(){ a = 0 }
    function sb(){ b = 0 }
");
    let mut deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    deps.sort_unstable();
    let mut expected = vec![idx(&c, "a"), idx(&c, "b")];
    expected.sort_unstable();
    assert_eq!(deps, expected);
}

#[test]
fn handler_write_set_transitive_and_cycle_safe() {
    let c = ok("\
html:
    <button @click=\"go()\">${a}${b}</button>
script:
    let a = 0
    let b = 0
    function go(){ a = 1; more() }
    function more(){ b = 2; go() }
");
    assert_eq!(c.handlers.len(), 1);
    let mut writes = c.handlers[0].writes.indices().to_vec();
    writes.sort_unstable();
    let mut expected = vec![idx(&c, "a"), idx(&c, "b")];
    expected.sort_unstable();
    assert_eq!(writes, expected);
}

#[test]
fn handler_direct_assignment_write_set() {
    let c = ok("\
html:
    <button @click=\"n = n + 1\">x</button>
script:
    let n = 0
    function seed(){ n = 0 }
");
    assert_eq!(c.handlers.len(), 1);
    assert_eq!(c.handlers[0].writes.indices(), &[idx(&c, "n")]);
}

#[test]
fn handler_reading_but_not_writing_has_empty_write_set() {
    let c = ok("\
html:
    <button @click=\"log(n)\">x</button>
script:
    let n = 0
    function bump(){ n++ }
    function log(x){ return x }
");
    // The handler only reads `n`; it writes nothing.
    assert!(c.handlers[0].writes.is_empty());
}

#[test]
fn multiple_handlers_have_independent_write_sets() {
    let c = ok("\
html:
    <div>
        <button @click=\"seta()\">a</button>
        <button @click=\"setb()\">b</button>
    </div>
script:
    let a = 0
    let b = 0
    function seta(){ a = 1 }
    function setb(){ b = 1 }
");
    assert_eq!(c.handlers.len(), 2);
    let by_event = |ev: &str| {
        c.handlers
            .iter()
            .find(|h| h.event == "click" && h.handler.contains(ev))
            .unwrap()
    };
    assert_eq!(by_event("seta").writes.indices(), &[idx(&c, "a")]);
    assert_eq!(by_event("setb").writes.indices(), &[idx(&c, "b")]);
}

// --- dep sets on each dynamic kind ----------------------------------------

#[test]
fn every_dynamic_kind_is_recorded_with_deps() {
    let c = ok("\
html:
    <div :title=\"t\" class=\"a ${t} b\">
        <input :value=\"v\" ::checked=\"flag\">
        <span :if=\"open\">x</span>
        <li :for=\"i of items\">${i}</li>
        text ${t}
    </div>
script:
    let t = \"\"
    let v = \"\"
    let flag = false
    let open = false
    let items = []
    function f(){ t=\"z\"; v=\"z\"; flag=true; open=true; items=[1] }
");
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::Attribute("title".into())),
        vec![idx(&c, "t")]
    );
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::AttributeText("class".into())),
        vec![idx(&c, "t")]
    );
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::Attribute("value".into())),
        vec![idx(&c, "v")]
    );
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::TwoWay("checked".into())),
        vec![idx(&c, "flag")]
    );
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::IfCondition),
        vec![idx(&c, "open")]
    );
    assert_eq!(
        deps_of(&c, |d| d.kind == DynamicKind::ForIterable),
        vec![idx(&c, "items")]
    );
}

#[test]
fn expression_reading_two_vars_has_both_deps() {
    let c = ok("\
html:
    <p>${a + b}</p>
script:
    let a = 0
    let b = 0
    function f(){ a=1; b=2 }
");
    let mut deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    deps.sort_unstable();
    assert_eq!(deps, vec![idx(&c, "a"), idx(&c, "b")]);
}

#[test]
fn non_reactive_reads_produce_empty_deps() {
    let c = ok("\
html:
    <p>${K + 1}</p>
script:
    const K = 10
");
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::Text)
        .unwrap();
    assert!(part.deps.is_empty());
}

// --- Deps::mask_u128 -------------------------------------------------------

#[test]
fn mask_combines_bits() {
    let c = ok("\
html:
    <p>${a + b + d}</p>
script:
    let a = 0
    let b = 0
    let d = 0
    function f(){ a=1; b=1; d=1 }
");
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::Text)
        .unwrap();
    let expected = (1u128 << idx(&c, "a")) | (1u128 << idx(&c, "b")) | (1u128 << idx(&c, "d"));
    assert_eq!(part.deps.mask_u128(), Some(expected));
    assert_eq!(part.deps.mask_u128(), Some(0b111));
}

#[test]
fn empty_deps_mask_is_zero() {
    let c = ok("\
html:
    <p>${K}</p>
script:
    const K = 1
");
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::Text)
        .unwrap();
    assert_eq!(part.deps.mask_u128(), Some(0));
}

#[test]
fn deps_indices_are_sorted_and_unique() {
    // Expression reads b then a then b again: indices come back sorted+deduped.
    let c = ok("\
html:
    <p>${b + a + b}</p>
script:
    let a = 0
    let b = 0
    function f(){ a=1; b=1 }
");
    let deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    let mut sorted = deps.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(deps, sorted, "indices must be sorted and unique: {deps:?}");
}

#[test]
fn deps_contains_reports_membership() {
    let c = ok("\
html:
    <p>${a}</p>
script:
    let a = 0
    let b = 0
    function f(){ a=1; b=1 }
");
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::Text)
        .unwrap();
    assert!(part.deps.contains(idx(&c, "a")));
    assert!(!part.deps.contains(idx(&c, "b")));
}

// --- decl ranges -----------------------------------------------------------

#[test]
fn decl_range_slices_to_binding_name() {
    let src = "\
html:
    <p>${count}</p>
script:
    let count = 0
    function f(){ count++ }
";
    let c = ok(src);
    let v = c.reactive_vars.iter().find(|v| v.name == "count").unwrap();
    assert_eq!(v.decl_range.unwrap().slice(src), Some("count"));
}

#[test]
fn prop_reactive_var_has_decl_range() {
    let c = ok("\
@input title:string = \"\"
html:
    <p>${title}</p>
");
    let v = c.reactive_vars.iter().find(|v| v.name == "title").unwrap();
    assert!(v.decl_range.is_some(), "prop reactive var carries a range");
}

// --- never-panic on malformed sources, with diagnostics --------------------

#[test]
fn malformed_never_panics_matrix() {
    let cases = [
        "",
        "html:",
        "script:",
        "@input",
        "@use",
        "@input\n@use\nscript:\n    let",
        "html:\n    <p>${</p>",
        "html:\n    <p>${}</p>",
        "html:\n    <p>${()()()}</p>",
        "html:\n    <div :a=\"[[[\" :b=\"}}}\" @c=\";;;\">${...}</div>",
        "html:\n    <li :for=\"of of of\">x</li>",
        "html:\n    <li :for=\"\" :key=\"\">x</li>",
        "html:\n    <input ::=\"\">",
        "html:\n    <input ::value=\"a.b[c].d\">\nscript:\n    let a={}",
        "script:\n    let a = a = a = 1\nhtml:\n    <p>${a}</p>",
        "html:\n    <p>${日本語 + \u{1f980}}</p>\nscript:\n    let 日本語 = 0\n    function f(){ 日本語=1 }",
        "@input x\n@input x\nhtml:\n    <p>${x}</p>",
        "html:\n    <p>${a}</p>\nscript:\n    function r(){ return r() }",
        "html:\n    <div>\u{0}\u{7}</div>",
        "html:\n    <p>${a ? b : c ? d : e}</p>\nscript:\n    let a=0",
    ];
    for case in cases {
        let (_c, _d) = resolve(case);
    }
}

#[test]
fn script_parse_error_reports_diagnostic_and_recovers() {
    let (c, diags) = resolve("html:\n    <p>hi</p>\nscript:\n    let = = =\n");
    assert!(
        diags.iter().any(|d| d.is_error()),
        "a broken script yields an error diagnostic"
    );
    // The component still resolves (template intact, no reactive vars).
    assert!(c.template.is_some());
    assert!(c.reactive_vars.is_empty());
}

#[test]
fn no_html_means_no_dynamics_or_handlers() {
    let c = ok("script:\n    let x = 0\n    function f(){ x = 1 }\n");
    assert!(c.dynamics.is_empty());
    assert!(c.handlers.is_empty());
    assert!(c.template.is_none());
    // The reactive var is still numbered from the script.
    assert!(c.is_reactive("x"));
}
