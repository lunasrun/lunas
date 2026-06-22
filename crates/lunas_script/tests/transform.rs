//! Tests for the TypeScript-to-JavaScript transform.

use lunas_script::transform_ts_to_js;

#[test]
fn strips_types_keeps_imports() {
    let ts = r#"
        import axios from 'axios';
        interface Args { name: string; }
        function greet(arg: Args): void {
            console.log(`Hello, ${arg.name}!`);
        }
    "#;
    let js = transform_ts_to_js(ts).expect("transform failed");
    assert!(js.contains("function greet("));
    assert!(!js.contains("string"));
    assert!(js.contains("import axios from 'axios';"));
}

#[test]
fn empty_input_ok() {
    assert_eq!(transform_ts_to_js("").expect("ok").trim(), "");
}

#[test]
fn strips_let_type_annotation() {
    let js = transform_ts_to_js("let count: number = 0").expect("ok");
    assert!(!js.contains("number"));
    assert!(js.contains("count"));
}

#[test]
fn invalid_ts_is_error() {
    assert!(transform_ts_to_js("let x: = =").is_err());
}
