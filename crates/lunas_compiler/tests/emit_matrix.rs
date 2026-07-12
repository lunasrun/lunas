//! Emit-correctness matrix for the JS emitter (`compile`). Structural
//! assertions (contains / count / regex-ish substring checks) over the emitted
//! module, covering the feature surface of `docs/output-design.md`:
//!
//! - text binds, attribute binds (setAttribute vs property vs boolean),
//!   interpolated static attributes, events, two-way (value/checked/other),
//! - `:if` / `:elseif` / `:else`, keyed `:for`,
//! - child mount + props getters/setProp, slots (default/named/scoped),
//! - class/style normalizers, refs, `:html`, dynamic `:is`, teleport,
//! - fragments/multi-root, `@input` defaults, non-reactive declared vars,
//! - helper-name collision aliasing.
//!
//! These complement `codegen_emit.rs` with a broader edge matrix; they assert
//! structural properties rather than full-string snapshots (except where a
//! stable exact fragment is the clearest contract).

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

// A small counter component whose only reactive var is `n` at index 0.
fn one_reactive(html: &str) -> String {
    emit(&format!(
        "html:\n    {html}\nscript:\n    let n = 0\n    function inc(){{ n++ }}\n"
    ))
}

// --- text binds ------------------------------------------------------------

#[test]
fn text_bind_creates_anchor_and_bind() {
    let js = one_reactive("<p>${n}</p>");
    assert!(js.contains("const HTML = \"<p></p>\";"), "{js}");
    assert!(js.contains("anchorAppend"), "{js}");
    assert!(js.contains("t0.data = `${n.v}`;"), "{js}");
    assert!(js.contains("bind(c, [0]"), "{js}");
}

#[test]
fn interleaved_literal_and_interpolation_is_one_template_literal() {
    let js = one_reactive("<p>a${n}b${n}c</p>");
    assert!(
        js.contains("t0.data = `a${n.v}b${n.v}c`;"),
        "whole run is one literal: {js}"
    );
}

#[test]
fn adjacent_interpolations_are_one_run_with_union_deps() {
    let js = emit(
        "html:\n    <p>${a}${b}</p>\nscript:\n    let a=0\n    let b=0\n    function f(){ a=1; b=2 }\n",
    );
    // Adjacent interpolations with no literal between them form ONE text run:
    // a single anchor + one bind on the union of deps.
    assert!(js.contains("t0.data = `${a.v}${b.v}`;"), "{js}");
    assert!(js.contains("bind(c, [0, 1]"), "union deps: {js}");
    assert!(!js.contains("t1.data"), "one anchor for the run: {js}");
}

#[test]
fn separated_interpolations_are_two_runs() {
    let js = emit(
        "html:\n    <p>${a}<b>x</b>${b}</p>\nscript:\n    let a=0\n    let b=0\n    function f(){ a=1; b=2 }\n",
    );
    // A static element separates the two interpolations into distinct text runs.
    assert!(js.contains("`${a.v}`;"), "{js}");
    assert!(js.contains("`${b.v}`;"), "{js}");
    assert!(js.contains("bind(c, [0]"), "{js}");
    assert!(js.contains("bind(c, [1]"), "{js}");
}

#[test]
fn combined_expression_bind_lists_union_of_deps() {
    let js = emit(
        "html:\n    <p>${a + b}</p>\nscript:\n    let a=0\n    let b=0\n    function f(){ a=1; b=2 }\n",
    );
    assert!(js.contains("bind(c, [0, 1]"), "union dep list: {js}");
}

// --- attribute binds: setAttribute vs property vs boolean ------------------

#[test]
fn generic_attribute_uses_setattribute() {
    let js = one_reactive("<div :data-id=\"n\"></div>");
    assert!(
        js.contains("e0.setAttribute(\"data-id\", n.v);"),
        "generic attr via setAttribute: {js}"
    );
}

#[test]
fn value_attribute_uses_value_property() {
    let js = one_reactive("<input :value=\"n\">");
    assert!(js.contains("e0.value = n.v;"), "value via property: {js}");
    assert!(
        !js.contains("setAttribute(\"value\""),
        "value never via setAttribute: {js}"
    );
}

#[test]
fn boolean_attributes_use_property_with_truthiness() {
    for (attr, prop) in [
        ("disabled", "disabled"),
        ("checked", "checked"),
        ("selected", "selected"),
        ("readonly", "readOnly"),
        ("multiple", "multiple"),
        ("hidden", "hidden"),
    ] {
        let js = one_reactive(&format!("<input :{attr}=\"n\">"));
        assert!(
            js.contains(&format!("e0.{prop} = !!(n.v);")),
            "{attr} -> {prop} truthiness: {js}"
        );
    }
}

#[test]
fn interpolated_static_attribute_is_a_template_literal() {
    let js = one_reactive("<div title=\"count ${n}!\"></div>");
    assert!(
        js.contains("e0.setAttribute(\"title\", `count ${n.v}!`);"),
        "{js}"
    );
    assert!(js.contains("bind(c, [0]"), "{js}");
}

#[test]
fn static_only_attribute_stays_in_html_and_gets_no_bind() {
    let js = emit("html:\n    <div id=\"main\" class=\"box\">hi</div>\n");
    assert!(
        js.contains("id=\\\"main\\\"") && js.contains("class=\\\"box\\\""),
        "static attrs stay in skeleton HTML: {js}"
    );
    assert!(!js.contains("bind("), "no bind for static attrs: {js}");
    assert!(!js.contains("setAttribute"), "{js}");
}

// --- events ----------------------------------------------------------------

#[test]
fn event_listener_wraps_handler() {
    let js = one_reactive("<button @click=\"inc()\">x</button>");
    assert!(js.contains("on(e0, \"click\", () => { inc(); });"), "{js}");
}

#[test]
fn event_with_various_names() {
    for ev in ["click", "input", "keydown", "mouseenter"] {
        let js = one_reactive(&format!("<button @{ev}=\"inc()\">x</button>"));
        assert!(js.contains(&format!("on(e0, \"{ev}\"")), "{ev}: {js}");
    }
}

// --- two-way bindings ------------------------------------------------------

#[test]
fn two_way_value_reads_property_and_writes_on_input() {
    let js = emit(
        "html:\n    <input ::value=\"name\">\nscript:\n    let name=\"\"\n    function f(){ name=\"x\" }\n",
    );
    assert!(js.contains("e0.value = name.v;"), "read side: {js}");
    assert!(
        js.contains("on(e0, \"input\", () => { name.v = e0.value; });"),
        "write side listens on input: {js}"
    );
}

#[test]
fn two_way_checked_reads_boolean_and_writes_on_change() {
    let js = emit(
        "html:\n    <input ::checked=\"done\">\nscript:\n    let done=false\n    function f(){ done=true }\n",
    );
    assert!(js.contains("e0.checked = !!(done.v);"), "read side: {js}");
    assert!(
        js.contains("on(e0, \"change\", () => { done.v = e0.checked; });"),
        "write side on change: {js}"
    );
}

// --- :if / :elseif / :else -------------------------------------------------

#[test]
fn plain_if_uses_ifblock() {
    let js = one_reactive("<div><span :if=\"n\">y</span></div>");
    assert!(
        js.contains("ifBlock(c, a0, [0], () => (n.v), () => {"),
        "single :if -> ifBlock: {js}"
    );
    assert!(
        !js.contains("ifChain"),
        "single branch never uses ifChain: {js}"
    );
}

#[test]
fn if_else_uses_ifchain_with_else_index() {
    let js = one_reactive("<div><p :if=\"n\">a</p><p :else>b</p></div>");
    // which(): condition true -> 0, else -> branch count (1).
    assert!(
        js.contains("ifChain(c, a0, [0], () => (n.v) ? 0 : 1, ["),
        "else is the last branch index: {js}"
    );
}

#[test]
fn if_elseif_else_maps_each_condition() {
    let js = emit(
        "html:\n    <div><p :if=\"a > 0\">p</p><p :elseif=\"a < 0\">n</p><p :else>z</p></div>\nscript:\n    let a=0\n    function f(){ a=1 }\n",
    );
    assert!(
        js.contains("() => (a.v > 0) ? 0 : (a.v < 0) ? 1 : 2, ["),
        "cascade selector: {js}"
    );
}

#[test]
fn if_elseif_without_else_selects_minus_one() {
    let js = emit(
        "html:\n    <div><p :if=\"a\">x</p><p :elseif=\"b\">y</p></div>\nscript:\n    let a=true\n    let b=false\n    function f(){ a=false }\n",
    );
    assert!(js.contains("? 1 : -1, ["), "no else -> -1: {js}");
}

#[test]
fn branch_bodies_are_hoisted_skeletons() {
    let js = one_reactive("<div><span :if=\"n\">y</span></div>");
    assert!(js.contains("const HTML_1 = \"<span>y</span>\";"), "{js}");
    assert!(js.contains("fromHTML(HTML_1, a0)"), "{js}");
}

// --- keyed :for ------------------------------------------------------------

#[test]
fn keyed_for_emits_forblock_wire_and_keyof() {
    let js = emit(
        "html:\n    <ul><li :for=\"item of items\" :key=\"item.id\">${item.label}</li></ul>\nscript:\n    let items=[]\n    function f(){ items.push({id:1,label:\"a\"}) }\n",
    );
    assert!(js.contains("const HTML_1 = \"<li></li>\";"), "{js}");
    assert!(
        js.contains("forBlock(c, a0, [0], () => Array.from((items.v) || []), {"),
        "iterable of-form: {js}"
    );
    assert!(js.contains("html: HTML_1,"), "{js}");
    assert!(js.contains("wire: (r0, d0) => {"), "{js}");
    assert!(js.contains("let item = d0;"), "{js}");
    // The patch-return data cell allocates d1, so keyOf's own cell is d2.
    assert!(
        js.contains("keyOf: (d2) => { const item = d2; return (item.id); },"),
        "keyOf reads the loop binding: {js}"
    );
    assert!(
        !js.contains("setAttribute(\"key\""),
        ":key stripped from the DOM: {js}"
    );
}

#[test]
fn for_in_form_uses_object_keys() {
    let js = emit(
        "html:\n    <ul><li :for=\"k in obj\">${k}</li></ul>\nscript:\n    let obj={}\n    function f(){ obj.a=1 }\n",
    );
    assert!(
        js.contains("() => Object.keys((obj.v) || {})"),
        "in-form iterates keys: {js}"
    );
}

#[test]
fn for_item_text_bind_is_item_coupled_empty_deps() {
    let js = emit(
        "html:\n    <ul><li :for=\"item of items\" :key=\"item\">${item}</li></ul>\nscript:\n    let items=[]\n    function f(){ items.push(1) }\n",
    );
    assert!(
        js.contains("bind(c, [], () => { t0.data = `${item}`; });"),
        "loop-binding read is item-coupled (empty deps, not rewritten to .v): {js}"
    );
}

#[test]
fn for_without_key_omits_keyof() {
    let js = emit(
        "html:\n    <ul><li :for=\"x of xs\">${x}</li></ul>\nscript:\n    let xs=[]\n    function f(){ xs.push(1) }\n",
    );
    assert!(js.contains("forBlock(c, a0, [0]"), "{js}");
    assert!(!js.contains("keyOf:"), "no key -> no keyOf: {js}");
}

// --- child components ------------------------------------------------------

#[test]
fn child_mount_imports_and_anchors() {
    let js = emit("@use Child from \"./Child.lunas\"\nhtml:\n    <div><Child/></div>\n");
    assert!(js.contains("import Child from \"./Child.lunas\";"), "{js}");
    assert!(
        js.contains("const ch0 = mountChild(c, a0, Child, {});"),
        "{js}"
    );
}

#[test]
fn child_reactive_prop_is_getter_plus_setprop() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <div><Card :count=\"n\"/></div>\nscript:\n    let n=0\n    function f(){ n++ }\n",
    );
    assert!(
        js.contains("mountChild(c, a0, Card, { count: () => (n.v) });"),
        "reactive prop seeded via getter: {js}"
    );
    assert!(
        js.contains("bind(c, [0], () => { ch0.setProp(\"count\", n.v); });"),
        "driven via setProp on deps: {js}"
    );
}

#[test]
fn child_static_prop_has_no_setprop() {
    let js = emit("@use Card from \"./Card.lunas\"\nhtml:\n    <div><Card title=\"hi\"/></div>\n");
    assert!(js.contains("title: `hi`"), "static prop is a value: {js}");
    assert!(!js.contains("setProp"), "static prop not driven: {js}");
}

#[test]
fn child_valueless_prop_is_boolean_true() {
    let js = emit("@use B from \"./B.lunas\"\nhtml:\n    <div><B flag/></div>\n");
    assert!(js.contains("flag: true"), "{js}");
}

#[test]
fn component_ref_assigns_handle_into_box() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <div><Card :ref=\"card\"/></div>\nscript:\n    let card\n    function f(){ card.foo() }\n",
    );
    assert!(
        js.contains("card.v = ch0;"),
        "component ref assigns the mount handle: {js}"
    );
    // The ref target is not passed as a prop.
    assert!(!js.contains("ref: "), "ref stripped from props: {js}");
}

// --- slots -----------------------------------------------------------------

#[test]
fn default_slot_outlet_reads_parent_default() {
    let js = emit("html:\n    <div><slot>fb</slot></div>\n");
    assert!(
        js.contains("props.$slots && props.$slots[\"default\"]"),
        "{js}"
    );
    assert!(
        js.contains("const HTML_1 = \"fb\";"),
        "fallback hoisted: {js}"
    );
}

#[test]
fn slot_without_fallback_passes_null() {
    let js = emit("html:\n    <div><slot></slot></div>\n");
    assert!(
        js.contains("props.$slots && props.$slots[\"default\"], null);"),
        "{js}"
    );
}

#[test]
fn named_slot_outlet_keyed_by_name() {
    let js = emit("html:\n    <div><slot name=\"foot\"></slot></div>\n");
    assert!(
        js.contains("props.$slots && props.$slots[\"foot\"]"),
        "{js}"
    );
}

#[test]
fn scoped_slot_outlet_props_getter() {
    let js = emit(
        "html:\n    <div><slot :row=\"data\"></slot></div>\nscript:\n    let data=1\n    function f(){ data=2 }\n",
    );
    assert!(js.contains("() => ({ row: (data.v) })"), "{js}");
}

#[test]
fn parent_default_slot_factory() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card>hi ${n}</Card>\nscript:\n    let n=0\n    function f(){ n++ }\n",
    );
    assert!(
        js.contains("default: (slotProps, onCleanup) =>"),
        "default slot factory: {js}"
    );
    assert!(js.contains("$slots: s0"), "$slots passed to child: {js}");
    assert!(js.contains("slotContent"), "{js}");
}

#[test]
fn parent_named_slot_via_hash_shorthand() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card><template #foot>ft</template></Card>\n",
    );
    assert!(js.contains("foot: (slotProps, onCleanup) =>"), "{js}");
    assert!(!js.contains("default:"), "only named -> no default: {js}");
}

#[test]
fn parent_named_slot_via_slot_attr() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card><template slot=\"foot\">ft</template></Card>\n",
    );
    assert!(js.contains("foot: (slotProps, onCleanup) =>"), "{js}");
}

#[test]
fn parent_scoped_slot_binds_param_name() {
    let js = emit(
        "@use Card from \"./Card.lunas\"\nhtml:\n    <Card><template #default=\"p\">${p.k}</template></Card>\n",
    );
    assert!(js.contains("slotContent(c, (p) => {"), "{js}");
}

// --- class / style normalizers ---------------------------------------------

#[test]
fn class_binding_merges_static_via_setclass() {
    let js = emit(
        "html:\n    <div class=\"base\" :class=\"{ active: on }\"></div>\nscript:\n    let on=true\n    function f(){ on=false }\n",
    );
    assert!(
        js.contains("setClass(e0, \"base\", { active: on.v });"),
        "{js}"
    );
    assert!(
        js.contains("class=\\\"base\\\""),
        "static class stays in HTML: {js}"
    );
}

#[test]
fn class_binding_without_static_passes_empty_base() {
    let js = emit(
        "html:\n    <div :class=\"cls\"></div>\nscript:\n    let cls=\"\"\n    function f(){ cls=\"x\" }\n",
    );
    assert!(js.contains("setClass(e0, \"\", cls.v);"), "{js}");
}

#[test]
fn style_binding_uses_setstyle() {
    let js = emit(
        "html:\n    <div :style=\"{ color: hue }\"></div>\nscript:\n    let hue=\"red\"\n    function f(){ hue=\"blue\" }\n",
    );
    assert!(js.contains("setStyle(e0, \"\", { color: hue.v });"), "{js}");
}

#[test]
fn style_binding_merges_static() {
    let js = emit(
        "html:\n    <div style=\"color: red\" :style=\"more\"></div>\nscript:\n    let more={}\n    function f(){ more={a:1} }\n",
    );
    assert!(
        js.contains("setStyle(e0, \"color: red\", more.v);"),
        "static style merged: {js}"
    );
}

// --- refs ------------------------------------------------------------------

#[test]
fn element_ref_assigns_node_no_bind() {
    let js = emit(
        "html:\n    <input :ref=\"el\">\nscript:\n    let el\n    function f(){ el.focus() }\n",
    );
    assert!(js.contains("el.v = e0;"), "ref assignment: {js}");
    assert!(
        !js.contains("() => { el.v = e0"),
        "ref is fixed, no bind: {js}"
    );
}

// --- :html -----------------------------------------------------------------

#[test]
fn html_bind_sets_inner_html() {
    let js = one_reactive("<div :html=\"n\"></div>");
    assert!(js.contains("e0.innerHTML = n.v;"), "{js}");
    assert!(js.contains("bind(c, [0]"), "reactive: {js}");
}

// --- dynamic :is component -------------------------------------------------

#[test]
fn dynamic_component_uses_dynamicblock() {
    let js = emit(
        "@use Foo from \"./Foo.lun\"\n@use Bar from \"./Bar.lun\"\nhtml:\n    <component :is=\"view\" :label=\"txt\"/>\nscript:\n    let view=Foo\n    let txt=\"\"\n    function f(){ view=Bar; txt=\"x\" }\n",
    );
    assert!(js.contains("import Foo from \"./Foo.lun\";"), "{js}");
    assert!(js.contains("import Bar from \"./Bar.lun\";"), "{js}");
    assert!(js.contains("dynamicBlock(c, a0, "), "{js}");
    assert!(js.contains("() => (view.v)"), "factory getter: {js}");
    assert!(js.contains("label: () => (txt.v)"), "prop getter: {js}");
    assert!(js.contains(".setProp(\"label\", txt.v)"), "{js}");
}

// --- teleport --------------------------------------------------------------

#[test]
fn teleport_static_target_is_selector_string() {
    let js = one_reactive("<teleport to=\"#modal\"><p>${n}</p></teleport>");
    assert!(js.contains("teleportBlock(c, a0, () => (`#modal`)"), "{js}");
    assert!(js.contains("fromHTML(HTML_1, a0)"), "{js}");
}

#[test]
fn teleport_bound_target_is_expression() {
    let js = emit(
        "html:\n    <teleport :to=\"target\"><span></span></teleport>\nscript:\n    let target=null\n    function f(){ target=1 }\n",
    );
    assert!(js.contains("teleportBlock(c, a0, () => (target.v)"), "{js}");
}

// --- fragments / multi-root ------------------------------------------------

#[test]
fn multi_root_uses_fragment_factory() {
    let js = emit(
        "html:\n    <h1>${a}</h1>\n    <p>${b}</p>\nscript:\n    let a=\"\"\n    let b=\"\"\n    function f(){ a=\"x\"; b=\"y\" }\n",
    );
    assert!(js.contains("import { fragment"), "{js}");
    assert!(
        js.contains("export default fragment({}, HTML, (c, props) => {"),
        "{js}"
    );
    assert!(
        !js.contains("component(\"div\""),
        "no wrapper element: {js}"
    );
    assert!(js.contains("const HTML = \"<h1></h1><p></p>\";"), "{js}");
}

#[test]
fn single_root_uses_component_factory() {
    let js = one_reactive("<p>${n}</p>");
    assert!(js.contains("component(\"div\""), "{js}");
    assert!(!js.contains("fragment("), "{js}");
}

#[test]
fn whitespace_and_comments_do_not_trigger_multi_root() {
    // A single element with surrounding whitespace/comment stays single-root.
    let js = emit("html:\n    <!-- hi -->\n    <p>only</p>\n");
    assert!(js.contains("component(\"div\""), "{js}");
    assert!(!js.contains("fragment("), "{js}");
}

// --- @input defaults -------------------------------------------------------

#[test]
fn prop_default_is_emitted_in_prop_helper() {
    let js = emit("@input start:number = 42\nhtml:\n    <p>${start}</p>\n");
    assert!(
        js.contains("const start = prop(c, \"start\", 0, props.start, (42));"),
        "default seeded into prop helper: {js}"
    );
}

#[test]
fn prop_without_default_passes_undefined() {
    let js = emit("@input label:string\nhtml:\n    <p>${label}</p>\n");
    assert!(
        js.contains("prop(c, \"label\", 0, props.label, undefined)"),
        "no default -> undefined: {js}"
    );
}

// --- non-reactive declared vars still emitted ------------------------------

#[test]
fn non_reactive_const_read_by_template_is_emitted_without_box() {
    let js = emit("html:\n    <p>${WIDTH}</p>\nscript:\n    const WIDTH = 5\n");
    assert!(js.contains("const WIDTH = 5"), "{js}");
    assert!(js.contains("t0.data = `${WIDTH}`;"), "{js}");
    assert!(!js.contains(", box"), "no box helper: {js}");
}

#[test]
fn helper_functions_are_preserved_verbatim() {
    // A non-reactive helper function used by an event handler must survive.
    let js = emit(
        "html:\n    <button @click=\"greet()\">x</button>\nscript:\n    let n=0\n    function greet(){ n = format(n) }\n    function format(x){ return x + 1 }\n",
    );
    assert!(
        js.contains("function format(x){ return x + 1 }"),
        "non-reactive helper preserved: {js}"
    );
}

// --- helper-name collision aliasing ---------------------------------------

#[test]
fn user_var_named_on_aliases_the_on_helper() {
    let js = emit(
        "html:\n    <button @click=\"flip()\" :if=\"on\">x</button>\nscript:\n    let on=false\n    function flip(){ on=!on }\n",
    );
    assert!(js.contains("on as $on"), "helper aliased in import: {js}");
    assert!(
        js.contains("const on = box(c, 0, false)"),
        "user var kept: {js}"
    );
    assert!(js.contains("$on(e0, \"click\""), "helper via alias: {js}");
    assert!(!js.contains(" on(e0"), "no bare on() call: {js}");
}

#[test]
fn user_var_named_bind_aliases_the_bind_helper() {
    // `bind` is both a helper and a reactive user var here.
    let js =
        emit("html:\n    <p>${bind}</p>\nscript:\n    let bind=0\n    function f(){ bind=1 }\n");
    assert!(js.contains("bind as $bind"), "bind helper aliased: {js}");
    // The text bind must be emitted via the alias, not the user var.
    assert!(js.contains("$bind(c, [0]"), "bind call via alias: {js}");
}

#[test]
fn user_var_named_box_aliases_box_constructor() {
    let js = emit("html:\n    <p>${box}</p>\nscript:\n    let box=0\n    function f(){ box=1 }\n");
    assert!(js.contains("box as $box"), "{js}");
    assert!(js.contains("const box = $box(c, 0, 0)"), "{js}");
}

#[test]
fn child_import_name_collision_is_aliased() {
    let js = emit(
        "@use bind from \"./bind.lunas\"\nhtml:\n    <div><bind :v=\"n\"/></div>\nscript:\n    let n=0\n    function f(){ n++ }\n",
    );
    assert!(js.contains("import bind$ from \"./bind.lunas\";"), "{js}");
    assert!(js.contains("mountChild(c, a0, bind$, {"), "{js}");
}

#[test]
fn unused_use_import_is_not_emitted() {
    let js = emit("@use Unused from \"./U.lunas\"\nhtml:\n    <p>hi</p>\n");
    assert!(!js.contains("import Unused"), "{js}");
}

// --- import line minimality ------------------------------------------------

#[test]
fn import_line_only_lists_referenced_helpers() {
    // A pure static template needs no bind/box/on helpers.
    let js = emit("html:\n    <p>static</p>\n");
    let import_line = js.lines().next().unwrap();
    assert!(import_line.contains("component"), "{import_line}");
    assert!(
        !import_line.contains("bind"),
        "no unused bind: {import_line}"
    );
    assert!(!import_line.contains("box"), "no unused box: {import_line}");
    assert!(!import_line.contains(" on"), "no unused on: {import_line}");
}

#[test]
fn no_template_emits_no_module() {
    let (js, diags) = compile("script:\n    let x = 0\n");
    assert!(js.is_none());
    assert!(!diags.iter().any(|d| d.is_error()));
}
