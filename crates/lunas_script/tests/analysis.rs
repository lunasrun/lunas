//! Tests for top-level binding extraction.

use lunas_script::declared_bindings;

fn bindings(code: &str) -> Vec<String> {
    declared_bindings(code).expect("parse ok")
}

#[test]
fn simple_let_const_var() {
    assert_eq!(
        bindings("let a = 1\nconst b = 2\nvar c = 3"),
        ["a", "b", "c"]
    );
}

#[test]
fn multiple_declarators() {
    assert_eq!(bindings("let a = 1, b = 2, c"), ["a", "b", "c"]);
}

#[test]
fn function_and_class() {
    assert_eq!(bindings("function foo(){}\nclass Bar {}"), ["foo", "Bar"]);
}

#[test]
fn array_destructuring() {
    assert_eq!(bindings("const [a, , b] = xs"), ["a", "b"]);
}

#[test]
fn object_destructuring() {
    // {x, y: z, ...rest}
    assert_eq!(
        bindings("const { x, y: z, ...rest } = obj"),
        ["x", "z", "rest"]
    );
}

#[test]
fn nested_and_default_patterns() {
    assert_eq!(
        bindings("const { a: { b }, c = 5, [0]: d } = obj"),
        ["b", "c", "d"]
    );
}

#[test]
fn imports() {
    let got = bindings("import def, { named, other as alias } from 'm'\nimport * as ns from 'n'");
    assert_eq!(got, ["def", "named", "alias", "ns"]);
}

#[test]
fn exported_declarations() {
    assert_eq!(
        bindings("export const a = 1\nexport function f(){}"),
        ["a", "f"]
    );
}

#[test]
fn typescript_is_handled() {
    // Parsed natively as TS; type-only constructs declare no value bindings.
    assert_eq!(
        bindings("interface I {}\ntype T = number\nlet x: T = 0"),
        ["x"]
    );
}

#[test]
fn nested_blocks_are_not_top_level() {
    // Only top-level declarations are reported; inner ones are ignored.
    assert_eq!(
        bindings("let top = 1\nfunction f(){ let inner = 2; }"),
        ["top", "f"]
    );
}

#[test]
fn invalid_is_error() {
    assert!(declared_bindings("let = = =").is_err());
}

// --- referenced_identifiers ---

use lunas_script::referenced_identifiers;

fn refs(code: &str) -> Vec<String> {
    referenced_identifiers(code).expect("parse ok")
}

#[test]
fn refs_simple() {
    assert_eq!(refs("a + b"), ["a", "b"]);
}

#[test]
fn refs_static_member_excludes_property() {
    assert_eq!(refs("a.b.c"), ["a"]);
}

#[test]
fn refs_computed_member_includes_key() {
    assert_eq!(refs("obj[key]"), ["obj", "key"]);
}

#[test]
fn refs_object_literal_keys_excluded_shorthand_included() {
    // {x, y: z} -> x (shorthand read) and z (value), not the key y.
    assert_eq!(refs("({ x, y: z })"), ["x", "z"]);
}

#[test]
fn refs_call_and_args() {
    assert_eq!(refs("f(a, b.c)"), ["f", "a", "b"]);
}

#[test]
fn refs_ternary() {
    assert_eq!(refs("cond ? yes : no"), ["cond", "yes", "no"]);
}

#[test]
fn refs_intersect_with_bindings_for_reactivity() {
    // The orchestrator's pattern: which component bindings does an expr depend on?
    let bound = bindings("let count = 0\nlet other = 1");
    let used = refs("count + helper(other)");
    let reactive: Vec<&String> = used.iter().filter(|u| bound.contains(u)).collect();
    assert_eq!(reactive, [&"count".to_string(), &"other".to_string()]);
}

#[test]
fn refs_invalid_is_error() {
    assert!(referenced_identifiers("= = =").is_err());
}

// --- assigned_identifiers ---

use lunas_script::assigned_identifiers;

fn assigns(code: &str) -> Vec<String> {
    assigned_identifiers(code).expect("parse ok")
}

#[test]
fn assign_simple() {
    assert_eq!(assigns("x = 1"), ["x"]);
}

#[test]
fn assign_compound_and_update() {
    assert_eq!(assigns("x += 1; y -= 2; z++; --w"), ["x", "y", "z", "w"]);
}

#[test]
fn assign_member_reports_root() {
    assert_eq!(assigns("obj.a.b = 1"), ["obj"]);
    assert_eq!(assigns("arr[i] = 1"), ["arr"]);
}

#[test]
fn assign_destructuring() {
    assert_eq!(assigns("[a, b] = pair"), ["a", "b"]);
    assert_eq!(assigns("({ x, y: z } = obj)"), ["x", "z"]);
}

#[test]
fn assign_nested_chain() {
    // a = b = 1 mutates both a and b.
    assert_eq!(assigns("a = b = 1"), ["a", "b"]);
}

#[test]
fn assign_inside_function_body() {
    // A handler that mutates component state.
    let code = "function toggle(){ running = !running; count++ }";
    assert_eq!(assigns(code), ["running", "count"]);
}

#[test]
fn assign_not_triggered_by_equality() {
    // `==`/`===` are comparisons, not assignments.
    assert_eq!(assigns("a == b; c === d"), Vec::<String>::new());
}

#[test]
fn assign_invalid_is_error() {
    assert!(assigned_identifiers("= = =").is_err());
}

// --- free_identifiers (scope-aware) ---

use lunas_script::free_identifiers;

fn free(code: &str) -> Vec<String> {
    free_identifiers(code).expect("parse ok")
}

#[test]
fn free_excludes_arrow_params() {
    assert_eq!(free("items.map(x => x.active)"), ["items"]);
    assert_eq!(free("() => count + 1"), ["count"]);
}

#[test]
fn free_excludes_function_params_and_locals() {
    assert_eq!(free("(function(a){ let b = a; return b + c; })"), ["c"]);
}

#[test]
fn free_keeps_outer_references() {
    // `total` is free; `n` is the arrow param (bound).
    assert_eq!(free("list.reduce((total, n) => total + n)"), ["list"]);
}

#[test]
fn free_destructured_params_excluded() {
    assert_eq!(
        free("data.map(({ id }) => id + suffix)"),
        ["data", "suffix"]
    );
}

#[test]
fn free_invalid_is_error() {
    assert!(free_identifiers("=>").is_err());
}

// --- function_mutations ---

use lunas_script::function_mutations;

#[test]
fn function_mutations_basic() {
    let muts = function_mutations(
        "function add(){ items = items.concat(x); count++ }\nfunction noop(){ return 1 }",
    )
    .unwrap();
    assert_eq!(
        muts,
        vec![
            (
                "add".to_string(),
                vec!["items".to_string(), "count".to_string()]
            ),
            ("noop".to_string(), vec![]),
        ]
    );
}

#[test]
fn function_mutations_arrow_const() {
    let muts = function_mutations("const inc = () => { n++ }").unwrap();
    assert_eq!(muts, vec![("inc".to_string(), vec!["n".to_string()])]);
}

#[test]
fn function_mutations_dedups() {
    let muts = function_mutations("function f(){ a = 1; a = 2; a++ }").unwrap();
    assert_eq!(muts, vec![("f".to_string(), vec!["a".to_string()])]);
}

#[test]
fn function_mutations_ignores_locals_but_keeps_outer() {
    // `local` is a local var; it's still reported (flat analysis), but the
    // important outer `state` mutation is captured.
    let muts = function_mutations("function f(){ let local = 0; state = local }").unwrap();
    assert_eq!(muts[0].0, "f");
    assert!(muts[0].1.contains(&"state".to_string()));
}

#[test]
fn function_mutations_on_real_fixture() {
    // counter-game's functions and what they mutate.
    let path = format!(
        "{}/../lunas_parser/tests/fixtures/counter-game.lun",
        env!("CARGO_MANIFEST_DIR")
    );
    let src = std::fs::read_to_string(&path).expect("read");
    // Extract the script block crudely for this analysis-only check.
    let script = src
        .split("script:")
        .nth(1)
        .unwrap()
        .split("style:")
        .next()
        .unwrap();
    let muts = function_mutations(script).unwrap();
    let by_name = |n: &str| {
        muts.iter()
            .find(|(name, _)| name == n)
            .map(|(_, m)| m.clone())
    };
    assert!(by_name("increment").unwrap().contains(&"count".to_string()));
    assert!(by_name("clear").unwrap().contains(&"count".to_string()));
    assert!(by_name("toggle").unwrap().contains(&"interval".to_string()));
}

// --- analyze_script (single-parse combined) ---

use lunas_script::analyze_script;

#[test]
fn analyze_script_combines_bindings_and_mutations() {
    let a = analyze_script(
        "let count = 0\nlet items = []\nfunction add(){ items = items.concat(1); count++ }",
    )
    .unwrap();
    assert_eq!(a.bindings, ["count", "items", "add"]);
    assert_eq!(
        a.function_mutations,
        vec![(
            "add".to_string(),
            vec!["items".to_string(), "count".to_string()]
        )]
    );
}

#[test]
fn analyze_script_matches_individual_functions() {
    let code = "import x from 'm'\nconst a = 1\nfunction f(){ a2 = 1 }\nconst g = () => { b++ }";
    let combined = analyze_script(code).unwrap();
    assert_eq!(combined.bindings, declared_bindings(code).unwrap());
    assert_eq!(
        combined.function_mutations,
        function_mutations(code).unwrap()
    );
}

// --- referenced_identifiers_with_spans ---

use lunas_script::referenced_identifiers_with_spans;

#[test]
fn refs_with_spans_slice_back() {
    let code = "count + label * count";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["count", "label", "count"]);
    // Every reported range must slice back to its identifier text.
    for (name, range) in &ids {
        assert_eq!(
            range.slice(code),
            Some(name.as_str()),
            "bad span for {name}"
        );
    }
    // The two `count` occurrences have distinct ranges.
    assert_ne!(ids[0].1, ids[2].1);
    assert_eq!(ids[0].1.slice(code), Some("count"));
    assert_eq!(ids[2].1.start().raw(), 16);
}

#[test]
fn refs_with_spans_member_and_call() {
    let code = "a.b ? f(c) : d[e]";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["a", "f", "c", "d", "e"]);
}

#[test]
fn refs_with_spans_unicode() {
    let code = "あ + b";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
}

#[test]
fn refs_with_spans_invalid_is_error() {
    assert!(referenced_identifiers_with_spans("= = =").is_err());
}

// --- declared_bindings_with_spans ---

use lunas_script::declared_bindings_with_spans;

#[test]
fn declared_spans_slice_back() {
    let code = "let count = 0\nconst { x, y: z } = p\nfunction inc(){}\nclass C {}";
    let decls = declared_bindings_with_spans(code).unwrap();
    let names: Vec<_> = decls.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["count", "x", "z", "inc", "C"]);
    for (name, range) in &decls {
        assert_eq!(
            range.slice(code),
            Some(name.as_str()),
            "bad span for {name}"
        );
    }
}

#[test]
fn declared_spans_names_match_declared_bindings() {
    let code = "import d from 'm'\nlet a = 1, b = 2";
    let with: Vec<String> = declared_bindings_with_spans(code)
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(with, declared_bindings(code).unwrap());
}

#[test]
fn declared_spans_invalid_is_error() {
    assert!(declared_bindings_with_spans("let = = =").is_err());
}

// --- free_identifiers_with_spans ---

use lunas_script::free_identifiers_with_spans;

#[test]
fn free_with_spans_excludes_shadows_and_slices_back() {
    let code = "count + items.map(count => count)";
    let ids = free_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    // Proper lexical scoping: the OUTER `count` is free (and renameable); the
    // two inner `count`s are the arrow param and are excluded. `items` is free.
    assert_eq!(names, ["count", "items"]);
    // The reported `count` is the outer one (offset 0), not an inner param.
    assert_eq!(ids[0].1.start().raw(), 0);
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
}

#[test]
fn free_with_spans_keeps_distinct_free_uses() {
    let code = "a + b.f(a)";
    let ids = free_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["a", "b", "a"]);
    assert_ne!(ids[0].1, ids[2].1);
    for (name, range) in &ids {
        assert_eq!(range.slice(code), Some(name.as_str()));
    }
}

#[test]
fn free_with_spans_invalid_is_error() {
    assert!(free_identifiers_with_spans("=>").is_err());
}

#[test]
fn free_lexical_scoping_handles_shadowing() {
    // Proper lexical scoping: the outer `a` is free even when an inner arrow
    // shadows the name; the inner `a` is bound to the param.
    assert_eq!(free("a + (a => a)"), ["a"]);
    assert_eq!(free("a + (b => b)"), ["a"]);
    // A name used only as an inner param is not free.
    assert_eq!(free("xs.map(a => a)"), ["xs"]);
    // Block-scoped local declarations are bound within their block.
    assert_eq!(
        free("(function(){ let local = outer; return local })"),
        ["outer"]
    );
}

// --- lexical scoping edge cases ---

#[test]
fn free_nested_arrow_shadowing() {
    assert_eq!(free("a => (a => a)"), Vec::<String>::new());
    assert_eq!(
        free("outer + (a => (b => a + b + outer))"),
        ["outer", "outer"]
    );
}

#[test]
fn free_block_scope_in_arrow_body() {
    assert_eq!(
        free("() => { const local = dep; return local + 1 }"),
        ["dep"]
    );
}

#[test]
fn free_nested_function_chain() {
    assert_eq!(
        free("function f(a){ return function g(b){ return a + b + c } }"),
        ["c"]
    );
}

#[test]
fn free_does_not_leak_inner_block_binding() {
    assert_eq!(
        free("function f(){ if (cond) { let y = 1 } return y }"),
        ["cond", "y"]
    );
}

#[test]
fn free_destructured_and_default_params_scope() {
    assert_eq!(
        free("xs.map(({ a }, b = fallback) => a + b)"),
        ["xs", "fallback"]
    );
}
