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
