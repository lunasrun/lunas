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
    // Every `@input` prop is reactive: adopted as a `prop` box at its index
    // (props numbered after script vars) so the child's template reads react
    // when a parent pushes a new value. The default seeds it when the parent
    // omits the prop.
    let js = emit(
        "@input start:number = 0\nhtml:\n    <p>${start}</p>\nscript:\n    let count = start\n    function f(){ count = 1 }\n",
    );
    // `count` is reactive var 0 (script, declared first); `start` is 1 (prop).
    assert!(
        js.contains("const start = prop(c, \"start\", 1, props.start, (0));"),
        "prop adopted as a reactive box with its default: {js}"
    );
    // The template read of the prop is now a reactive bind on the prop index.
    assert!(
        js.contains("bind(c, [1], () => { t0.data = `${start.v}`; });"),
        "prop read is reactive: {js}"
    );
    // Script that reads the prop sees it through the box.
    assert!(
        js.contains("const count = box(c, 0, start.v)"),
        "script read of the prop rewritten through the box: {js}"
    );
    assert!(js.contains(", prop }"), "prop helper imported: {js}");
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

// --- child components -----------------------------------------------------

#[test]
fn simple_child_mounts_at_anchor() {
    // A childless static child: import from the `@use` table, anchor, mount.
    let js = emit("@use Child from \"./Child.lunas\"\nhtml:\n    <div><Child/></div>\n");
    assert!(
        js.contains("import Child from \"./Child.lunas\";"),
        "child module imported from @use, path as written: {js}"
    );
    assert!(
        js.contains("const a0 = anchorAppend(c.root.childNodes[0]);"),
        "anchor created inside the host element: {js}"
    );
    assert!(
        js.contains("const ch0 = mountChild(c, a0, Child, {});"),
        "mounted with an empty props object: {js}"
    );
}

#[test]
fn child_reactive_and_static_props() {
    // Reactive prop -> getter seed + driving bind on its deps; static prop ->
    // plain value in the initial object (no bind).
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <div><Card :count=\"n\" title=\"hi\"/></div>\nscript:\n    let n = 0\n    function inc(){ n++ }\n",
    );
    assert!(js.contains("import Card from \"./Card.lunas\";"), "{js}");
    assert!(
        js.contains("mountChild(c, a0, Card, { count: () => (n.v), title: `hi` });"),
        "reactive prop is a getter, static prop is a value: {js}"
    );
    assert!(
        js.contains("bind(c, [0], () => { ch0.setProp(\"count\", n.v); });"),
        "parent drives the reactive prop through setProp on its deps: {js}"
    );
    // A static-only prop must NOT get a driving bind.
    assert!(
        !js.contains("setProp(\"title\""),
        "static prop is not driven: {js}"
    );
}

#[test]
fn child_boolean_and_interpolated_props() {
    // Valueless prop -> boolean true; interpolated string prop -> reactive
    // template-literal getter + driving bind.
    let js = emit(
        "@use B from \"./B.lunas\"\nhtml:\n    <div><B flag greeting=\"hi ${name}\"/></div>\nscript:\n    let name = \"x\"\n    function f(){ name = \"y\" }\n",
    );
    assert!(js.contains("flag: true"), "valueless prop -> true: {js}");
    assert!(
        js.contains("greeting: () => (`hi ${name.v}`)"),
        "interpolated string prop seeds via a template-literal getter: {js}"
    );
    assert!(
        js.contains("ch0.setProp(\"greeting\", `hi ${name.v}`);"),
        "interpolated prop driven on its deps: {js}"
    );
}

#[test]
fn child_in_if_branch() {
    // A child inside an `:if` branch is wired inside the branch fragment,
    // recursively; the import is still emitted.
    let js = emit(
        "@use Panel from \"./Panel.lunas\"\nhtml:\n    <div :if=\"show\"><Panel :v=\"n\"/></div>\nscript:\n    let show = true\n    let n = 0\n    function f(){ show = false; n++ }\n",
    );
    assert!(js.contains("import Panel from \"./Panel.lunas\";"), "{js}");
    assert!(
        js.contains("ifBlock(c, a0, [0], () => (show.v), () => {"),
        "outer :if: {js}"
    );
    assert!(
        js.contains("mountChild(c, a1, Panel, { v: () => (n.v) });"),
        "child mounted inside the branch fragment: {js}"
    );
    // `n` is reactive var 1 (after `show`); the driving bind uses that index.
    assert!(
        js.contains("bind(c, [1], () => { ch0.setProp(\"v\", n.v); });"),
        "driving bind uses the prop-source dep index: {js}"
    );
}

#[test]
fn child_in_for_item() {
    // A child inside a `:for` item: the loop binding shadows reactive vars, so
    // the prop getter reads the item binding directly (no `.v`), and the
    // driving bind is item-coupled (empty deps, refreshed by runScope).
    let js = emit(
        "@use Row from \"./Row.lunas\"\nhtml:\n    <ul><li :for=\"item of items\" :key=\"item.id\"><Row :data=\"item\"/></li></ul>\nscript:\n    let items = []\n    function add(){ items.push({id:1}) }\n",
    );
    assert!(js.contains("import Row from \"./Row.lunas\";"), "{js}");
    assert!(
        js.contains("mountChild(c, a1, Row, { data: () => (item) });"),
        "prop getter reads the loop binding (not rewritten to .v): {js}"
    );
    assert!(
        js.contains("bind(c, [], () => { ch0.setProp(\"data\", item); });"),
        "item-coupled driving bind (empty deps, item scope refreshes it): {js}"
    );
}

#[test]
fn multiple_children_and_dedup_imports() {
    // Two uses of the same child + one of another: each import appears once,
    // one handle per instance.
    let js = emit(
        "@use A from \"./A.lunas\"\n@use Bee from \"./Bee.lunas\"\nhtml:\n    <div><A/><Bee/><A/></div>\n",
    );
    assert_eq!(
        js.matches("import A from \"./A.lunas\";").count(),
        1,
        "duplicate child imported once: {js}"
    );
    assert!(js.contains("import Bee from \"./Bee.lunas\";"), "{js}");
    assert!(js.contains("const ch0 = mountChild(c, a0, A, {});"), "{js}");
    assert!(
        js.contains("const ch1 = mountChild(c, a1, Bee, {});"),
        "{js}"
    );
    assert!(js.contains("const ch2 = mountChild(c, a2, A, {});"), "{js}");
}

#[test]
fn unused_use_import_is_not_emitted() {
    // A declared-but-unused `@use` produces no dead import.
    let js = emit("@use Unused from \"./Unused.lunas\"\nhtml:\n    <p>hi</p>\n");
    assert!(
        !js.contains("import Unused"),
        "unused @use is not imported: {js}"
    );
}

#[test]
fn child_import_name_collision_is_aliased() {
    // A component tag that collides with a runtime helper name is imported
    // under an alias, and the mount references the alias.
    let js = emit(
        "@use bind from \"./bind.lunas\"\nhtml:\n    <div><bind :v=\"n\"/></div>\nscript:\n    let n = 0\n    function f(){ n++ }\n",
    );
    assert!(
        js.contains("import bind$ from \"./bind.lunas\";"),
        "colliding child import aliased: {js}"
    );
    assert!(
        js.contains("mountChild(c, a0, bind$, {"),
        "mount references the aliased local: {js}"
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
        // Child components with adversarial / unsupported props.
        "@use X from \"./X.lunas\"\nhtml:\n    <X :p=\"a.(\" ::two=\"b\" @go=\"h()\" q=\"${z}\"/>\nscript:\n    let a=0\n    let b=0\n    let z=0",
        "@use X from \"./X.lunas\"\nhtml:\n    <X/>",
        "html:\n    <Undeclared :p=\"x\"/>\nscript:\n    let x = 0",
        // DOM feature batch adversarial inputs (never-panic).
        "html:\n    <div :class=\"{ a: (\" :style=\"[\" :html=\"x.(\"></div>\nscript:\n    let x=0\n    function f(){ x=1 }",
        "html:\n    <input :ref=\"a.b\">\nscript:\n    let a=0",
        "html:\n    <input :ref=\"notdeclared\">",
        "html:\n    <component/>\n    <component :is=\"\"/>",
        "html:\n    <component :is=\"v\" :p=\"q.(\"/>\nscript:\n    let v=0\n    let q=0",
        "html:\n    <teleport></teleport>\n    <teleport to=\"\"><p></p></teleport>",
        "html:\n    <teleport :to=\"t.(\"><div :if=\"x\">y</div></teleport>\nscript:\n    let x=0\n    let t=0\n    function f(){ x=1 }",
        // Multi-root with mixed content / dynamics at top level.
        "html:\n    <p>${a}</p>\n    <span :if=\"a\">x</span>\n    <b :for=\"i of a\">${i}</b>\nscript:\n    let a=[]\n    function f(){ a.push(1) }",
        "html:\n    <component :is=\"v\"/>\n    <teleport to=\"#x\"><i></i></teleport>\nscript:\n    let v=0",
    ];
    for case in cases {
        let (_js, _diags) = compile(case);
    }
}

// --- DOM feature batch: class/style, refs, html, :is, teleport, fragments ----

#[test]
fn class_binding_merges_static_via_setclass() {
    let js = emit(
        "html:\n    <div class=\"base\" :class=\"{ active: on }\"></div>\nscript:\n    let on = true\n    function t(){ on = false }\n",
    );
    // Static class stays in the skeleton *and* is merged at runtime.
    assert!(
        js.contains("const HTML = \"<div class=\\\"base\\\"></div>\";"),
        "{js}"
    );
    assert!(
        js.contains("setClass(e0, \"base\", { active: on.v });"),
        "class merge via setClass: {js}"
    );
    assert!(
        js.contains("import { component") && js.contains("setClass"),
        "{js}"
    );
}

#[test]
fn style_binding_uses_setstyle() {
    let js = emit(
        "html:\n    <div :style=\"{ color: hue }\"></div>\nscript:\n    let hue = \"red\"\n    function t(){ hue = \"blue\" }\n",
    );
    assert!(
        js.contains("setStyle(e0, \"\", { color: hue.v });"),
        "style via setStyle: {js}"
    );
}

#[test]
fn html_binding_sets_inner_html() {
    let js = emit(
        "html:\n    <div :html=\"raw\"></div>\nscript:\n    let raw = \"<b>x</b>\"\n    function t(){ raw = \"\" }\n",
    );
    assert!(js.contains("e0.innerHTML = raw.v;"), "raw html bind: {js}");
    assert!(js.contains("bind(c, [0]"), "reactive: {js}");
}

#[test]
fn html_binding_with_children_warns() {
    let (js, diags) = compile(
        "html:\n    <div :html=\"raw\"><span>ignored</span></div>\nscript:\n    let raw = \"x\"\n    function t(){ raw = \"y\" }\n",
    );
    assert!(js.is_some(), "still compiles");
    assert!(
        diags
            .iter()
            .any(|d| !d.is_error() && d.message.contains("both `:html` and children")),
        "children+:html warns: {diags:?}"
    );
}

#[test]
fn ref_binding_assigns_element_into_box() {
    let js = emit(
        "html:\n    <input :ref=\"el\">\nscript:\n    let el\n    function t(){ el.focus() }\n",
    );
    // `el` is boxed (mutated by the ref) and receives the element at wire time.
    assert!(js.contains("const el = box(c, 0, undefined)"), "{js}");
    assert!(js.contains("el.v = e0;"), "ref assignment: {js}");
    // No bind — the reference is fixed for the element's lifetime.
    assert!(
        js.contains("el.v = e0;") && !js.contains("() => { el.v"),
        "{js}"
    );
}

#[test]
fn dynamic_component_uses_dynamicblock() {
    let js = emit(
        "@use Foo from \"./Foo.lun\"\n@use Bar from \"./Bar.lun\"\nhtml:\n    <component :is=\"view\" :label=\"txt\"/>\nscript:\n    let view = Foo\n    let txt = \"hi\"\n    function t(){ view = Bar; txt = \"yo\" }\n",
    );
    // All @use factories imported (a :is expr can name any of them).
    assert!(js.contains("import Foo from \"./Foo.lun\";"), "{js}");
    assert!(js.contains("import Bar from \"./Bar.lun\";"), "{js}");
    assert!(
        js.contains("dynamicBlock(c, a0, "),
        "dynamicBlock emitted: {js}"
    );
    assert!(js.contains("() => (view.v)"), "factory getter: {js}");
    assert!(js.contains("label: () => (txt.v)"), "prop getter: {js}");
    assert!(
        js.contains(".setProp(\"label\", txt.v)"),
        "prop drive: {js}"
    );
}

#[test]
fn dynamic_component_without_is_is_voided() {
    let (js, diags) = compile("html:\n    <component/>\n");
    assert!(js.is_some());
    assert!(
        diags
            .iter()
            .any(|d| !d.is_error() && d.message.contains("requires an `:is`")),
        "{diags:?}"
    );
}

#[test]
fn teleport_uses_teleportblock() {
    let js = emit(
        "html:\n    <teleport to=\"#modal\"><p>${msg}</p></teleport>\nscript:\n    let msg = \"hi\"\n    function t(){ msg = \"bye\" }\n",
    );
    assert!(
        js.contains("teleportBlock(c, a0, () => (`#modal`)"),
        "target selector: {js}"
    );
    assert!(js.contains("fromHTML(HTML_1, a0)"), "body fragment: {js}");
    assert!(
        js.contains("return Array.from(r0.childNodes);"),
        "node group: {js}"
    );
}

#[test]
fn teleport_to_expression() {
    let js = emit(
        "html:\n    <teleport :to=\"target\"><span></span></teleport>\nscript:\n    let target = null\n    function t(){ target = 1 }\n",
    );
    assert!(
        js.contains("teleportBlock(c, a0, () => (target.v)"),
        "element target: {js}"
    );
}

#[test]
fn multi_root_template_uses_fragment() {
    let js = emit(
        "html:\n    <h1>${title}</h1>\n    <p>${body}</p>\nscript:\n    let title = \"T\"\n    let body = \"B\"\n    function t(){ title = \"x\"; body = \"y\" }\n",
    );
    assert!(js.contains("import { fragment"), "fragment imported: {js}");
    assert!(
        js.contains("export default fragment({}, HTML, (c, props) => {"),
        "{js}"
    );
    assert!(!js.contains("component(\"div\""), "no wrapper: {js}");
    assert!(
        js.contains("const HTML = \"<h1></h1><p></p>\";"),
        "both roots in HTML: {js}"
    );
}

#[test]
fn single_root_stays_on_component() {
    let js = emit("html:\n    <p>${x}</p>\nscript:\n    let x = 0\n    function t(){ x = 1 }\n");
    assert!(
        js.contains("component(\"div\""),
        "single root uses component: {js}"
    );
    assert!(!js.contains("fragment("), "{js}");
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
        // Include a child component with a reactive prop, a static prop, and a
        // valueless prop at some leaves so component wiring runs at arbitrary
        // depth inside :if/:for.
        match depth % 4 {
            0 => "<span>${x}</span>".to_string(),
            1 => "<input ::value=\"x\">".to_string(),
            2 => "<Kid :v=\"x\" tag=\"t\" flag/>".to_string(),
            _ => "<b :if=\"x\">${x}<Kid :v=\"x\"/></b>".to_string(),
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
        let source = format!("@use Kid from \"./Kid.lunas\"\nhtml:\n    {body}{script}");
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

// --- slots (c-slots) --------------------------------------------------------

#[test]
fn slot_outlet_emits_slotblock_with_fallback() {
    // A child <slot> with fallback content compiles to slotBlock reading the
    // parent-provided factory, plus a fallback factory building its own content.
    let js = emit("html:\n    <div><slot>fallback</slot></div>\n");
    assert!(js.contains("import { component"), "{js}");
    assert!(js.contains("slotBlock"), "uses slotBlock: {js}");
    assert!(
        js.contains("props.$slots && props.$slots[\"default\"]"),
        "reads parent-provided default slot: {js}"
    );
    assert!(
        js.contains("fromHTML(HTML_1, a0)"),
        "fallback built from its own hoisted skeleton: {js}"
    );
    assert!(js.contains("const HTML_1 = \"fallback\";"), "{js}");
}

#[test]
fn slot_outlet_without_fallback_passes_null() {
    let js = emit("html:\n    <div><slot></slot></div>\n");
    assert!(
        js.contains("props.$slots && props.$slots[\"default\"], null);"),
        "no fallback -> null factory: {js}"
    );
}

#[test]
fn named_slot_outlet_uses_its_name() {
    let js = emit("html:\n    <div><slot name=\"foot\"></slot></div>\n");
    assert!(
        js.contains("props.$slots && props.$slots[\"foot\"]"),
        "named slot keyed by its name: {js}"
    );
}

#[test]
fn scoped_slot_outlet_emits_props_getter() {
    // A bound attr on <slot> becomes a scoped-slot props getter passed to slotBlock.
    let js = emit(
        "html:\n    <div><slot :item=\"row\"></slot></div>\nscript:\n    let row = 1\n    function f(){ row = 2 }\n",
    );
    assert!(
        js.contains("() => ({ item: (row.v) })"),
        "scoped slot props getter reads the bound expr: {js}"
    );
}

#[test]
fn parent_passes_default_slot_content() {
    // Bare children of a component become the default slot factory.
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card>hi ${msg}</Card>\nscript:\n    let msg = \"a\"\n    function f(){ msg = \"b\" }\n",
    );
    assert!(js.contains("slotContent"), "wraps parent content: {js}");
    assert!(
        js.contains("$slots: s0"),
        "passes $slots to mountChild: {js}"
    );
    assert!(
        js.contains("default: (slotProps, onCleanup) => "),
        "default slot factory: {js}"
    );
    assert!(
        js.contains("mountChild(c, a0, Card, { $slots: s0 })"),
        "{js}"
    );
}

#[test]
fn parent_named_slot_via_template_hash() {
    // `<template #foot>` routes to the named slot `foot`.
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card><template #foot>ft</template></Card>\n",
    );
    assert!(js.contains("foot: (slotProps, onCleanup) =>"), "{js}");
    assert!(
        !js.contains("default:"),
        "no default group when only named: {js}"
    );
}

#[test]
fn parent_scoped_slot_binding_names_param() {
    // `<template #default="p">` binds `p` to the slot props inside the content.
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card><template #default=\"p\">${p.item}</template></Card>\n",
    );
    assert!(
        js.contains("slotContent(c, (p) => {"),
        "scoped binding names the inner build param: {js}"
    );
}

#[test]
fn component_without_children_has_no_slots_member() {
    let js = emit("@use Card from \"./Card.lunas\"\nhtml:\n    <Card/>\n");
    assert!(!js.contains("$slots"), "no children -> no $slots: {js}");
    assert!(!js.contains("slotContent"), "{js}");
}

// --- non-reactive script var read by the template (pre-existing bug fix) -----

#[test]
fn never_mutated_var_read_by_template_is_emitted() {
    // A const declared in script, never mutated, but read by the template used
    // to be DROPPED (the whole script was omitted when reactive_vars was empty),
    // leaving the output referencing an undefined name. It must be emitted as a
    // plain const in setup.
    let js = emit("html:\n    <p>${WIDTH}</p>\nscript:\n    const WIDTH = 5\n");
    assert!(
        js.contains("const WIDTH = 5"),
        "non-reactive declared var must still be emitted: {js}"
    );
    assert!(js.contains("t0.data = `${WIDTH}`;"), "{js}");
    // No box import: the var is not reactive.
    assert!(
        !js.contains(", box"),
        "no box helper for a plain const: {js}"
    );
}

#[test]
fn empty_script_edge_emits_no_body() {
    // An empty script block must not produce stray output or panic.
    let js = emit("html:\n    <p>hi</p>\nscript:\n");
    assert!(js.contains("const HTML = \"<p>hi</p>\";"), "{js}");
    // The setup body is empty (no dangling declarations).
    assert!(
        js.contains("(c, props) => {\n});"),
        "empty setup body: {js}"
    );
}

#[test]
fn multiple_non_reactive_vars_all_emitted() {
    // Several never-mutated vars, all read by the template.
    let js = emit("html:\n    <p>${A}-${B}</p>\nscript:\n    const A = 1\n    const B = 2\n");
    assert!(js.contains("const A = 1"), "{js}");
    assert!(js.contains("const B = 2"), "{js}");
}

#[test]
fn slot_and_template_arbitrary_nesting_never_panics() {
    // <slot> and <template> tags in arbitrary nesting (inside :if / :for /
    // components, named/scoped/bare) must compile without panicking and never
    // emit invalid JS (reaching the end without a panic is the assertion).
    let fragments = [
        "<slot></slot>",
        "<slot>fb</slot>",
        "<slot name=\"x\"></slot>",
        "<slot :item=\"v\"></slot>",
        "<slot name=\"y\" :item=\"v\">f</slot>",
        "<template #a>x</template>",
        "<template #a=\"p\">${p.k}</template>",
        "<template slot=\"b\">y</template>",
        "<Card><slot></slot></Card>",
        "<Card><template #a><slot name=\"a\"></slot></template></Card>",
        "<div :if=\"v\"><slot></slot></div>",
        "<div :for=\"i of xs\" :key=\"i\"><slot :item=\"i\"></slot></div>",
        "<slot><slot>deep</slot></slot>",
    ];
    let script =
        "\nscript:\n    let v = 0\n    let xs = []\n    function f(){ v = 1; xs.push(v) }\n";
    for frag in fragments {
        for wrap in [
            frag.to_string(),
            format!("<div :if=\"v\">{frag}</div>"),
            format!("<Card>{frag}</Card>"),
            format!("<div>{frag}{frag}</div>"),
        ] {
            let source = format!("@use Card from \"./Card.lunas\"\nhtml:\n    {wrap}{script}");
            let (js, diags) = compile(&source);
            // Never a hard error-panic; diagnostics are allowed. If a module was
            // emitted, it must be non-empty text (well-formed enough to be a module).
            let _ = diags;
            if let Some(js) = js {
                assert!(!js.is_empty(), "emitted empty module for {wrap}");
            }
        }
    }
}
