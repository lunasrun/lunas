//! Tests for the resolution layer: building a `ResolvedComponent` from source.

use lunas_compiler::resolve;

fn ok(src: &str) -> lunas_compiler::ResolvedComponent {
    let (c, diags) = resolve(src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected errors: {diags:?}"
    );
    c
}

const COUNTER: &str = "\
@input start:number = 0
@use Card from \"./Card.lunas\"
html:
    <button @click=\"inc()\">${count}</button>
style:
    button { color: red }
script:
    let count = start
    const label = \"hi\"
    function inc(){ count++ }
";

#[test]
fn extracts_props_and_imports() {
    let c = ok(COUNTER);
    assert_eq!(c.props.len(), 1);
    assert_eq!(c.props[0].name, "start");
    assert_eq!(c.imports.len(), 1);
    assert_eq!(c.imports[0].component_name, "Card");
    assert!(c.template.is_some());
    assert!(c.script.is_some());
    assert!(c.style.is_some());
}

#[test]
fn reactive_var_is_mutated_binding() {
    let c = ok(COUNTER);
    // `count` is reassigned by `inc`, so it is reactive.
    assert!(c.is_reactive("count"));
    // `label` is a const never mutated → not reactive.
    assert!(!c.is_reactive("label"));
    // `start` is an `@input` prop: a parent can change it after init, so it is
    // reactive too, numbered after the script vars.
    assert!(c.is_reactive("start"));
    assert!(
        c.reactive_index("start").unwrap() > c.reactive_index("count").unwrap(),
        "props are numbered after script reactive vars"
    );
}

#[test]
fn reactive_vars_numbered_in_order() {
    let src = "\
html:
    <p>${a}${b}</p>
script:
    let a = 0
    let b = 0
    let c = 0
    function f(){ a = 1; b = 2 }
";
    let c = ok(src);
    // a and b are mutated; c is not.
    let names: Vec<_> = c.reactive_vars.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(c.reactive_index("a"), Some(0));
    assert_eq!(c.reactive_index("b"), Some(1));
    assert_eq!(c.reactive_index("c"), None);
}

#[test]
fn top_level_reassignment_is_reactive() {
    // A binding reassigned at the top level (not just inside a function).
    let src = "\
html:
    <p>${x}</p>
script:
    let x = 0
    x = 1
";
    let c = ok(src);
    assert!(c.is_reactive("x"));
}

#[test]
fn decl_range_is_file_absolute() {
    let c = ok(COUNTER);
    let v = c
        .reactive_vars
        .iter()
        .find(|v| v.name == "count")
        .expect("count");
    let range = v.decl_range.expect("decl range");
    assert_eq!(range.slice(COUNTER), Some("count"));
}

#[test]
fn no_script_means_no_reactive_vars() {
    let c = ok("html:\n    <p>hi</p>\n");
    assert!(c.reactive_vars.is_empty());
    assert!(c.script.is_none());
}

#[test]
fn unparseable_script_reports_error_and_recovers() {
    let (c, diags) = resolve("html:\n    <p>hi</p>\nscript:\n    let = = =\n");
    assert!(diags.iter().any(|d| d.is_error()));
    // Still returns a component.
    assert!(c.reactive_vars.is_empty());
    assert!(c.template.is_some());
}
