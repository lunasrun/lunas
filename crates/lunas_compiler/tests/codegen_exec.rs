//! Execution test: compiles a component with `compile`, then *runs* the emitted
//! ES module against the real runtime (`packages/lunas/src`) under Node using a
//! dependency-free DOM shim. Verifies initial render and an event → state
//! change → text update.
//!
//! Skipped gracefully (with an eprintln) if Node is not available at the pinned
//! path, so CI without that toolchain still passes.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const NODE: &str = concat!(env!("HOME"), "/.nvm/versions/node/v22.18.0/bin/node");

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/lunas_compiler
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn node_available() -> bool {
    std::path::Path::new(NODE).exists()
}

/// Writes the emitted module + a driver script into `packages/lunas/test`, runs
/// node, returns stdout. The emitted module's `from "lunas"` import is rewritten
/// to the runtime's index so node resolves it without a package install.
fn run_component(name: &str, source: &str, driver_body: &str) -> String {
    let (js, diags) = lunas_compiler::compile(source);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "compile diagnostics: {diags:?}"
    );
    let js = js.expect("module emitted");
    let js = js.replace("from \"lunas\";", "from \"./src/index.mjs\";");

    let root = repo_root();
    let test_dir = root.join("packages/lunas");
    let mod_path = test_dir.join(format!("test/__exec_{name}.gen.mjs"));
    let drv_path = test_dir.join(format!("test/__exec_{name}.drv.mjs"));

    // The module imports "./src/index.mjs" — it lives at packages/lunas/, so
    // write it there (not under test/) to keep that relative path valid.
    let mod_at_root = test_dir.join(format!("__exec_{name}.gen.mjs"));
    std::fs::write(&mod_at_root, &js).unwrap();

    let driver = format!(
        "import {{ installDom }} from \"./test/dom-shim.mjs\";\n\
         import factory from \"./__exec_{name}.gen.mjs\";\n\
         const tick = () => new Promise((r) => setTimeout(r, 0));\n\
         installDom();\n\
         {driver_body}\n"
    );
    let drv_at_root = test_dir.join(format!("__exec_{name}.drv.mjs"));
    std::fs::write(&drv_at_root, driver).unwrap();

    let out = Command::new(NODE)
        .arg(&drv_at_root)
        .current_dir(&test_dir)
        .output()
        .expect("spawn node");

    // Clean up generated files.
    let _ = std::fs::remove_file(&mod_at_root);
    let _ = std::fs::remove_file(&drv_at_root);
    let _ = std::fs::remove_file(&mod_path);
    let _ = std::fs::remove_file(&drv_path);

    if !out.status.success() {
        std::io::stderr().write_all(&out.stderr).unwrap();
        panic!("node exited with failure for {name}");
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Compiles a parent + child pair (two `.lunas` sources) into two ES modules,
/// wires the parent's `@use` import (`child_use_path`, the path as written in
/// the parent's `@use`) to the generated child module, and runs the driver.
/// Both modules' `from "lunas"` imports are rewritten to the runtime index so
/// node resolves them without a package install.
fn run_parent_child(
    name: &str,
    parent_src: &str,
    child_src: &str,
    child_use_path: &str,
    driver_body: &str,
) -> String {
    let root = repo_root();
    let test_dir = root.join("packages/lunas");

    let compile = |src: &str| {
        let (js, diags) = lunas_compiler::compile(src);
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "compile diagnostics: {diags:?}"
        );
        js.expect("module emitted")
            .replace("from \"lunas\";", "from \"./src/index.mjs\";")
    };

    let child_file = format!("__exec_{name}_child.gen.mjs");
    let parent_js = compile(parent_src).replace(
        &format!("from \"{child_use_path}\";"),
        &format!("from \"./{child_file}\";"),
    );
    let child_js = compile(child_src);

    let parent_path = test_dir.join(format!("__exec_{name}.gen.mjs"));
    let child_path = test_dir.join(&child_file);
    std::fs::write(&parent_path, &parent_js).unwrap();
    std::fs::write(&child_path, &child_js).unwrap();

    let driver = format!(
        "import {{ installDom }} from \"./test/dom-shim.mjs\";\n\
         import factory from \"./__exec_{name}.gen.mjs\";\n\
         const tick = () => new Promise((r) => setTimeout(r, 0));\n\
         installDom();\n\
         {driver_body}\n"
    );
    let drv_path = test_dir.join(format!("__exec_{name}.drv.mjs"));
    std::fs::write(&drv_path, driver).unwrap();

    let out = Command::new(NODE)
        .arg(&drv_path)
        .current_dir(&test_dir)
        .output()
        .expect("spawn node");

    let _ = std::fs::remove_file(&parent_path);
    let _ = std::fs::remove_file(&child_path);
    let _ = std::fs::remove_file(&drv_path);

    if !out.status.success() {
        std::io::stderr().write_all(&out.stderr).unwrap();
        panic!("node exited with failure for {name}");
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn counter_renders_and_reacts() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "@input start:number = 0\n\
        html:\n\
        \x20   <button @click=\"inc()\">count: ${count}</button>\n\
        script:\n\
        \x20   let count = start\n\
        \x20   function inc(){ count++ }\n";

    let driver = "\
        const root = factory({ start: 2 });\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";

    let out = run_component("counter", source, driver);
    assert!(out.contains("INITIAL:count: 2"), "initial render: {out}");
    assert!(out.contains("AFTER:count: 3"), "post-event render: {out}");
}

#[test]
fn attr_bind_reacts() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <input :value=\"name\" @focus=\"clear()\">\n\
        script:\n\
        \x20   let name = \"alice\"\n\
        \x20   function clear(){ name = \"\" }\n";

    let driver = "\
        const root = factory({});\n\
        const input = root.childNodes[0];\n\
        console.log('INITIAL:' + input.value);\n\
        input.dispatch('focus');\n\
        await tick();\n\
        console.log('AFTER:[' + input.value + ']');\n";

    let out = run_component("attr", source, driver);
    assert!(out.contains("INITIAL:alice"), "initial value: {out}");
    assert!(out.contains("AFTER:[]"), "post-event value: {out}");
}

#[test]
fn mixed_text_anchor_updates_in_place() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Interpolation between two static text runs: the anchor is a split text
    // node; only the dynamic node's data changes on update.
    let source = "html:\n\
        \x20   <p>a=${a} b=${b}!</p>\n\
        script:\n\
        \x20   let a = 1\n\
        \x20   let b = 2\n\
        \x20   function bumpA(){ a = 9 }\n";

    let driver = "\
        const root = factory({});\n\
        const p = root.childNodes[0];\n\
        console.log('INITIAL:' + p.innerHTMLString());\n";

    let out = run_component("mixed", source, driver);
    assert!(out.contains("INITIAL:a=1 b=2!"), "mixed render: {out}");
}

#[test]
fn two_way_input_writes_back_to_state() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // `::value` binds the property from state AND writes back on input. A text
    // node mirrors the state so we can see the write-back land.
    let source = "html:\n\
        \x20   <div><input ::value=\"name\"><span>${name}</span></div>\n\
        script:\n\
        \x20   let name = \"a\"\n";

    let driver = "\
        const root = factory({});\n\
        const input = root.childNodes[0].childNodes[0];\n\
        const span = root.childNodes[0].childNodes[1];\n\
        console.log('INITIAL:' + input.value + '/' + span.innerHTMLString());\n\
        input.value = 'zed';\n\
        input.dispatch('input');\n\
        await tick();\n\
        console.log('AFTER:' + span.innerHTMLString());\n";

    let out = run_component("twoway", source, driver);
    assert!(out.contains("INITIAL:a/a"), "initial reflects state: {out}");
    assert!(
        out.contains("AFTER:zed"),
        "input write-back updates state and dependent text: {out}"
    );
}

#[test]
fn if_toggles_branch_on_event() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <div><button @click=\"toggle()\">t</button><span :if=\"on\">HERE</span></div>\n\
        script:\n\
        \x20   let on = false\n\
        \x20   function toggle(){ on = !on }\n";

    let driver = "\
        const root = factory({});\n\
        const box = root.childNodes[0];\n\
        const btn = box.childNodes[0];\n\
        console.log('INITIAL:' + box.innerHTMLString());\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('SHOWN:' + box.innerHTMLString());\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('HIDDEN:' + box.innerHTMLString());\n";

    let out = run_component("iftoggle", source, driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("INITIAL:") && !l.contains("HERE")),
        "initially hidden: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("SHOWN:") && l.contains("<span>HERE</span>")),
        "branch built and inserted on toggle: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("HIDDEN:") && !l.contains("HERE")),
        "branch removed on toggle back: {out}"
    );
}

#[test]
fn for_initial_push_splice_and_reorder() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Keyed :for over a deeply-mutated array. Mutations are driven through
    // button click handlers (the only way to reach setup-local state from the
    // driver): .push/.splice and a whole-array reassignment (reorder) all mark
    // the list reactive; the keyed reconciler preserves node identity for
    // surviving keys across a reorder.
    let source = "html:\n\
        \x20   <div>\n\
        \x20     <button @click=\"push3()\">p</button>\n\
        \x20     <button @click=\"drop2()\">d</button>\n\
        \x20     <button @click=\"reorder()\">r</button>\n\
        \x20     <ul><li :for=\"item of items\" :key=\"item.id\">${item.label}</li></ul>\n\
        \x20   </div>\n\
        script:\n\
        \x20   let items = [{id:1,label:\"a\"},{id:2,label:\"b\"}]\n\
        \x20   function push3(){ items.push({id:3,label:\"c\"}) }\n\
        \x20   function drop2(){ items.splice(1, 1) }\n\
        \x20   function reorder(){ items = [items[1], items[0]] }\n";

    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const [pBtn, dBtn, rBtn, ul] = div.childNodes;\n\
        const lis = () => ul.childNodes.filter(n => n.tag === 'li');\n\
        const labels = () => lis().map(n => n.innerHTMLString()).join(',');\n\
        console.log('INITIAL:' + labels());\n\
        const [firstA, firstB] = lis();\n\
        pBtn.dispatch('click'); await tick();\n\
        console.log('PUSH:' + labels());\n\
        dBtn.dispatch('click'); await tick();\n\
        console.log('SPLICE:' + labels());\n\
        rBtn.dispatch('click'); await tick();\n\
        const after = lis();\n\
        console.log('REORDER:' + labels());\n\
        console.log('IDENTITY:' + (after[after.length-1] === firstA));\n";

    let out = run_component("forkeyed", source, driver);
    assert!(out.contains("INITIAL:a,b"), "initial bulk render: {out}");
    assert!(out.contains("PUSH:a,b,c"), "push appends: {out}");
    assert!(
        out.contains("SPLICE:a,c"),
        "splice removes the middle: {out}"
    );
    assert!(
        out.contains("REORDER:c,a"),
        "reorder reflects new order: {out}"
    );
    assert!(
        out.contains("IDENTITY:true"),
        "surviving key keeps its DOM node across a reorder: {out}"
    );
}

// --- child components -----------------------------------------------------

#[test]
fn child_renders_and_receives_reactive_props() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Parent holds `n`, passes it as a reactive prop to Child, which renders it
    // as text. A parent event changes `n`; the child's own text must update
    // (parent -> child reactivity end to end). A child event changes only the
    // child's local state, never the parent's `n`.
    let parent = "@use Child from \"./Child.lunas\"\n\
        html:\n\
        \x20   <div><button @click=\"inc()\">p</button><Child :count=\"n\"/></div>\n\
        script:\n\
        \x20   let n = 1\n\
        \x20   function inc(){ n++ }\n";
    let child = "@input count:number = 0\n\
        html:\n\
        \x20   <section><button @click=\"bump()\">c</button><span>${count}+${local}</span></section>\n\
        script:\n\
        \x20   let local = 0\n\
        \x20   function bump(){ local++ }\n";

    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const pBtn = div.childNodes[0];\n\
        const childRoot = div.childNodes[1];\n\
        const section = childRoot.childNodes[0];\n\
        const cBtn = section.childNodes[0];\n\
        const span = section.childNodes[1];\n\
        console.log('INITIAL:' + span.innerHTMLString());\n\
        pBtn.dispatch('click'); await tick();\n\
        console.log('AFTER_PARENT:' + span.innerHTMLString());\n\
        cBtn.dispatch('click'); await tick();\n\
        console.log('AFTER_CHILD:' + span.innerHTMLString());\n\
        // A second parent change must still land (getter/box stays live).\n\
        pBtn.dispatch('click'); await tick();\n\
        console.log('AFTER_PARENT2:' + span.innerHTMLString());\n";

    let out = run_parent_child("child_props", parent, child, "./Child.lunas", driver);
    assert!(
        out.contains("INITIAL:1+0"),
        "child renders the seeded prop and its own state: {out}"
    );
    assert!(
        out.contains("AFTER_PARENT:2+0"),
        "parent state change propagates to child text: {out}"
    );
    assert!(
        out.contains("AFTER_CHILD:2+1"),
        "child event mutates only child-local state (prop unchanged): {out}"
    );
    assert!(
        out.contains("AFTER_PARENT2:3+1"),
        "prop stays live across multiple parent changes: {out}"
    );
}

#[test]
fn child_emit_drives_parent_state() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Child raises `emit("changed", 1)` from a click. The parent listens with
    // `@changed="onChanged($event)"`, whose handler mutates parent state — that
    // write (a parent box setter) is what re-renders the parent text. This is
    // the full child → parent event round-trip (output-design.md §5, c-emits).
    let parent = "@use Child from \"./Child.lunas\"\n\
        html:\n\
        \x20   <div><Child @changed=\"onChanged($event)\"/><p>total: ${total}</p></div>\n\
        script:\n\
        \x20   let total = 0\n\
        \x20   function onChanged(n){ total = total + n }\n";
    let child = "html:\n\
        \x20   <button @click=\"bump()\">c</button>\n\
        script:\n\
        \x20   function bump(){ emit(\"changed\", 1) }\n";

    // Layout: outer div (root) > inner div > [childRoot(button), anchor, p].
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const cBtn = div.childNodes[0].childNodes[0];\n\
        const p = div.childNodes[2];\n\
        console.log('INITIAL:' + p.innerHTMLString());\n\
        cBtn.dispatch('click'); await tick();\n\
        console.log('AFTER_EMIT:' + p.innerHTMLString());\n\
        cBtn.dispatch('click'); await tick();\n\
        console.log('AFTER_EMIT2:' + p.innerHTMLString());\n";

    let out = run_parent_child("child_emit", parent, child, "./Child.lunas", driver);
    assert!(
        out.contains("INITIAL:total: 0"),
        "parent renders initial state: {out}"
    );
    assert!(
        out.contains("AFTER_EMIT:total: 1"),
        "child emit runs the parent handler, which mutates parent state: {out}"
    );
    assert!(
        out.contains("AFTER_EMIT2:total: 2"),
        "the emit channel stays live across repeated events: {out}"
    );
}

#[test]
fn child_emit_payload_and_no_listener_are_safe() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Two facets in one round-trip: (1) an object payload is delivered intact to
    // the parent handler as `$event`; (2) a second event the parent does NOT
    // listen for is a no-op (emit returns false, nothing throws).
    let parent = "@use Child from \"./Child.lunas\"\n\
        html:\n\
        \x20   <div><Child @save=\"onSave($event)\"/><p>${label}:${count}</p></div>\n\
        script:\n\
        \x20   let label = \"none\"\n\
        \x20   let count = 0\n\
        \x20   function onSave(e){ label = e.name; count = e.n }\n";
    let child = "html:\n\
        \x20   <button @click=\"go()\">c</button>\n\
        script:\n\
        \x20   function go(){ emit(\"save\", { name: \"a\", n: 7 }); emit(\"unheard\", 1) }\n";

    // Layout: outer div (root) > inner div > [childRoot(button), anchor, p].
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const cBtn = div.childNodes[0].childNodes[0];\n\
        const p = div.childNodes[2];\n\
        console.log('INITIAL:' + p.innerHTMLString());\n\
        cBtn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + p.innerHTMLString());\n";

    let out = run_parent_child("child_emit_payload", parent, child, "./Child.lunas", driver);
    assert!(
        out.contains("INITIAL:none:0"),
        "initial parent render: {out}"
    );
    assert!(
        out.contains("AFTER:a:7"),
        "object payload delivered as $event and an unlistened event is a no-op: {out}"
    );
}

#[test]
fn child_uses_default_when_prop_omitted() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Parent mounts Child with NO props; the child's `@input` default seeds it.
    let parent = "@use Child from \"./Child.lunas\"\n\
        html:\n\
        \x20   <div><Child/></div>\n";
    let child = "@input label:string = \"def\"\n\
        html:\n\
        \x20   <span>${label}</span>\n";

    let driver = "\
        const root = factory({});\n\
        const childRoot = root.childNodes[0].childNodes[0];\n\
        const span = childRoot.childNodes[0];\n\
        console.log('INITIAL:' + span.innerHTMLString());\n";

    let out = run_parent_child("child_default", parent, child, "./Child.lunas", driver);
    assert!(
        out.contains("INITIAL:def"),
        "omitted prop falls back to the @input default: {out}"
    );
}

#[test]
fn child_in_if_mounts_and_unmounts() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Child lives inside an `:if`. Toggling the condition mounts and unmounts it
    // (scope teardown): the child's DOM appears and disappears.
    let parent = "@use Child from \"./Child.lunas\"\n\
        html:\n\
        \x20   <div><button @click=\"toggle()\">t</button><span :if=\"on\"><Child :v=\"n\"/></span></div>\n\
        script:\n\
        \x20   let on = false\n\
        \x20   let n = 5\n\
        \x20   function toggle(){ on = !on }\n";
    let child = "@input v:number = 0\n\
        html:\n\
        \x20   <b>${v}</b>\n";

    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const btn = div.childNodes[0];\n\
        console.log('INITIAL:' + div.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('SHOWN:' + div.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('HIDDEN:' + div.innerHTMLString());\n";

    let out = run_parent_child("child_in_if", parent, child, "./Child.lunas", driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("INITIAL:") && !l.contains("<b>")),
        "child absent before the :if is true: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("SHOWN:") && l.contains("<b>5</b>")),
        "child mounts with its prop when the branch shows: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("HIDDEN:") && !l.contains("<b>")),
        "child unmounts when the branch is removed: {out}"
    );
}

// --- DOM feature batch exec tests --------------------------------------------

#[test]
fn class_binding_reacts_and_merges_static() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <div class=\"base\" :class=\"{ active: on }\" @click=\"toggle()\"></div>\n\
        script:\n\
        \x20   let on = false\n\
        \x20   function toggle(){ on = !on }\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INITIAL:[' + div.getAttribute('class') + ']');\n\
        div.dispatch('click'); await tick();\n\
        console.log('AFTER:[' + div.getAttribute('class') + ']');\n";
    let out = run_component("classbind", source, driver);
    assert!(
        out.contains("INITIAL:[base]"),
        "static only when inactive: {out}"
    );
    assert!(
        out.contains("AFTER:[base active]"),
        "merged when active: {out}"
    );
}

#[test]
fn style_binding_reacts() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <div :style=\"{ color: hue }\" @click=\"go()\"></div>\n\
        script:\n\
        \x20   let hue = \"red\"\n\
        \x20   function go(){ hue = \"blue\" }\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INITIAL:[' + div.getAttribute('style') + ']');\n\
        div.dispatch('click'); await tick();\n\
        console.log('AFTER:[' + div.getAttribute('style') + ']');\n";
    let out = run_component("stylebind", source, driver);
    assert!(
        out.contains("INITIAL:[color: red;]"),
        "initial style: {out}"
    );
    assert!(
        out.contains("AFTER:[color: blue;]"),
        "reactive style: {out}"
    );
}

#[test]
fn html_binding_reacts() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <div :html=\"raw\" @click=\"go()\"></div>\n\
        script:\n\
        \x20   let raw = \"<b>hi</b>\"\n\
        \x20   function go(){ raw = \"<i>bye</i>\" }\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INITIAL:' + div.innerHTMLString());\n\
        div.dispatch('click'); await tick();\n\
        console.log('AFTER:' + div.innerHTMLString());\n";
    let out = run_component("htmlbind", source, driver);
    assert!(
        out.contains("INITIAL:<b>hi</b>"),
        "raw html rendered: {out}"
    );
    assert!(out.contains("AFTER:<i>bye</i>"), "raw html reacts: {out}");
}

#[test]
fn ref_exposes_element_to_script() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // The ref box receives the element; a handler reads it and mutates the DOM.
    let source = "html:\n\
        \x20   <div><input :ref=\"el\"><button @click=\"fill()\">f</button></div>\n\
        script:\n\
        \x20   let el\n\
        \x20   function fill(){ el.value = \"set\" }\n";
    let driver = "\
        const root = factory({});\n\
        const box = root.childNodes[0];\n\
        const input = box.childNodes[0];\n\
        const btn = box.childNodes[1];\n\
        console.log('INITIAL:[' + input.value + ']');\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:[' + input.value + ']');\n";
    let out = run_component("refbind", source, driver);
    assert!(out.contains("INITIAL:[]"), "empty before: {out}");
    assert!(
        out.contains("AFTER:[set]"),
        "ref lets script reach the element: {out}"
    );
}

#[test]
fn multi_root_fragment_renders_all_roots() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <h1>${title}</h1>\n\
        \x20   <p @click=\"go()\">${body}</p>\n\
        script:\n\
        \x20   let title = \"T\"\n\
        \x20   let body = \"B\"\n\
        \x20   function go(){ body = \"B2\" }\n";
    // A fragment factory returns an array of nodes; mount them into a container.
    let driver = "\
        const frag = factory({});\n\
        const host = document.createElement('div');\n\
        for (const n of frag) host.appendChild(n);\n\
        console.log('COUNT:' + frag.length);\n\
        console.log('INITIAL:' + host.innerHTMLString());\n\
        host.childNodes[1].dispatch('click'); await tick();\n\
        console.log('AFTER:' + host.innerHTMLString());\n";
    let out = run_component("fragroot", source, driver);
    assert!(out.contains("COUNT:2"), "two top-level roots: {out}");
    assert!(
        out.contains("INITIAL:<h1>T</h1><p>B</p>"),
        "both roots render: {out}"
    );
    assert!(
        out.contains("AFTER:<h1>T</h1><p>B2</p>"),
        "fragment reacts: {out}"
    );
}

#[test]
fn teleport_renders_into_target() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    let source = "html:\n\
        \x20   <div><teleport to=\"#portal\"><p @click=\"go()\">${msg}</p></teleport></div>\n\
        script:\n\
        \x20   let msg = \"hi\"\n\
        \x20   function go(){ msg = \"bye\" }\n";
    // A #portal target attached to document.body; the teleport content lands
    // there, not inside the component's inline <div>.
    let driver = "\
        const portal = document.createElement('div');\n\
        portal.setAttribute('id', 'portal');\n\
        document.body.appendChild(portal);\n\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INLINE:' + div.innerHTMLString());\n\
        console.log('PORTAL:' + portal.innerHTMLString());\n\
        portal.childNodes[0].dispatch('click'); await tick();\n\
        console.log('REACT:' + portal.innerHTMLString());\n";
    let out = run_component("teleport", source, driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("INLINE:") && !l.contains("<p>")),
        "content is not inline: {out}"
    );
    assert!(
        out.contains("PORTAL:<p>hi</p>"),
        "content rendered into #portal: {out}"
    );
    assert!(
        out.contains("REACT:<p>bye</p>"),
        "teleported content stays reactive: {out}"
    );
}

#[test]
fn dynamic_component_mounts_and_passes_prop() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Parent has a `<component :is="view">` where `view` starts as the @use
    // child factory. mountChild-style prop passing must work through :is.
    let parent = "@use Kid from \"./Kid.lunas\"\n\
        html:\n\
        \x20   <div><component :is=\"view\" :label=\"txt\"/></div>\n\
        script:\n\
        \x20   let view = Kid\n\
        \x20   let txt = \"hello\"\n\
        \x20   function drop(){ view = null }\n";
    let child = "@input label:string = \"\"\n\
        html:\n\
        \x20   <b>${label}</b>\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INITIAL:' + div.innerHTMLString());\n";
    let out = run_parent_child("dyncomp", parent, child, "./Kid.lunas", driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("INITIAL:") && l.contains("<b>hello</b>")),
        "dynamic component mounts the :is factory with its prop: {out}"
    );
}

#[test]
fn multi_root_fragment_with_top_level_if() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // A top-level :if inside a fragment: its anchor is created against the host
    // and must travel with the fragment node group (snapshot-after-setup), so
    // the branch toggles correctly once the nodes are mounted elsewhere.
    let source = "html:\n\
        \x20   <h1 @click=\"go()\">${t}</h1>\n\
        \x20   <p :if=\"show\">shown</p>\n\
        script:\n\
        \x20   let t = \"T\"\n\
        \x20   let show = true\n\
        \x20   function go(){ show = !show }\n";
    let driver = "\
        const frag = factory({});\n\
        const host = document.createElement('div');\n\
        for (const n of frag) host.appendChild(n);\n\
        console.log('INITIAL:' + host.innerHTMLString());\n\
        host.childNodes[0].dispatch('click'); await tick();\n\
        console.log('HIDDEN:' + host.innerHTMLString());\n\
        host.childNodes[0].dispatch('click'); await tick();\n\
        console.log('SHOWN:' + host.innerHTMLString());\n";
    let out = run_component("fragif", source, driver);
    assert!(
        out.contains("INITIAL:<h1>T</h1><p>shown</p>"),
        "branch shown initially: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("HIDDEN:") && !l.contains("shown")),
        "branch removed on toggle: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("SHOWN:") && l.contains("shown")),
        "branch re-inserted at the right slot: {out}"
    );
}

#[test]
fn component_ref_exposes_mount_handle() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // `:ref="kid"` on a child exposes the mountChild handle to the parent
    // script, which can drive the child via setProp.
    let parent = "@use Kid from \"./Kid.lunas\"\n\
        html:\n\
        \x20   <div><Kid :label=\"txt\" :ref=\"kid\"/><button @click=\"push()\">p</button></div>\n\
        script:\n\
        \x20   let txt = \"a\"\n\
        \x20   let kid\n\
        \x20   function push(){ kid.setProp(\"label\", \"b\") }\n";
    let child = "@input label:string = \"\"\n\
        html:\n\
        \x20   <b>${label}</b>\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        console.log('INITIAL:' + div.innerHTMLString());\n\
        const btn = div.childNodes.find((n) => n.tag === 'button');\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + div.innerHTMLString());\n";
    let out = run_parent_child("compref", parent, child, "./Kid.lunas", driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("INITIAL:") && l.contains("<b>a</b>")),
        "child mounts: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("AFTER:") && l.contains("<b>b</b>")),
        "parent drives child through the ref handle: {out}"
    );
}

// --- slots (c-slots) --------------------------------------------------------

#[test]
fn slot_default_content_renders_in_child() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Parent passes default-slot content; the child's <slot> renders it.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><Card>hello ${name}</Card></div>\n\
        script:\n\
        \x20   let name = \"world\"\n\
        \x20   function chg(){ name = \"lunas\" }\n";
    let child = "html:\n\
        \x20   <section><slot>fallback</slot></section>\n";

    let driver = "\
        const root = factory({});\n\
        const card = root.childNodes[0].childNodes[0];\n\
        console.log('INITIAL:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_default", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("INITIAL:<section>hello world</section>"),
        "parent default content renders in child slot: {out}"
    );
}

#[test]
fn slot_fallback_when_no_content_given() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Parent mounts the child with NO content; the slot shows its own fallback.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><Card/></div>\n";
    let child = "html:\n\
        \x20   <section><slot>fallback text</slot></section>\n";

    let driver = "\
        const root = factory({});\n\
        const card = root.childNodes[0].childNodes[0];\n\
        console.log('INITIAL:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_fallback", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("INITIAL:<section>fallback text</section>"),
        "fallback shows when parent gives no content: {out}"
    );
}

#[test]
fn slot_parent_state_updates_content_in_place() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Slot content reads parent state; a parent change updates it in place —
    // the parent's reactivity drives content living inside the child.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><button @click=\"inc()\">p</button><Card><span>n=${n}</span></Card></div>\n\
        script:\n\
        \x20   let n = 1\n\
        \x20   function inc(){ n++ }\n";
    let child = "html:\n\
        \x20   <section><slot></slot></section>\n";

    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const btn = div.childNodes[0];\n\
        const card = div.childNodes[1];\n\
        console.log('INITIAL:' + card.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_reactive", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("INITIAL:<section><span>n=1</span></section>"),
        "initial slot content: {out}"
    );
    assert!(
        out.contains("AFTER:<section><span>n=2</span></section>"),
        "parent state change updated slot content in place: {out}"
    );
}

#[test]
fn slot_named_routing() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Default + a named slot via <template #foot>; each routes to its outlet.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><Card>body here<template #foot>the footer</template></Card></div>\n";
    let child = "html:\n\
        \x20   <section><main><slot></slot></main><footer><slot name=\"foot\"></slot></footer></section>\n";

    let driver = "\
        const root = factory({});\n\
        const card = root.childNodes[0].childNodes[0];\n\
        console.log('OUT:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_named", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("<main>body here</main>"),
        "default slot routed to <main>: {out}"
    );
    assert!(
        out.contains("<footer>the footer</footer>"),
        "named slot routed to <footer>: {out}"
    );
}

#[test]
fn slot_scoped_props_flow_up() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // The child exposes a scoped prop `:item` on its <slot>; the parent's
    // <template #default="p"> reads it.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><Card><template #default=\"p\">got ${p.item}</template></Card></div>\n";
    let child = "html:\n\
        \x20   <section><slot :item=\"row\"></slot></section>\n\
        script:\n\
        \x20   let row = \"apple\"\n";

    let driver = "\
        const root = factory({});\n\
        const card = root.childNodes[0].childNodes[0];\n\
        console.log('OUT:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_scoped", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("<section>got apple</section>"),
        "scoped slot prop flowed from child up into parent content: {out}"
    );
}

#[test]
fn slot_teardown_on_child_unmount() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // A child carrying slot content lives inside an :if. Toggling off unmounts
    // the child; the parent-owned slot binds must not fire afterwards (no leak,
    // no late write). We prove it by continuing to mutate parent state after
    // teardown and checking the (removed) content never errors and the child is
    // gone.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><button @click=\"t()\">t</button><button @click=\"inc()\">i</button>\
        <span :if=\"on\"><Card><b>n=${n}</b></Card></span></div>\n\
        script:\n\
        \x20   let on = true\n\
        \x20   let n = 1\n\
        \x20   function t(){ on = !on }\n\
        \x20   function inc(){ n++ }\n";
    let child = "html:\n\
        \x20   <section><slot></slot></section>\n";

    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const tBtn = div.childNodes[0];\n\
        const iBtn = div.childNodes[1];\n\
        console.log('SHOWN:' + div.innerHTMLString());\n\
        tBtn.dispatch('click'); await tick();\n\
        console.log('HIDDEN:' + div.innerHTMLString());\n\
        // Mutating parent state after unmount must not throw or resurrect content.\n\
        iBtn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + div.innerHTMLString());\n";

    let out = run_parent_child("slot_teardown", parent, child, "./Card.lunas", driver);
    assert!(
        out.lines()
            .any(|l| l.starts_with("SHOWN:") && l.contains("<section><b>n=1</b></section>")),
        "slot content shown while child mounted: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("HIDDEN:") && !l.contains("<section>")),
        "child + slot content removed on unmount: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("AFTER:") && !l.contains("<section>")),
        "post-unmount parent mutation does not resurrect content: {out}"
    );
}

#[test]
fn slot_component_in_slot_content() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Slot content that itself contains a child component (component-in-slot):
    // the inner component mounts inside the outer child's slot outlet.
    let parent = "@use Card from \"./Card.lunas\"\n\
        html:\n\
        \x20   <div><Card>wrap ${label}</Card></div>\n\
        script:\n\
        \x20   let label = \"x\"\n";
    let child = "html:\n\
        \x20   <section><slot></slot></section>\n";

    let driver = "\
        const root = factory({});\n\
        const card = root.childNodes[0].childNodes[0];\n\
        console.log('OUT:' + card.innerHTMLString());\n";

    let out = run_parent_child("slot_nested", parent, child, "./Card.lunas", driver);
    assert!(
        out.contains("<section>wrap x</section>"),
        "slot content renders: {out}"
    );
}

// --- compiler-injected name hygiene (c / props collisions) -----------------

#[test]
fn user_var_named_c_reads_and_mutates() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // A top-level `let c` collides with the injected runtime-context param `c`.
    // Before the fix this emitted `const c = box(c, …)` → SyntaxError. It must
    // now compile, render `${c}`, and update when a handler mutates `c`. Refs,
    // events, `:if`, and `:for` are all present to exercise every generated
    // local against the mangled context name.
    let source = "html:\n\
        \x20   <div>\n\
        \x20     <button @click=\"inc()\">c=${c}</button>\n\
        \x20     <span :if=\"c > 1\">big</span>\n\
        \x20     <ul><li :for=\"n of list\" :key=\"n\">${n}</li></ul>\n\
        \x20   </div>\n\
        script:\n\
        \x20   let c = 1\n\
        \x20   let list = [1, 2]\n\
        \x20   function inc(){ c++ }\n";

    let driver = "\
        const root = factory({});\n\
        const box = root.childNodes[0];\n\
        const btn = box.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        console.log('IFINIT:' + box.innerHTMLString().includes('big'));\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n\
        console.log('IFAFTER:' + box.innerHTMLString().includes('big'));\n";

    let out = run_component("hygiene_c", source, driver);
    assert!(out.contains("INITIAL:c=1"), "initial render: {out}");
    assert!(out.contains("IFINIT:false"), "if hidden while c<=1: {out}");
    assert!(
        out.contains("AFTER:c=2"),
        "text updates after mutate c: {out}"
    );
    assert!(out.contains("IFAFTER:true"), "if shows after c>1: {out}");
}

#[test]
fn user_var_named_props_reads_and_mutates() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // A top-level `let props` collides with the injected props param `props`.
    let source = "html:\n\
        \x20   <button @click=\"bump()\">props=${props}</button>\n\
        script:\n\
        \x20   let props = 5\n\
        \x20   function bump(){ props += 10 }\n";

    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";

    let out = run_component("hygiene_props", source, driver);
    assert!(out.contains("INITIAL:props=5"), "initial render: {out}");
    assert!(
        out.contains("AFTER:props=15"),
        "mutate props updates: {out}"
    );
}

#[test]
fn injected_c_collides_with_real_input_prop() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // The collision also fires when the *prop* is named `c`/`props`: the prop is
    // boxed via `prop($$c, "c", …, props.c, …)` and read as `.v`.
    let source = "@input c:number = 0\n\
        html:\n\
        \x20   <p>c=${c}</p>\n";

    let driver = "\
        const root = factory({ c: 7 });\n\
        console.log('INITIAL:' + root.childNodes[0].innerHTMLString());\n";

    let out = run_component("hygiene_prop_c", source, driver);
    assert!(out.contains("INITIAL:c=7"), "prop named c renders: {out}");
}

#[test]
fn user_var_named_e0_collides_with_generated_ref() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // A top-level `let e0` collides with a generated element-ref local `e0`.
    // Every generated local switches to the reserved `$$` prefix so both
    // coexist. An event handler forces a real `e0` ref to be emitted.
    let source = "html:\n\
        \x20   <button @click=\"inc()\">e0=${e0}</button>\n\
        script:\n\
        \x20   let e0 = 3\n\
        \x20   function inc(){ e0++ }\n";

    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click');\n\
        await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";

    let out = run_component("hygiene_e0", source, driver);
    assert!(out.contains("INITIAL:e0=3"), "initial render: {out}");
    assert!(out.contains("AFTER:e0=4"), "mutate e0 updates: {out}");
}

// --- inline template-handler mutations (fix/inline-handler-mutations) --------
//
// A mutation written INLINE in an `@event` handler (`@click="n = n + 1"`,
// `@click="count++"`, `@click="obj.k = 1"`, `@click="a++; b++"`) must be
// recognized as a reactive write and compiled through the `.v` box-setter path,
// so the DOM updates on click — matching Vue/Svelte. Previously such mutations
// were silently ignored unless routed through a named script function.

#[test]
fn inline_handler_assignment_updates_dom() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // `n = n + 1` written inline, `n` never assigned in script.
    let source = "html:\n\
        \x20   <button @click=\"n = n + 1\">count: ${n}</button>\n\
        script:\n\
        \x20   let n = 0\n";
    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";
    let out = run_component("inline_assign", source, driver);
    assert!(out.contains("INITIAL:count: 0"), "initial render: {out}");
    assert!(
        out.contains("AFTER:count: 2"),
        "inline assign updates DOM: {out}"
    );
}

#[test]
fn inline_handler_increment_updates_dom() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // `n++` written inline.
    let source = "html:\n\
        \x20   <button @click=\"n++\">count: ${n}</button>\n\
        script:\n\
        \x20   let n = 5\n";
    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";
    let out = run_component("inline_incr", source, driver);
    assert!(out.contains("INITIAL:count: 5"), "initial render: {out}");
    assert!(
        out.contains("AFTER:count: 6"),
        "inline ++ updates DOM: {out}"
    );
}

#[test]
fn inline_handler_deep_member_assignment_updates_dom() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // `obj.k = 1` (deep member write): the target must be a deepBox so the
    // in-place mutation marks the var.
    let source = "html:\n\
        \x20   <button @click=\"obj.k = obj.k + 10\">v: ${obj.k}</button>\n\
        script:\n\
        \x20   let obj = { k: 1 }\n";
    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";
    let out = run_component("inline_deep", source, driver);
    assert!(out.contains("INITIAL:v: 1"), "initial render: {out}");
    assert!(
        out.contains("AFTER:v: 11"),
        "inline member assign updates DOM: {out}"
    );
}

#[test]
fn inline_handler_multiple_statements_update_dom() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // Two statements in one handler: `a++; b++`.
    let source = "html:\n\
        \x20   <button @click=\"a++; b++\">${a}-${b}</button>\n\
        script:\n\
        \x20   let a = 0\n\
        \x20   let b = 10\n";
    let driver = "\
        const root = factory({});\n\
        const btn = root.childNodes[0];\n\
        console.log('INITIAL:' + btn.innerHTMLString());\n\
        btn.dispatch('click'); await tick();\n\
        console.log('AFTER:' + btn.innerHTMLString());\n";
    let out = run_component("inline_multi", source, driver);
    assert!(out.contains("INITIAL:0-10"), "initial render: {out}");
    assert!(
        out.contains("AFTER:1-11"),
        "multi-statement handler updates DOM: {out}"
    );
}

#[test]
fn inline_and_function_call_handlers_mix() {
    if !node_available() {
        eprintln!("skipping codegen_exec: node not found at {NODE}");
        return;
    }
    // One button mutates inline; another calls a script function. Both must work
    // and drive the same reactive state.
    let source = "html:\n\
        \x20   <div><button @click=\"n = n + 1\">inc</button><button @click=\"reset()\">reset</button><p>${n}</p></div>\n\
        script:\n\
        \x20   let n = 0\n\
        \x20   function reset(){ n = 0 }\n";
    let driver = "\
        const root = factory({});\n\
        const div = root.childNodes[0];\n\
        const btns = div.childNodes.filter((c) => c.tag === 'button');\n\
        const p = div.childNodes.find((c) => c.tag === 'p');\n\
        console.log('INITIAL:' + p.innerHTMLString());\n\
        btns[0].dispatch('click'); await tick();\n\
        btns[0].dispatch('click'); await tick();\n\
        console.log('MID:' + p.innerHTMLString());\n\
        btns[1].dispatch('click'); await tick();\n\
        console.log('AFTER:' + p.innerHTMLString());\n";
    let out = run_component("inline_mixed", source, driver);
    assert!(out.contains("INITIAL:0"), "initial render: {out}");
    assert!(out.contains("MID:2"), "inline handler drives state: {out}");
    assert!(
        out.contains("AFTER:0"),
        "function-call handler still works: {out}"
    );
}
