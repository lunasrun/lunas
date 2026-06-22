//! Tests for the script AST parser.

use lunas_script::parse_to_ast_json;

#[test]
fn parses_simple_js() {
    let ast = parse_to_ast_json("let count = 0;").expect("parse ok");
    assert_eq!(ast["type"], "Module");
    assert!(ast["body"].is_array());
    assert_eq!(ast["body"][0]["type"], "VariableDeclaration");
}

#[test]
fn empty_input_is_ok() {
    let ast = parse_to_ast_json("").expect("parse ok");
    assert_eq!(ast["type"], "Module");
    assert_eq!(ast["body"].as_array().expect("array").len(), 0);
}

#[test]
fn captures_function_and_import() {
    let ast = parse_to_ast_json("import x from 'y';\nfunction f(){}").expect("ok");
    assert_eq!(ast["body"][0]["type"], "ImportDeclaration");
    assert_eq!(ast["body"][1]["type"], "FunctionDeclaration");
}

#[test]
fn parses_typescript_natively_without_stripping() {
    // The AST parser accepts TypeScript directly — no TS->JS conversion first.
    let ast = parse_to_ast_json("let count: number = 0\ninterface A { x: number }").expect("ok");
    assert_eq!(ast["type"], "Module");
    assert_eq!(ast["body"][0]["type"], "VariableDeclaration");
}

#[test]
fn invalid_js_is_error() {
    assert!(parse_to_ast_json("let = = =").is_err());
}
