//! Coverage of `transform_ts_to_js` (TypeScript type-stripping → JS) and
//! `parse_to_ast_json` (shallow span-annotated AST projection). The transform
//! must produce valid JS with all type-only syntax removed; the AST projection
//! must classify top-level statement kinds and expose byte spans.

use lunas_script::{parse_to_ast_json, transform_ts_to_js};

fn js(ts: &str) -> String {
    transform_ts_to_js(ts).expect("transform ok")
}

// --- type-only syntax stripped ---

#[test]
fn ts_strips_interface_and_type_alias() {
    let out = js("interface I { x: number }\ntype T = I | null\nconst v = 1");
    assert!(!out.contains("interface"));
    assert!(!out.contains("type T"));
    assert!(out.contains("v"));
}

#[test]
fn ts_strips_param_and_return_annotations() {
    let out = js("function add(a: number, b: number): number { return a + b }");
    assert!(out.contains("function add("));
    assert!(!out.contains("number"));
}

#[test]
fn ts_strips_generics() {
    let out = js("function id<T>(x: T): T { return x }");
    assert!(out.contains("function id("));
    assert!(!out.contains("<T>"));
}

#[test]
fn ts_strips_as_cast() {
    let out = js("const n = value as number");
    assert!(!out.contains("as number"));
    assert!(out.contains("value"));
}

#[test]
fn ts_strips_satisfies() {
    let out = js("const cfg = { a: 1 } satisfies Record<string, number>");
    assert!(!out.contains("satisfies"));
    assert!(out.contains("a: 1") || out.contains("a:1"));
}

#[test]
fn ts_strips_as_const() {
    let out = js("const x = [1, 2] as const");
    assert!(!out.contains("as const"));
    assert!(out.contains("1"));
}

#[test]
fn ts_strips_non_null_assertion() {
    let out = js("const y = a!.b");
    assert!(!out.contains("!."));
    assert!(out.contains("a.b") || out.contains("a .b"));
}

#[test]
fn ts_strips_definite_assignment() {
    let out = js("let x!: number");
    assert!(!out.contains("number"));
    assert!(!out.contains("!"));
    assert!(out.contains("x"));
}

#[test]
fn ts_strips_optional_param_marker() {
    let out = js("function f(a?: number){ return a }");
    assert!(!out.contains("?"));
    assert!(!out.contains("number"));
}

#[test]
fn ts_strips_readonly_and_index_signature() {
    let out = js("interface I { readonly x: number; [k: string]: unknown }\nconst v = 1");
    assert!(!out.contains("readonly"));
    assert!(!out.contains("interface"));
    assert!(out.contains("v"));
}

#[test]
fn ts_strips_declare() {
    let out = js("declare const g: number\nconst u = 1");
    assert!(!out.contains("declare"));
    assert!(out.contains("u"));
}

// --- type-only imports / exports ---

#[test]
fn ts_strips_type_only_import() {
    let out = js("import type { Foo } from 'm'\nimport { bar } from 'n'\nconst x: Foo = bar");
    assert!(!out.contains("import type"));
    assert!(out.contains("bar"));
    assert!(!out.contains(": Foo"));
}

#[test]
fn ts_strips_type_only_export() {
    let out = js("export type { Foo } from 'm'\nexport { bar } from 'n'");
    assert!(!out.contains("export type"));
    assert!(out.contains("bar"));
}

#[test]
fn ts_strips_inline_type_import_specifier() {
    let out = js("import { type A, b } from 'm'\nconst x = b");
    assert!(!out.contains("type A"));
    assert!(out.contains("b"));
}

// --- constructs that lower to runtime code ---

#[test]
fn ts_enum_lowers_to_runtime_object() {
    let out = js("enum Color { Red, Green }\nconst c = Color.Red");
    assert!(!out.contains("enum"));
    // The enum becomes an IIFE-style runtime object referencing `Color`.
    assert!(out.contains("Color"));
}

#[test]
fn ts_namespace_lowers_to_runtime() {
    let out = js("namespace NS { export const x = 1 }\nconst y = NS.x");
    assert!(!out.contains("namespace"));
    assert!(out.contains("NS"));
}

#[test]
fn ts_constructor_param_properties_expanded() {
    // `private x: number` param property becomes a field + `this.x = x`.
    let out = js("class C { constructor(private x: number){} }");
    assert!(!out.contains("private"));
    assert!(out.contains("this.x = x"));
}

#[test]
fn ts_abstract_class_keyword_stripped() {
    let out = js("abstract class A { abstract m(): void {} }");
    assert!(!out.contains("abstract"));
    assert!(out.contains("class A"));
}

#[test]
fn ts_import_equals_lowers_to_require() {
    let out = js("import M = require('m')\nconst v = M");
    assert!(!out.contains("import M ="));
    assert!(out.contains("require('m')"));
}

// --- preserved runtime syntax ---

#[test]
fn ts_preserves_optional_chaining_and_nullish() {
    let out = js("const y = a?.b ?? c");
    assert!(out.contains("?."));
    assert!(out.contains("??"));
}

#[test]
fn ts_preserves_value_imports() {
    let out = js("import axios from 'axios'\nconst r = axios");
    assert!(out.contains("import axios from 'axios'"));
}

#[test]
fn ts_preserves_template_literals() {
    let out = js("const s: string = `hi ${name}`");
    assert!(out.contains("`hi ${name}`"));
    assert!(!out.contains(": string"));
}

// --- transform edge cases ---

#[test]
fn ts_empty_input_is_empty() {
    assert_eq!(transform_ts_to_js("").expect("ok").trim(), "");
}

#[test]
fn ts_plain_js_passthrough() {
    let out = js("const a = 1\nfunction f(){ return a }");
    assert!(out.contains("const a = 1"));
    assert!(out.contains("function f("));
}

#[test]
fn ts_invalid_is_error_not_panic() {
    assert!(transform_ts_to_js("let x: = =").is_err());
}

#[test]
fn ts_decorators_unsupported_are_error() {
    // The parser's default TS syntax disables decorators; a decorated member is
    // a parse error rather than a panic (never-panic contract).
    assert!(transform_ts_to_js("class C { @dec method(){} }").is_err());
}

// --- parse_to_ast_json ---

#[test]
fn ast_json_top_level_kinds() {
    let ast = parse_to_ast_json(
        "import a from 'm'\nlet x = 1\nfunction f(){}\nclass C {}\nexport const y = 2",
    )
    .unwrap();
    let body = ast["body"].as_array().unwrap();
    let kinds: Vec<&str> = body.iter().map(|n| n["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "ImportDeclaration",
            "VariableDeclaration",
            "FunctionDeclaration",
            "ClassDeclaration",
            "ExportDeclaration",
        ]
    );
    assert_eq!(ast["type"], "Module");
}

#[test]
fn ast_json_control_flow_kinds() {
    let ast = parse_to_ast_json(
        "if (a) {}\nfor (const i of xs) {}\nfor (const k in o) {}\nwhile (b) {}\nreturn 1",
    )
    .unwrap();
    let body = ast["body"].as_array().unwrap();
    let kinds: Vec<&str> = body.iter().map(|n| n["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "IfStatement",
            "ForOfStatement",
            "ForInStatement",
            "WhileStatement",
            "ReturnStatement",
        ]
    );
}

#[test]
fn ast_json_spans_are_ordered_and_present() {
    let ast = parse_to_ast_json("let a = 1\nlet b = 2").unwrap();
    let body = ast["body"].as_array().unwrap();
    assert_eq!(body.len(), 2);
    let lo0 = body[0]["span"]["lo"].as_u64().unwrap();
    let hi0 = body[0]["span"]["hi"].as_u64().unwrap();
    let lo1 = body[1]["span"]["lo"].as_u64().unwrap();
    assert!(lo0 < hi0);
    assert!(hi0 <= lo1, "spans should not overlap and be ordered");
}

#[test]
fn ast_json_typescript_parsed_natively() {
    // TS is parsed without stripping first; type-only items still project.
    let ast = parse_to_ast_json("interface I {}\nlet x: number = 0").unwrap();
    let body = ast["body"].as_array().unwrap();
    // The `let` declaration is present regardless of the interface classification.
    assert!(body.iter().any(|n| n["type"] == "VariableDeclaration"));
}

#[test]
fn ast_json_empty_module() {
    let ast = parse_to_ast_json("").unwrap();
    assert_eq!(ast["body"].as_array().unwrap().len(), 0);
}

#[test]
fn ast_json_invalid_is_error() {
    assert!(parse_to_ast_json("let = = =").is_err());
}
