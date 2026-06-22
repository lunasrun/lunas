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
