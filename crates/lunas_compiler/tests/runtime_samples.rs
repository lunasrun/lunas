//! Sample-directory E2E runner — the foundation for 1000+ Lunas test cases.
//!
//! Each case lives in `tests/runtime/samples/<category>/<case>/`:
//!   - `App.lunas`         entry component (may `@use` sibling `*.lunas`)
//!   - `_config.json`      optional: props / diagnostics / skip / description
//!   - `expected.html`     normalized initial DOM after mount (regeneratable)
//!   - `steps.mjs`         optional interaction/assertion script
//!   - `expected.after.html` optional: normalized DOM after steps (regen)
//!
//! See `tests/runtime/README.md` for the authoring format and the assertion kit.
//!
//! ## Design: per-case granularity + a single batched node process
//!
//! `build.rs` generates one `#[test] fn case_<name>()` per case dir (no
//! proc-macro). Every generated test calls [`run_case`], which lazily runs the
//! WHOLE batch exactly once (guarded by a `OnceLock`): all cases are compiled in
//! Rust, their emitted modules + a manifest are written into one temp dir, and
//! `node harness/run-samples.mjs` is spawned ONCE for the entire suite. Each
//! per-case test then looks up its own result. This gives:
//!   - per-case test names + parallel `cargo test` scheduling + isolated
//!     failures (a broken case fails only its own test), and
//!   - O(1) node spawns regardless of case count — the #1 scale requirement.
//!
//! ## Regenerating expected output
//!
//! ```text
//! UPDATE_EXPECTED=1 cargo test -p lunas_compiler --test runtime_samples
//! ```
//!
//! writes each case's freshly captured `expected.html` (and, for cases with a
//! `steps.mjs`, `expected.after.html`) to disk and passes. Review the diff.
//!
//! ## Node absent
//!
//! If node is not at the pinned path the whole suite skips loudly (an eprintln),
//! exactly like the other exec suites — CI without node still passes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The pinned local node (matches the other exec suites' developer path).
const PINNED_NODE: &str = concat!(env!("HOME"), "/.nvm/versions/node/v22.18.0/bin/node");

/// Resolve the node binary to use, in priority order:
///
/// 1. `$LUNAS_NODE` if set (CI sets this to the `setup-node` binary), else
/// 2. the pinned local path if it exists, else
/// 3. bare `node` on PATH if runnable.
///
/// Returns `None` if none is available — the suite then skips loudly, so CI
/// without node still passes (never a hard failure).
fn node_bin() -> Option<String> {
    if let Some(env) = std::env::var_os("LUNAS_NODE") {
        let p = PathBuf::from(&env);
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    if Path::new(PINNED_NODE).exists() {
        return Some(PINNED_NODE.to_string());
    }
    // PATH fallback: `node --version` must succeed.
    if Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("node".to_string());
    }
    None
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime/samples")
}

fn harness_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime/harness")
}

fn update_mode() -> bool {
    std::env::var_os("UPDATE_EXPECTED").is_some()
}

/// Per-case config from `_config.json` (all fields optional).
#[derive(Default)]
struct CaseConfig {
    props: Option<serde_json::Value>,
    /// "none" (default) or "expected": whether non-error diagnostics are allowed.
    diagnostics: String,
    skip: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
}

fn load_config(dir: &Path) -> CaseConfig {
    let path = dir.join("_config.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return CaseConfig::default();
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            // A malformed config fails only this case, never the batch.
            return CaseConfig {
                skip: Some(format!("invalid _config.json: {e}")),
                ..Default::default()
            };
        }
    };
    CaseConfig {
        props: v.get("props").cloned(),
        diagnostics: v
            .get("diagnostics")
            .and_then(|d| d.as_str())
            .unwrap_or("none")
            .to_string(),
        skip: v
            .get("skip")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        description: v
            .get("description")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
    }
}

/// The outcome the batch computed for one case.
#[derive(Clone)]
enum Outcome {
    /// Ran under node with this status/message from the driver.
    Ran {
        status: String,
        message: String,
        initial_html: Option<String>,
        after_html: Option<String>,
    },
    /// Never reached node — compile error, bad config, or an explicit skip.
    Skipped {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

/// The whole batch result: case name -> outcome. Built once.
struct Batch {
    node_missing: bool,
    outcomes: BTreeMap<String, Outcome>,
}

static BATCH: OnceLock<Batch> = OnceLock::new();

/// Every case dir under `samples/`, relative path with `/` separators.
fn discover_cases() -> Vec<String> {
    let root = samples_dir();
    let mut out = Vec::new();
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut subdirs = Vec::new();
        let mut has_app = false;
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                subdirs.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some("App.lunas") {
                has_app = true;
            }
        }
        if has_app {
            if let Ok(rel) = dir.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        for s in subdirs {
            walk(root, &s, out);
        }
    }
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// Compile `App.lunas` plus every sibling `*.lunas` in a case dir into a temp
/// dir, rewriting the `lunas` runtime import and `@use` sibling imports so node
/// resolves everything with no package install. Returns the manifest entry, or
/// an `Err(reason)` if the case should be skipped/failed before node.
fn prepare_case(
    name: &str,
    case_dir: &Path,
    tmp_dir: &Path,
    runtime_dir: &Path,
    cfg: &CaseConfig,
) -> Result<serde_json::Value, Outcome> {
    // The per-case work dir under tmp (flatten separators to a single dir).
    let work_name = name.replace('/', "__");
    let work = tmp_dir.join(&work_name);
    std::fs::create_dir_all(&work).map_err(|e| Outcome::Failed {
        reason: format!("mkdir work dir: {e}"),
    })?;

    // Compile every *.lunas in the case dir. `App.lunas` -> `App.gen.mjs`, and
    // each sibling `Foo.lunas` -> `Foo.gen.mjs`; the `@use "./Foo.lunas"` import
    // is rewritten to `./Foo.gen.mjs`.
    let mut lunas_files: Vec<PathBuf> = std::fs::read_dir(case_dir)
        .map_err(|e| Outcome::Failed {
            reason: format!("read case dir: {e}"),
        })?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("lunas"))
        .collect();
    lunas_files.sort();

    // Import the runtime by an absolute `file://` URL so node resolves it
    // regardless of the temp dir's location (and macOS's /var -> /private/var
    // symlink, which breaks naive relative-path arithmetic). Canonicalize so the
    // URL points at the real file.
    let runtime_index = runtime_dir.join("index.mjs");
    let runtime_url = file_url(&runtime_index);

    for src_path in &lunas_files {
        let src = std::fs::read_to_string(src_path).map_err(|e| Outcome::Failed {
            reason: format!("read {}: {e}", src_path.display()),
        })?;
        let (js, diags) = lunas_compiler::compile(&src);

        let errors: Vec<String> = diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.message.clone())
            .collect();
        if !errors.is_empty() {
            return Err(Outcome::Failed {
                reason: format!(
                    "compile `{}` produced errors: {errors:?}",
                    src_path.display()
                ),
            });
        }
        let warnings: Vec<String> = diags
            .iter()
            .filter(|d| !d.is_error())
            .map(|d| d.message.clone())
            .collect();
        // diagnostics: "none" (default) forbids warnings; "expected" allows them.
        if cfg.diagnostics != "expected" && !warnings.is_empty() {
            return Err(Outcome::Failed {
                reason: format!(
                    "compile `{}` produced unexpected warnings (set \"diagnostics\": \"expected\" to allow): {warnings:?}",
                    src_path.display()
                ),
            });
        }

        let Some(mut js) = js else {
            return Err(Outcome::Failed {
                reason: format!("`{}` emitted no module", src_path.display()),
            });
        };
        // Rewrite the runtime import to an absolute file URL node can resolve.
        js = js.replace("from \"lunas\";", &format!("from \"{runtime_url}\";"));
        // Rewrite sibling `@use "./Foo.lunas"` imports to the generated file.
        for sib in &lunas_files {
            let stem = sib.file_stem().and_then(|s| s.to_str()).unwrap();
            js = js.replace(
                &format!("from \"./{stem}.lunas\";"),
                &format!("from \"./{stem}.gen.mjs\";"),
            );
        }
        let stem = src_path.file_stem().and_then(|s| s.to_str()).unwrap();
        let out_path = work.join(format!("{stem}.gen.mjs"));
        std::fs::write(&out_path, &js).map_err(|e| Outcome::Failed {
            reason: format!("write {}: {e}", out_path.display()),
        })?;
    }

    // Load stored expected files (may be absent -> null, checked in the driver).
    let expected_html = std::fs::read_to_string(case_dir.join("expected.html"))
        .ok()
        .map(|s| s.trim_end_matches('\n').to_string());
    let expected_after = std::fs::read_to_string(case_dir.join("expected.after.html"))
        .ok()
        .map(|s| s.trim_end_matches('\n').to_string());
    let has_steps = case_dir.join("steps.mjs").exists();

    // Copy steps.mjs into the work dir so its relative import of the harness kit
    // resolves. It imports the kit by a fixed relative path we control below.
    if has_steps {
        let steps_src =
            std::fs::read_to_string(case_dir.join("steps.mjs")).map_err(|e| Outcome::Failed {
                reason: format!("read steps.mjs: {e}"),
            })?;
        std::fs::write(work.join("steps.mjs"), steps_src).map_err(|e| Outcome::Failed {
            reason: format!("write steps.mjs: {e}"),
        })?;
    }

    Ok(serde_json::json!({
        "name": name,
        "dir": work.to_string_lossy(),
        "entry": "App.gen.mjs",
        "props": cfg.props.clone().unwrap_or(serde_json::Value::Null),
        "hasSteps": has_steps,
        "expectedHtml": expected_html,
        "expectedAfterHtml": expected_after,
    }))
}

/// An absolute `file://` URL for a path (canonicalized so macOS symlinks like
/// `/var` -> `/private/var` do not confuse node's resolver). Used as an ES
/// import specifier for the runtime index.
fn file_url(path: &Path) -> String {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = canon.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// Build the batch: compile everything, spawn node once, parse per-case results.
fn build_batch() -> Batch {
    let Some(node) = node_bin() else {
        eprintln!(
            "skipping runtime_samples: no node found (set LUNAS_NODE, or install \
             {PINNED_NODE}, or put `node` on PATH)"
        );
        return Batch {
            node_missing: true,
            outcomes: BTreeMap::new(),
        };
    };

    let root = repo_root();
    // Canonicalize so the URLs the driver builds match, byte-for-byte, the ones
    // the compiled `.gen.mjs` modules import — node then loads a SINGLE runtime
    // module instance shared across the whole batch (shared box identity + one
    // global `document`).
    let runtime_dir = std::fs::canonicalize(root.join("packages/lunas/src"))
        .unwrap_or_else(|_| root.join("packages/lunas/src"));
    let shim_path = std::fs::canonicalize(root.join("packages/lunas/test/dom-shim.mjs"))
        .unwrap_or_else(|_| root.join("packages/lunas/test/dom-shim.mjs"));

    // One temp dir for the whole batch.
    let tmp_dir = std::env::temp_dir().join(format!("lunas_samples_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    // The kit imports normalize.mjs; the driver imports both. We point the
    // manifest at the real harness dir so those modules load from source, and
    // copy nothing — steps.mjs in each work dir imports the kit by a relative
    // path back to the harness dir, which we inject here.
    let harness = harness_dir();

    let mut outcomes: BTreeMap<String, Outcome> = BTreeMap::new();
    let mut manifest_cases = Vec::new();

    for name in discover_cases() {
        let case_dir = samples_dir().join(&name);
        let cfg = load_config(&case_dir);
        if let Some(reason) = &cfg.skip {
            outcomes.insert(
                name.clone(),
                Outcome::Skipped {
                    reason: reason.clone(),
                },
            );
            continue;
        }
        match prepare_case(&name, &case_dir, &tmp_dir, &runtime_dir, &cfg) {
            Ok(entry) => manifest_cases.push(entry),
            Err(o) => {
                outcomes.insert(name.clone(), o);
            }
        }
    }

    // steps.mjs files import the kit via `../_harness/kit.mjs`. Provide a stable
    // symlink-free path: copy the harness dir next to the work dirs as
    // `_harness`, and rewrite each steps.mjs import base. Simpler: point steps to
    // the real harness dir by absolute path at copy time. We already copied
    // steps.mjs verbatim; to keep authoring ergonomic, steps.mjs imports the kit
    // with a bare specifier `@kit` that we resolve via an import map is overkill
    // for node — instead, the driver injects the kit into a global before
    // importing steps. See run-samples.mjs: it passes the kit object into the
    // step function, so steps.mjs need NOT import anything. Nothing to rewrite.
    let _ = &harness;

    let manifest = serde_json::json!({
        "shimPath": shim_path.to_string_lossy(),
        "runtimeDir": runtime_dir.to_string_lossy(),
        "update": update_mode(),
        "cases": manifest_cases,
    });
    let manifest_path = tmp_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let driver = harness.join("run-samples.mjs");
    let out = Command::new(&node)
        .arg(&driver)
        .arg(&manifest_path)
        .output()
        .expect("spawn node");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if std::env::var_os("LUNAS_DEBUG").is_some() {
        eprintln!("=== node stdout ===\n{stdout}\n=== node stderr ===\n{stderr}");
    }

    // Extract the sentinel-wrapped JSON.
    let parsed = extract_results(&stdout);
    match parsed {
        Some(results) => {
            for r in results {
                let name = r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let status = r
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("error")
                    .to_string();
                let message = r
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let initial_html = r
                    .get("initialHtml")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let after_html = r
                    .get("afterHtml")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                outcomes.insert(
                    name,
                    Outcome::Ran {
                        status,
                        message,
                        initial_html,
                        after_html,
                    },
                );
            }
        }
        None => {
            // The node process itself blew up before emitting results. Mark
            // every case that reached node as failed with the node output, so
            // the failure is visible per-case (not a silent pass).
            let reason = format!(
                "node driver produced no results block.\nstdout:\n{stdout}\nstderr:\n{stderr}"
            );
            for entry in &manifest["cases"].as_array().cloned().unwrap_or_default() {
                if let Some(n) = entry.get("name").and_then(|v| v.as_str()) {
                    outcomes
                        .entry(n.to_string())
                        .or_insert_with(|| Outcome::Failed {
                            reason: reason.clone(),
                        });
                }
            }
        }
    }

    // In update mode, write regenerated expected files back to the source tree.
    if update_mode() {
        for (name, outcome) in &outcomes {
            if let Outcome::Ran {
                initial_html,
                after_html,
                ..
            } = outcome
            {
                let case_dir = samples_dir().join(name);
                if let Some(html) = initial_html {
                    let _ = std::fs::write(case_dir.join("expected.html"), format!("{html}\n"));
                }
                if let Some(html) = after_html {
                    let _ =
                        std::fs::write(case_dir.join("expected.after.html"), format!("{html}\n"));
                }
            }
        }
    }

    // Best-effort cleanup of the temp dir.
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Batch {
        node_missing: false,
        outcomes,
    }
}

/// Pull the JSON object between the sentinel lines out of the driver's stdout.
fn extract_results(stdout: &str) -> Option<Vec<serde_json::Value>> {
    let begin = stdout.find("__LUNAS_RESULTS_BEGIN__")?;
    let after = &stdout[begin + "__LUNAS_RESULTS_BEGIN__".len()..];
    let end = after.find("__LUNAS_RESULTS_END__")?;
    let json = after[..end].trim();
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("results")
        .and_then(|r| r.as_array())
        .map(|a| a.to_vec())
}

fn batch() -> &'static Batch {
    BATCH.get_or_init(build_batch)
}

/// The entry every generated `#[test]` calls.
fn run_case(name: &str) {
    let b = batch();
    if b.node_missing {
        eprintln!("skipping `{name}`: node not available");
        return; // skip loudly, not a hard failure
    }
    match b.outcomes.get(name) {
        Some(Outcome::Ran {
            status, message, ..
        }) => {
            match status.as_str() {
                // In update mode a "fail" is only an expected-HTML mismatch, which
                // we just regenerated — treat it as pass. An "error" (a thrown
                // exception, a bad module, a steps.mjs assertion) is a real bug
                // and must surface even under UPDATE_EXPECTED.
                "pass" => {}
                "fail" if update_mode() => {}
                "fail" => panic!("case `{name}` FAILED:\n{message}"),
                _ => panic!("case `{name}` ERRORED:\n{message}"),
            }
        }
        Some(Outcome::Skipped { reason }) => {
            eprintln!("skipping `{name}`: {reason}");
        }
        Some(Outcome::Failed { reason }) => {
            panic!("case `{name}` failed before node:\n{reason}");
        }
        None => panic!(
            "case `{name}` has no result — it was not discovered or the batch \
             lost it (this is a harness bug, not a case failure)"
        ),
    }
}

// The generated per-case `#[test] fn`s (one call to `run_case` each).
include!(concat!(env!("OUT_DIR"), "/runtime_samples_generated.rs"));

// A guard test so the file compiles even if zero cases exist, and so the batch
// path is exercised (and node-skip is visible) in an otherwise empty tree.
#[test]
fn harness_batch_is_reachable() {
    let _ = batch();
}
