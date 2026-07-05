//! Coverage of `function_dependencies` (per top-level function, the free names
//! its body reads — including the names of functions it calls, for transitive
//! expansion) plus `declared_bindings` and `referenced_identifiers` edge cases.

use lunas_script::{declared_bindings, function_dependencies, referenced_identifiers};

fn deps(code: &str) -> Vec<(String, Vec<String>)> {
    function_dependencies(code).expect("parse ok")
}

fn bindings(code: &str) -> Vec<String> {
    declared_bindings(code).expect("parse ok")
}

fn refs(code: &str) -> Vec<String> {
    referenced_identifiers(code).expect("parse ok")
}

// --- function_dependencies: read sets ---

#[test]
fn deps_reads_outer_names() {
    assert_eq!(
        deps("function total(){ return price * qty }"),
        vec![(
            "total".to_string(),
            vec!["price".to_string(), "qty".to_string()]
        )]
    );
}

#[test]
fn deps_excludes_params_and_locals() {
    assert_eq!(
        deps("function f(a){ const b = a + outer; return b }"),
        vec![("f".to_string(), vec!["outer".to_string()])]
    );
}

#[test]
fn deps_includes_called_function_names() {
    // A called function's name is a free read, listed so callers expand deps.
    assert_eq!(
        deps("const f = () => total() + tax"),
        vec![(
            "f".to_string(),
            vec!["total".to_string(), "tax".to_string()]
        )]
    );
}

#[test]
fn deps_dedups_repeated_reads() {
    assert_eq!(
        deps("function f(){ return a + a + b + a }"),
        vec![("f".to_string(), vec!["a".to_string(), "b".to_string()])]
    );
}

#[test]
fn deps_recursive_self_call_lists_own_name() {
    // `function_dependencies` visits the function body directly (not through a
    // block that would bind the decl name), so a recursive self-call IS a free
    // read — `fib` lists itself. A caller expanding to a fixpoint must guard
    // this cycle; the analysis records the name only once (deduped).
    assert_eq!(
        deps("function fib(n){ return n < 2 ? n : fib(n - 1) + fib(n - 2) }"),
        vec![("fib".to_string(), vec!["fib".to_string()])]
    );
}

#[test]
fn deps_transitive_pair_each_lists_callee() {
    // total() reads price/qty; grand() calls total and reads shipping. The two
    // dep sets let a caller expand grand -> total -> {price, qty, shipping}.
    let got = deps(
        "function total(){ return price * qty }\nfunction grand(){ return total() + shipping }",
    );
    assert_eq!(
        got,
        vec![
            (
                "total".to_string(),
                vec!["price".to_string(), "qty".to_string()]
            ),
            (
                "grand".to_string(),
                vec!["total".to_string(), "shipping".to_string()]
            ),
        ]
    );
}

#[test]
fn deps_mutual_recursion_cycle_safe() {
    // isEven/isOdd call each other; each lists the other's name once. A caller
    // expanding to a fixpoint must guard the cycle — the analysis itself never
    // loops (it only records names).
    let got = deps(
        "function isEven(n){ return n === 0 ? true : isOdd(n - 1) }\n\
         function isOdd(n){ return n === 0 ? false : isEven(n - 1) }",
    );
    assert_eq!(
        got,
        vec![
            ("isEven".to_string(), vec!["isOdd".to_string()]),
            ("isOdd".to_string(), vec!["isEven".to_string()]),
        ]
    );
}

#[test]
fn deps_arrow_with_block_body_locals_excluded() {
    assert_eq!(
        deps("const calc = () => { const t = base * rate; return t + fee }"),
        vec![(
            "calc".to_string(),
            vec!["base".to_string(), "rate".to_string(), "fee".to_string()]
        )]
    );
}

#[test]
fn deps_non_callable_const_skipped() {
    // `const x = a + b` is not a function; only `g` gets a dep entry.
    assert_eq!(
        deps("const x = a + b\nfunction g(){ return x + c }"),
        vec![("g".to_string(), vec!["x".to_string(), "c".to_string()])]
    );
}

#[test]
fn deps_exported_function_included() {
    assert_eq!(
        deps("export function label(){ return prefix + name }"),
        vec![(
            "label".to_string(),
            vec!["prefix".to_string(), "name".to_string()]
        )]
    );
}

#[test]
fn deps_nested_arrow_scoping() {
    // The inner arrow's param `x` is bound; `factor` and `xs` are free reads.
    assert_eq!(
        deps("function scale(){ return xs.map(x => x * factor) }"),
        vec![(
            "scale".to_string(),
            vec!["xs".to_string(), "factor".to_string()]
        )]
    );
}

// --- declared_bindings edge cases ---

#[test]
fn bindings_var_let_const_mixed() {
    assert_eq!(bindings("var a\nlet b\nconst c = 1"), ["a", "b", "c"]);
}

#[test]
fn bindings_array_with_holes_and_rest() {
    assert_eq!(bindings("const [a, , b, ...rest] = xs"), ["a", "b", "rest"]);
}

#[test]
fn bindings_object_rename_default_rest() {
    assert_eq!(
        bindings("const { a: x, b = 1, ...others } = obj"),
        ["x", "b", "others"]
    );
}

#[test]
fn bindings_deeply_nested_destructure() {
    assert_eq!(bindings("const { a: { b: [c, { d }] } } = src"), ["c", "d"]);
}

#[test]
fn bindings_array_default_element() {
    assert_eq!(bindings("const [a = 1, b = 2] = xs"), ["a", "b"]);
}

#[test]
fn bindings_default_import_and_namespace() {
    assert_eq!(
        bindings("import def from 'm'\nimport * as ns from 'n'"),
        ["def", "ns"]
    );
}

#[test]
fn bindings_named_imports_with_alias() {
    assert_eq!(
        bindings("import { a, b as c, d } from 'm'"),
        ["a", "c", "d"]
    );
}

#[test]
fn bindings_mixed_default_and_named_import() {
    assert_eq!(bindings("import def, { named } from 'm'"), ["def", "named"]);
}

#[test]
fn bindings_class_and_function_and_export() {
    assert_eq!(
        bindings("export class C {}\nexport function f(){}\nexport const k = 1"),
        ["C", "f", "k"]
    );
}

#[test]
fn bindings_typescript_type_only_declares_nothing() {
    // `collect_decl` only handles var/fn/class, so a TS `enum` (though it is a
    // runtime value) is NOT reported, and type/interface declare nothing. Only
    // the plain `let` binding surfaces.
    assert_eq!(
        bindings("type T = number\ninterface I {}\nenum E { A }\nlet v = 1"),
        ["v"]
    );
}

#[test]
fn bindings_nested_are_not_top_level() {
    assert_eq!(
        bindings("let top = 1\n{ let inner = 2 }\nif (x) { const c = 3 }"),
        ["top"]
    );
}

// --- referenced_identifiers edge cases (flat, no scoping) ---

#[test]
fn refs_optional_chaining_root_only() {
    assert_eq!(refs("a?.b?.c"), ["a"]);
}

#[test]
fn refs_optional_call_and_computed() {
    assert_eq!(refs("a?.[k]?.(x)"), ["a", "k", "x"]);
}

#[test]
fn refs_template_literal_interpolations() {
    assert_eq!(refs("`${a} and ${b.c}`"), ["a", "b"]);
}

#[test]
fn refs_spread_in_call_and_array() {
    assert_eq!(refs("f(...args, [...more])"), ["f", "args", "more"]);
}

#[test]
fn refs_new_expression() {
    assert_eq!(refs("new Ctor(arg)"), ["Ctor", "arg"]);
}

#[test]
fn refs_logical_and_nullish() {
    assert_eq!(refs("a && b || c ?? d"), ["a", "b", "c", "d"]);
}

#[test]
fn refs_arrow_body_flat_includes_param() {
    // referenced_identifiers does NOT scope: the arrow param binding `x` and
    // every body occurrence of `x` are all reported (flat identifier walk).
    assert_eq!(refs("xs.map(x => x + y)"), ["xs", "x", "x", "y"]);
}

#[test]
fn refs_sequence_expression() {
    assert_eq!(refs("(a, b, c)"), ["a", "b", "c"]);
}

#[test]
fn refs_tagged_template() {
    assert_eq!(refs("tag`${x}`"), ["tag", "x"]);
}
