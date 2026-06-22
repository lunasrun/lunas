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
