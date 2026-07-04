//! Snapshot + robustness tests for the JS emitter (`compile`). Snapshots are
//! kept inline: the exact emitted text is the contract with the runtime, so a
//! change should be a visible diff here.

use lunas_compiler::compile;

/// Compiles and asserts no error diagnostics, returning the module text.
fn emit(source: &str) -> String {
    let (js, diags) = compile(source);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected error diagnostics: {diags:?}"
    );
    js.expect("module emitted")
}

#[test]
fn plain_text_bind() {
    let js = emit(
        "html:\n    <p>${count}</p>\nscript:\n    let count = 0\n    function inc(){ count++ }\n",
    );
    assert_eq!(
        js,
        "\
import { component, anchorAppend, bind, box } from \"lunas\";

const HTML = \"<p></p>\";

export default component(\"div\", {}, HTML, (c, props) => {
  const count = box(c, 0, 0)
  function inc(){ count.v++ }
  const t0 = anchorAppend(c.root.childNodes[0]);
  bind(c, [0], () => { t0.data = `${count.v}`; });
});
"
    );
}

#[test]
fn mixed_text_run_is_one_node() {
    let js = emit(
        "html:\n    <p>count: ${count}!</p>\nscript:\n    let count = 0\n    function inc(){ count++ }\n",
    );
    // The literal "count: " and "!" live in the dynamic text node, not the HTML.
    assert!(js.contains("const HTML = \"<p></p>\";"), "{js}");
    assert!(
        js.contains("t0.data = `count: ${count.v}!`;"),
        "run reproduced as one template literal: {js}"
    );
    assert!(js.contains("bind(c, [0]"), "{js}");
}

#[test]
fn static_text_interpolation_no_bind() {
    // No reactive dep -> assigned once at build, no bind.
    let js = emit("html:\n    <p>${WIDTH}</p>\nscript:\n    const WIDTH = 5\n");
    assert!(js.contains("t0.data = `${WIDTH}`;"), "{js}");
    assert!(
        !js.contains("bind("),
        "no bind for static interpolation: {js}"
    );
}

#[test]
fn bound_attribute() {
    let js = emit(
        "html:\n    <div :title=\"label\"></div>\nscript:\n    let label = \"hi\"\n    function set(){ label = \"yo\" }\n",
    );
    assert!(js.contains("e0.setAttribute(\"title\", label.v);"), "{js}");
    assert!(js.contains("bind(c, [0], () => {"), "{js}");
}

#[test]
fn value_attribute_uses_property() {
    let js = emit(
        "html:\n    <input :value=\"name\">\nscript:\n    let name = \"a\"\n    function f(){ name = \"b\" }\n",
    );
    assert!(
        js.contains("e0.value = name.v;"),
        "value via property: {js}"
    );
}

#[test]
fn boolean_attribute_uses_property() {
    let js = emit(
        "html:\n    <input :disabled=\"locked\">\nscript:\n    let locked = false\n    function f(){ locked = true }\n",
    );
    assert!(
        js.contains("e0.disabled = !!(locked.v);"),
        "boolean via property with truthiness: {js}"
    );
}

#[test]
fn interpolated_static_attribute() {
    let js = emit(
        "html:\n    <div class=\"a ${x} b\"></div>\nscript:\n    let x = \"m\"\n    function f(){ x = \"n\" }\n",
    );
    assert!(
        js.contains("e0.setAttribute(\"class\", `a ${x.v} b`);"),
        "{js}"
    );
    assert!(js.contains("bind(c, [0]"), "{js}");
}

#[test]
fn event_listener() {
    let js = emit(
        "html:\n    <button @click=\"inc()\">x</button>\nscript:\n    let n = 0\n    function inc(){ n++ }\n",
    );
    assert!(js.contains("on(e0, \"click\", () => { inc(); });"), "{js}");
}

#[test]
fn props_seeded_from_input() {
    let js = emit(
        "@input start:number = 0\nhtml:\n    <p>${start}</p>\nscript:\n    let count = start\n    function f(){ count = 1 }\n",
    );
    assert!(js.contains("let start = props.start ?? (0);"), "{js}");
}

#[test]
fn deep_mutation_uses_deepbox() {
    // `o.k = 1` mutates a member, so `o` is reactive (root reported) *and*
    // deeply mutated -> deepBox.
    let js =
        emit("html:\n    <p>${o.k}</p>\nscript:\n    let o = {}\n    function add(){ o.k = 1 }\n");
    assert!(
        js.contains("deepBox(c, 0, {})"),
        "deep mutation -> deepBox: {js}"
    );
    assert!(
        js.contains("import { component, anchorAppend, bind, deepBox }"),
        "{js}"
    );
    // The template read is rewritten through the box.
    assert!(js.contains("t0.data = `${o.v.k}`;"), "{js}");
}

#[test]
fn two_way_binding_is_voided_with_diagnostic() {
    let (js, diags) = compile(
        "html:\n    <input ::value=\"name\">\nscript:\n    let name = \"a\"\n    function f(){ name = \"b\" }\n",
    );
    let js = js.expect("module still emitted");
    assert!(js.contains("/* TODO(wave2): two-way ::value"), "{js}");
    assert!(
        diags
            .iter()
            .any(|d| !d.is_error() && d.message.contains("two-way")),
        "a non-fatal diagnostic is reported: {diags:?}"
    );
}

#[test]
fn if_slot_voided_gracefully() {
    let js = emit("html:\n    <div><span :if=\"c\">y</span></div>\nscript:\n    let c = true\n    function t(){ c = false }\n");
    assert!(js.contains("/* TODO(wave2): if block"), "{js}");
    // Still a valid module.
    assert!(js.starts_with("import { component"), "{js}");
}

#[test]
fn no_template_emits_nothing() {
    let (js, diags) = compile("script:\n    let x = 0\n");
    assert!(js.is_none());
    assert!(!diags.iter().any(|d| d.is_error()));
}

// --- robustness: never panic ---------------------------------------------

#[test]
fn malformed_inputs_do_not_panic() {
    let cases = [
        "",
        "html:",
        "html:\n    <div :if=\"\" :for=\"\" @click=\"\">${}</div>",
        "html:\n    <p>${ a.b.c( }</p>\nscript:\n    let a = 0",
        "@input\n@use\nhtml:\n    <X/>",
        "html:\n    <button @click=\"a = b = c++\">x</button>\nscript:\n    let a=0\n    let b=0",
        "html:\n    <li :for=\"x of\">y</li>\nscript:\n    let z = 0",
        "html:\n    <p>${日本語}</p>\nscript:\n    let 日本語 = 0\n    function f(){ 日本語 = 1 }",
        "html:\n    <div :x=\"{ a }\"></div>\nscript:\n    let a = 0\n    function f(){ a = 1 }",
        "html:\n    <p>a ${x} b ${y} c</p>\nscript:\n    let x=0\n    let y=0\n    function f(){ x=1; y=2 }",
        "html:\n    <input ::value=\"o.k\">\nscript:\n    let o = {}\n    function f(){ o.k = 1 }",
    ];
    for case in cases {
        let (_js, _diags) = compile(case);
    }
}
