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
fn two_way_binding_emits_property_and_write_back() {
    // `::value` binds the property (read side) *and* an input listener that
    // writes the element's value back into the lvalue (write side).
    let (js, diags) = compile(
        "html:\n    <input ::value=\"name\">\nscript:\n    let name = \"a\"\n    function f(){ name = \"b\" }\n",
    );
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "no error diags: {diags:?}"
    );
    let js = js.expect("module emitted");
    assert!(
        js.contains("bind(c, [0], () => { e0.value = name.v; });"),
        "read side binds the value property: {js}"
    );
    assert!(
        js.contains("on(e0, \"input\", () => { name.v = e0.value; });"),
        "write side listens on input and writes back: {js}"
    );
}

#[test]
fn two_way_checked_uses_change_event() {
    // `::checked` reflects a boolean property and commits on `change`
    // (checkbox/radio semantics), not `input`.
    let js = emit(
        "html:\n    <input type=\"checkbox\" ::checked=\"done\">\nscript:\n    let done = false\n    function f(){ done = true }\n",
    );
    assert!(
        js.contains("e0.checked = !!(done.v);"),
        "read side sets checked truthiness: {js}"
    );
    assert!(
        js.contains("on(e0, \"change\", () => { done.v = e0.checked; });"),
        "write side listens on change: {js}"
    );
}

#[test]
fn plain_if_emits_ifblock() {
    let js = emit(
        "html:\n    <div><span :if=\"show\">y</span></div>\nscript:\n    let show = true\n    function t(){ show = false }\n",
    );
    // The branch skeleton is hoisted and built by its own fromHTML when shown.
    assert!(js.contains("const HTML_1 = \"<span>y</span>\";"), "{js}");
    assert!(
        js.contains("const a0 = anchorAppend(c.root.childNodes[0]);"),
        "{js}"
    );
    assert!(
        js.contains("ifBlock(c, a0, [0], () => (show.v), () => {"),
        "single :if uses the cheap ifBlock path: {js}"
    );
    assert!(js.contains("fromHTML(HTML_1, a0)"), "{js}");
}

#[test]
fn if_elseif_else_emits_ifchain() {
    let js = emit(
        "html:\n    <div><p :if=\"n > 0\">pos ${n}</p><p :elseif=\"n < 0\">neg</p><p :else>zero</p></div>\nscript:\n    let n = 0\n    function set(x){ n = x }\n",
    );
    // A cascade compiles to one ifChain with a which() index selector and a
    // parallel array of branch builders.
    assert!(
        js.contains("ifChain(c, a0, [0], () => (n.v > 0) ? 0 : (n.v < 0) ? 1 : 2, ["),
        "which() maps conditions to a branch index, :else = last: {js}"
    );
    // Each branch has its own hoisted skeleton; the first is dynamic.
    assert!(js.contains("const HTML_1 = \"<p></p>\";"), "{js}");
    assert!(js.contains("const HTML_2 = \"<p>neg</p>\";"), "{js}");
    assert!(js.contains("const HTML_3 = \"<p>zero</p>\";"), "{js}");
    assert!(
        js.contains("bind(c, [0], () => { t0.data = `pos ${n.v}`; });"),
        "nested bind inside a branch works: {js}"
    );
}

#[test]
fn if_without_else_selects_minus_one() {
    let js = emit(
        "html:\n    <div><p :if=\"a\">x</p><p :elseif=\"b\">y</p></div>\nscript:\n    let a = true\n    let b = false\n    function f(){ a = false }\n",
    );
    assert!(
        js.contains("? 1 : -1, ["),
        "no :else -> which() returns -1 (no branch): {js}"
    );
}

#[test]
fn keyed_for_emits_forblock_with_keyof() {
    let js = emit(
        "html:\n    <ul><li :for=\"item of items\" :key=\"item.id\" @click=\"del(item.id)\">${item.label}</li></ul>\nscript:\n    let items = [{id:1,label:\"a\"}]\n    function del(id){ items = items.filter((x) => x.id !== id) }\n",
    );
    // The item skeleton is hoisted; the loop binding shadows reactive vars
    // (so `item.label` is NOT rewritten to `.v`).
    assert!(js.contains("const HTML_1 = \"<li></li>\";"), "{js}");
    assert!(
        js.contains("forBlock(c, a0, [0], () => Array.from((items.v) || []), {"),
        "iterable evaluated in the outer scope, reactive on items: {js}"
    );
    assert!(js.contains("html: HTML_1,"), "{js}");
    assert!(
        js.contains("wire: (r0, d0) => {"),
        "compiled html/wire mode: {js}"
    );
    assert!(
        js.contains("let item = d0;"),
        "item bound from the data cell: {js}"
    );
    assert!(
        js.contains("bind(c, [], () => { t0.data = `${item.label}`; });"),
        "item-local text bind (empty deps, refreshed by runScope): {js}"
    );
    assert!(
        js.contains("on(e0, \"click\", () => { del(item.id); });"),
        "per-item listener closes over the item binding: {js}"
    );
    assert!(
        js.contains("keyOf: (d2) => { const item = d2; return (item.id); },"),
        ":key becomes keyOf, :key attr stripped from the DOM: {js}"
    );
    // The stripped :key must not appear as a DOM attribute.
    assert!(
        !js.contains("setAttribute(\"key\""),
        "key is not a DOM attr: {js}"
    );
}

#[test]
fn if_in_for_reacts_and_nests() {
    // Nested :if inside a :for item: the array is deep-mutated (push), so it is
    // a reactive deepBox and the forBlock depends on it; the nested ifBlock is
    // wired inside the item and refreshed via the item scope.
    let js = emit(
        "html:\n    <ul><li :for=\"item of items\" :key=\"item.id\"><em :if=\"item.done\">done</em>${item.label}</li></ul>\nscript:\n    let items = []\n    function add(){ items.push({id:1,label:\"a\",done:false}) }\n",
    );
    assert!(
        js.contains("deepBox(c, 0, [])"),
        "array push -> deepBox: {js}"
    );
    assert!(
        js.contains("forBlock(c, a0, [0], () => Array.from((items.v) || []), {"),
        "for reacts to the reactive array: {js}"
    );
    assert!(
        js.contains("ifBlock(c, a1, [], () => (item.done), () => {"),
        "nested :if wired inside the item, keyed on the loop binding: {js}"
    );
}

#[test]
fn for_in_if_reacts_and_nests() {
    let js = emit(
        "html:\n    <div :if=\"show\"><p :for=\"t of tags\">${t}</p></div>\nscript:\n    let show = true\n    let tags = [\"x\"]\n    function toggle(){ show = !show }\n    function addTag(){ tags.push(\"y\") }\n",
    );
    assert!(js.contains("box(c, 0, true)"), "show boxed: {js}");
    assert!(js.contains("deepBox(c, 1, [\"x\"])"), "tags deepBox: {js}");
    assert!(
        js.contains("ifBlock(c, a0, [0], () => (show.v), () => {"),
        "outer :if reacts to show: {js}"
    );
    assert!(
        js.contains("forBlock(c, a1, [1], () => Array.from((tags.v) || []), {"),
        "inner :for reacts to tags, nested inside the branch: {js}"
    );
}

#[test]
fn helper_name_collision_is_aliased() {
    // A reactive var named `on` would shadow the `on` runtime helper. The
    // emitter imports the helper under an alias and references the alias, so
    // the user binding and the helper coexist.
    let js = emit(
        "html:\n    <button @click=\"flip()\" :if=\"on\">x</button>\nscript:\n    let on = false\n    function flip(){ on = !on }\n",
    );
    assert!(
        js.contains("on as $on"),
        "helper imported under alias: {js}"
    );
    assert!(
        js.contains("const on = box(c, 0, false)"),
        "user binding kept: {js}"
    );
    assert!(
        js.contains("$on(e0, \"click\""),
        "helper referenced by alias: {js}"
    );
    // The bare `on(` call form must not appear (would call the box).
    assert!(!js.contains(" on(e0"), "no bare on() call: {js}");
}

#[test]
fn box_helper_collision_is_aliased() {
    // A binding literally named `box` collides with the box constructor helper.
    let js =
        emit("html:\n    <p>${box}</p>\nscript:\n    let box = 0\n    function f(){ box = 1 }\n");
    assert!(js.contains("box as $box"), "box helper aliased: {js}");
    assert!(
        js.contains("const box = $box(c, 0, 0)"),
        "user `box` boxed via alias: {js}"
    );
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

/// Fuzz: arbitrary nestings of `:if`/`:for` (plus a couple of adversarial
/// leaves) must never panic and must never produce error diagnostics — deep or
/// otherwise-unsupported nesting is voided with a *warning*, never a crash.
#[test]
fn arbitrary_control_flow_nesting_never_panics() {
    // A small grammar of nested control-flow fragments, expanded to increasing
    // depths. Leaves include a two-way bind and an interpolation so wiring runs
    // inside every level.
    fn leaf(depth: usize) -> String {
        match depth % 3 {
            0 => "<span>${x}</span>".to_string(),
            1 => "<input ::value=\"x\">".to_string(),
            _ => "<b :if=\"x\">${x}</b>".to_string(),
        }
    }
    fn wrap_if(inner: &str) -> String {
        format!("<div :if=\"x\">{inner}</div>")
    }
    fn wrap_for(inner: &str) -> String {
        format!("<div :for=\"x of xs\" :key=\"x\">{inner}</div>")
    }

    let script =
        "\nscript:\n    let x = 0\n    let xs = []\n    function f(){ x = 1; xs.push(x) }\n";
    for depth in 0..40usize {
        let mut body = leaf(depth);
        for i in 0..depth {
            body = if i % 2 == 0 {
                wrap_if(&body)
            } else {
                wrap_for(&body)
            };
        }
        let source = format!("html:\n    {body}{script}");
        let (js, diags) = compile(&source);
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "depth {depth}: unexpected error diagnostic: {diags:?}"
        );
        // Whatever is emitted must at least be a module (or None when there is
        // nothing to emit); never a panic — reaching here is the assertion.
        let _ = js;
    }
}
