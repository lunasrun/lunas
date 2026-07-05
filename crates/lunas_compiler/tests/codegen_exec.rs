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
