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

#[test]
fn projects_statement_kinds() {
    let code = "\
class C {}
if (a) {}
for (;;) {}
for (const x of xs) {}
for (const k in obj) {}
while (a) {}
function f(){ return 1 }
let v = 1
export const e = 2
export default 3
";
    let ast = parse_to_ast_json(code).expect("ok");
    let kinds: Vec<&str> = ast["body"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["type"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"ClassDeclaration"));
    assert!(kinds.contains(&"IfStatement"));
    assert!(kinds.contains(&"ForStatement"));
    assert!(kinds.contains(&"ForOfStatement"));
    assert!(kinds.contains(&"ForInStatement"));
    assert!(kinds.contains(&"WhileStatement"));
    assert!(kinds.contains(&"FunctionDeclaration"));
    assert!(kinds.contains(&"VariableDeclaration"));
    assert!(kinds.contains(&"ExportDeclaration"));
    assert!(kinds.contains(&"ExportDefaultExpression"));
}

#[test]
fn ast_spans_are_present() {
    let ast = parse_to_ast_json("let x = 1").expect("ok");
    let span = &ast["body"][0]["span"];
    assert!(span["lo"].is_number());
    assert!(span["hi"].is_number());
}
