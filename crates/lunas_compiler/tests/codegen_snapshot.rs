//! End-to-end **snapshot** suite: compiles a set of representative `.lunas`
//! fixtures that together exercise the whole feature matrix (text/attr/event
//! binds, two-way, `:if` cascades, keyed/nested `:for`, child components +
//! `@input` props, slots, `:class`/`:style`, `:ref`, `:html`, `<component :is>`,
//! `<teleport>`, multi-root fragments, `@use` imports) and locks the emitted JS
//! against on-disk snapshots under `tests/snapshots/`.
//!
//! The emitted module text IS the contract with the runtime, so a change should
//! surface as a reviewable snapshot diff.
//!
//! ## Regenerating snapshots
//!
//! When an intentional emitter change moves the output, regenerate every
//! snapshot in one shot:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test -p lunas_compiler --test codegen_snapshot
//! ```
//!
//! With `UPDATE_SNAPSHOTS` set the test writes the current output to disk and
//! passes; review the resulting `git diff` before committing. Without it (the
//! default, and in CI) each fixture is compiled and byte-compared to its stored
//! snapshot; a mismatch fails with a unified-ish diff and the regen hint.
//!
//! Diagnostics are asserted per fixture: none may be errors, and the set of
//! warning messages must match the fixture's declared expectation exactly.

use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn snapshots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn update_mode() -> bool {
    std::env::var_os("UPDATE_SNAPSHOTS").is_some()
}

/// Compiles `<fixtures>/<rel>.lunas`, asserts diagnostics, then snapshots the
/// emitted JS to `<snapshots>/<snapshot>.js`.
///
/// `expected_warnings` are substrings; each must match exactly one warning
/// message, and there must be no *unexpected* warnings and no errors.
fn snapshot(rel: &str, snapshot: &str, expected_warnings: &[&str]) {
    let src_path = fixtures_dir().join(format!("{rel}.lunas"));
    let source = std::fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", src_path.display()));

    let (js, diags) = lunas_compiler::compile(&source);

    // No errors, ever.
    let errors: Vec<&str> = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "fixture `{rel}` produced error diagnostics: {errors:?}"
    );

    // Warnings must match the declared expectation exactly (order-independent).
    let mut warnings: Vec<&str> = diags
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.message.as_str())
        .collect();
    let mut unmatched_expected = Vec::new();
    for want in expected_warnings {
        if let Some(pos) = warnings.iter().position(|w| w.contains(want)) {
            warnings.remove(pos);
        } else {
            unmatched_expected.push(*want);
        }
    }
    assert!(
        unmatched_expected.is_empty(),
        "fixture `{rel}`: expected warning(s) not found: {unmatched_expected:?}"
    );
    assert!(
        warnings.is_empty(),
        "fixture `{rel}`: unexpected warning diagnostics: {warnings:?}"
    );

    let js = js.unwrap_or_else(|| panic!("fixture `{rel}` emitted no module"));

    let snap_path = snapshots_dir().join(format!("{snapshot}.js"));
    if update_mode() {
        std::fs::create_dir_all(snapshots_dir()).unwrap();
        std::fs::write(&snap_path, &js)
            .unwrap_or_else(|e| panic!("write snapshot {}: {e}", snap_path.display()));
        return;
    }

    let stored = std::fs::read_to_string(&snap_path).unwrap_or_else(|_| {
        panic!(
            "snapshot missing for `{rel}` at {}.\n\
             Run `UPDATE_SNAPSHOTS=1 cargo test -p lunas_compiler --test codegen_snapshot` to create it.",
            snap_path.display()
        )
    });

    if stored != js {
        panic!(
            "snapshot mismatch for `{rel}` ({}).\n{}\n\
             Run `UPDATE_SNAPSHOTS=1 cargo test -p lunas_compiler --test codegen_snapshot` \
             to regenerate, then review the diff.",
            snap_path.display(),
            first_diff(&stored, &js)
        );
    }
}

/// A compact first-divergence report between the stored snapshot and the new
/// output — enough to see *where* the emitter changed without pulling in a diff
/// crate.
fn first_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let max = old_lines.len().max(new_lines.len());
    for i in 0..max {
        let o = old_lines.get(i).copied().unwrap_or("<eof>");
        let n = new_lines.get(i).copied().unwrap_or("<eof>");
        if o != n {
            return format!(
                "first divergence at line {}:\n  - stored: {o}\n  + actual: {n}",
                i + 1
            );
        }
    }
    "content differs only in trailing whitespace/length".to_string()
}

fn assert_fixtures_exist() {
    assert!(
        Path::new(&fixtures_dir()).is_dir(),
        "fixtures dir missing: {}",
        fixtures_dir().display()
    );
}

#[test]
fn text_attr_event() {
    assert_fixtures_exist();
    snapshot("text_attr_event", "text_attr_event", &[]);
}

#[test]
fn two_way() {
    snapshot("two_way", "two_way", &[]);
}

#[test]
fn if_cascade() {
    snapshot("if_cascade", "if_cascade", &[]);
}

#[test]
fn for_keyed_nested() {
    snapshot("for_keyed_nested", "for_keyed_nested", &[]);
}

#[test]
fn class_style() {
    snapshot("class_style", "class_style", &[]);
}

#[test]
fn ref_html() {
    snapshot("ref_html", "ref_html", &[]);
}

#[test]
fn dynamic_teleport() {
    snapshot("dynamic_teleport", "dynamic_teleport", &[]);
}

#[test]
fn multi_root() {
    snapshot("multi_root", "multi_root", &[]);
}

#[test]
fn app_badge() {
    snapshot("app/Badge", "app_Badge", &[]);
}

#[test]
fn app_counter() {
    snapshot("app/Counter", "app_Counter", &[]);
}

#[test]
fn app_card() {
    snapshot("app/Card", "app_Card", &[]);
}

#[test]
fn app_root() {
    snapshot("app/App", "app_App", &[]);
}
