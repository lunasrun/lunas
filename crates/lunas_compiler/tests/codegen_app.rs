//! Full-app **integration** test: compiles a realistic multi-component app
//! (parent `App` + `Counter` / `Card` / `Badge` children, wired through `@use`,
//! exercising `@input` props, default/named/scoped slots, keyed `:for` of child
//! components, an `:if`/`:elseif`/`:else` page cascade, two-way binding, and a
//! `<component :is>`) and runs the emitted ES modules against the REAL runtime
//! (`packages/lunas/src`) under the dependency-free node dom-shim.
//!
//! It then drives an app-sized journey — mount, read initial DOM, fire events,
//! mutate an array (`:for` grows), navigate the `:if` cascade, two-way-edit a
//! child input — and asserts the rendered DOM at each step. This extends the
//! `codegen_exec.rs` single-/two-file patterns to a whole app graph.
//!
//! Skipped gracefully (with an eprintln) when node is not at the pinned path, so
//! CI without that toolchain still passes.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const NODE: &str = concat!(env!("HOME"), "/.nvm/versions/node/v22.18.0/bin/node");

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn node_available() -> bool {
    std::path::Path::new(NODE).exists()
}

fn compile_fixture(rel: &str) -> String {
    let src = std::fs::read_to_string(fixtures_dir().join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"));
    let (js, diags) = lunas_compiler::compile(&src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "compile `{rel}` diagnostics: {diags:?}"
    );
    js.unwrap_or_else(|| panic!("`{rel}` emitted no module"))
        .replace("from \"lunas\";", "from \"./src/index.mjs\";")
}

/// Compiles the whole app graph and runs `driver_body` against it.
///
/// Every module's `@use`/`import … from "./Foo.lunas"` is rewritten to a
/// generated sibling file so node resolves the graph with no package install;
/// the runtime `from "lunas"` import is rewritten to the runtime index.
fn run_app(name: &str, driver_body: &str) -> String {
    let root = repo_root();
    let test_dir = root.join("packages/lunas");

    // (fixture path, generated basename, the `@use` path other modules import it by)
    let modules = [
        ("app/App.lunas", "App", None),
        ("app/Counter.lunas", "Counter", Some("./Counter.lunas")),
        ("app/Card.lunas", "Card", Some("./Card.lunas")),
        ("app/Badge.lunas", "Badge", Some("./Badge.lunas")),
    ];

    let mut written = Vec::new();
    for (fixture, base, _) in modules {
        let mut js = compile_fixture(fixture);
        // Rewrite every child import to the generated sibling file.
        for (_, dep_base, dep_use) in modules {
            if let Some(use_path) = dep_use {
                js = js.replace(
                    &format!("from \"{use_path}\";"),
                    &format!("from \"./__app_{name}_{dep_base}.gen.mjs\";"),
                );
            }
        }
        let path = test_dir.join(format!("__app_{name}_{base}.gen.mjs"));
        std::fs::write(&path, &js).unwrap();
        written.push(path);
    }

    let driver = format!(
        "import {{ installDom }} from \"./test/dom-shim.mjs\";\n\
         import factory from \"./__app_{name}_App.gen.mjs\";\n\
         const tick = () => new Promise((r) => setTimeout(r, 0));\n\
         installDom();\n\
         {driver_body}\n"
    );
    let drv_path = test_dir.join(format!("__app_{name}.drv.mjs"));
    std::fs::write(&drv_path, driver).unwrap();

    let out = Command::new(NODE)
        .arg(&drv_path)
        .current_dir(&test_dir)
        .output()
        .expect("spawn node");

    for p in &written {
        let _ = std::fs::remove_file(p);
    }
    let _ = std::fs::remove_file(&drv_path);

    if !out.status.success() {
        std::io::stderr().write_all(&out.stderr).unwrap();
        panic!("node exited with failure for app `{name}`");
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A realistic journey through the whole app graph. The driver walks the DOM by
/// role/tag helpers (robust to structural churn) and logs a labelled line per
/// step; the Rust side asserts each.
#[test]
fn app_journey_mount_events_navigate_list_two_way() {
    if !node_available() {
        eprintln!("skipping codegen_app: node not found at {NODE}");
        return;
    }

    let driver = r#"
        // A #portal target for any teleport (App uses none, but harmless).
        const portal = document.createElement('div');
        portal.setAttribute('id', 'portal');
        document.body.appendChild(portal);

        const root = factory({});
        document.body.appendChild(root);

        // --- DOM navigation helpers (depth-first) ---
        const all = (pred, from = root) => {
          const out = [];
          const walk = (n) => {
            for (const ch of n.childNodes) {
              if (ch.kind === 'element' && pred(ch)) out.push(ch);
              walk(ch);
            }
          };
          walk(from);
          return out;
        };
        const byTag = (t, from) => all((e) => e.tag === t, from);
        const byClass = (cls, from) =>
          all((e) => (e.getAttribute('class') || '').split(/\s+/).includes(cls), from);
        const text = (n) => n.innerHTMLString();
        const btnByLabel = (lbl) =>
          byTag('button').find((b) => text(b).trim() === lbl);

        // --- initial render assertions ---
        console.log('H1:' + text(byTag('h1')[0]));

        // Card default slot: parent content + a Counter child rendered inside it.
        const card = byClass('card')[0];
        console.log('CARD_CLASS:[' + card.getAttribute('class') + ']');
        console.log('CARD_HEAD:' + text(byTag('header', card)[0]).trim());
        console.log('CARD_FOOT:' + text(byTag('footer', card)[0]).trim());

        // Counter (child inside the Card default slot).
        const counterValue = () => text(byClass('value')[0]);
        console.log('COUNTER_INIT:' + counterValue());

        // The :if page cascade (starts on 'one').
        const pageText = () => {
          const ps = byTag('p').map(text);
          return ps.find((t) => t.includes('Page') || t.includes('No page')) || '';
        };
        console.log('PAGE_INIT:' + pageText());

        // Badge list from the keyed :for + the <component :is="Badge">.
        const badges = () => byClass('badge').map((b) => text(b).trim());
        console.log('BADGES_INIT:' + badges().join(','));

        // --- interactions ---
        // 1) Counter +: local child state updates, parent untouched.
        const plus = byTag('button', byClass('counter')[0]).find((b) => text(b).trim() === '+');
        plus.dispatch('click'); await tick();
        plus.dispatch('click'); await tick();
        console.log('COUNTER_AFTER_PLUS:' + counterValue());

        // 2) Navigate the page cascade: one -> two -> (n>... falls to else via unknown).
        btnByLabel('two').dispatch('click'); await tick();
        console.log('PAGE_TWO:' + pageText());
        btnByLabel('one').dispatch('click'); await tick();
        console.log('PAGE_ONE:' + pageText());

        // 3) Grow the keyed :for list (array push -> new Badge child mounts).
        btnByLabel('add tag').dispatch('click'); await tick();
        console.log('BADGES_AFTER_ADD:' + badges().join(','));

        // 4) Two-way edit the Counter's note input; the mirrored <p> updates.
        const noteInput = byTag('input', byClass('counter')[0])[0];
        noteInput.value = 'edited';
        noteInput.dispatch('input'); await tick();
        console.log('NOTE_AFTER_EDIT:' + text(byClass('note')[0]).trim());
    "#;

    let out = run_app("journey", driver);
    let line = |prefix: &str| {
        out.lines()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("missing `{prefix}` in output:\n{out}"))
            .strip_prefix(prefix)
            .unwrap()
            .to_string()
    };

    // Initial render.
    assert_eq!(line("H1:"), "Lunas E2E App", "app title renders");
    assert!(
        line("CARD_CLASS:").contains("lead"),
        "Card `tone` prop reached its `:class` interpolation"
    );
    assert_eq!(
        line("CARD_HEAD:"),
        "Dashboard for ada",
        "named #head slot routed + read parent `user` state"
    );
    assert_eq!(
        line("CARD_FOOT:"),
        "rows: 3",
        "scoped #foot slot received the child's `:count` (rows=3)"
    );
    assert_eq!(
        line("COUNTER_INIT:"),
        "10",
        "Counter seeded from the `:start` prop (seed=10)"
    );
    assert!(
        line("PAGE_INIT:").contains("Page one"),
        "initial :if branch is page one"
    );
    assert_eq!(
        line("BADGES_INIT:"),
        "[alpha],[beta],[ada]",
        "two :for badges + the <component :is> badge (user=ada)"
    );

    // Interactions.
    assert_eq!(
        line("COUNTER_AFTER_PLUS:"),
        "12",
        "child-local +1 twice updates only the child value"
    );
    assert!(
        line("PAGE_TWO:").contains("Page two"),
        ":elseif branch selected after navigating to two"
    );
    assert!(
        line("PAGE_ONE:").contains("Page one"),
        ":if branch reselected after navigating back to one"
    );
    assert_eq!(
        line("BADGES_AFTER_ADD:"),
        "[alpha],[beta],[extra],[ada]",
        "array push mounts a new keyed Badge; :is badge stays last"
    );
    assert_eq!(
        line("NOTE_AFTER_EDIT:"),
        "note: edited",
        "two-way input write-back updated the mirrored text in the child"
    );
}
