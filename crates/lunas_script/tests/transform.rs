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

#[test]
fn strips_enum() {
    let js = transform_ts_to_js("enum E { A, B }\nlet x = E.A").expect("ok");
    assert!(js.contains("E"));
    assert!(!js.contains("enum"));
}

#[test]
fn strips_generics_and_casts() {
    let ts = "function id<T>(x: T): T { return x }\nlet n = id<number>(1) as number";
    let js = transform_ts_to_js(ts).expect("ok");
    assert!(js.contains("function id("));
    assert!(!js.contains("<T>"));
    assert!(!js.contains("as number"));
}

#[test]
fn strips_type_only_import_and_annotations() {
    let ts = "import type { Foo } from 'm'\nimport { bar } from 'n'\nlet x: Foo = bar";
    let js = transform_ts_to_js(ts).expect("ok");
    assert!(!js.contains("import type"));
    assert!(js.contains("bar"));
    assert!(!js.contains(": Foo"));
}

#[test]
fn keeps_optional_chaining_and_nullish() {
    let js = transform_ts_to_js("let y = a?.b ?? c").expect("ok");
    assert!(js.contains("?."));
    assert!(js.contains("??"));
}

#[test]
fn strips_interface_and_type_alias() {
    let js =
        transform_ts_to_js("interface I { x: number }\ntype T = I | null\nlet v = 1").expect("ok");
    assert!(!js.contains("interface"));
    assert!(!js.contains("type T"));
    assert!(js.contains("v"));
}

#[test]
fn strips_non_null_assertion() {
    let js = transform_ts_to_js("let x = a!.b").expect("ok");
    assert!(!js.contains("!."));
}
