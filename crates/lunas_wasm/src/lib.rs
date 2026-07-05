//! Thin [`wasm-bindgen`] bindings over the Lunas compiler.
//!
//! This crate contains **no compiler logic** — it only adapts
//! [`lunas_compiler::compile`] into a shape JavaScript can consume, and
//! serializes the resulting diagnostics into plain JS objects. Build it with
//! `wasm-pack` (see the crate README) and load the generated package from a
//! bundler plugin or the browser.
//!
//! The single entry point is [`compile`], which returns a JS value shaped like:
//!
//! ```js
//! {
//!   code: string | null,          // the emitted ES module, or null on failure
//!   diagnostics: Array<{
//!     message: string,
//!     severity: "error" | "warning" | "hint",
//!     start: number,              // byte offset (inclusive)
//!     end: number,                // byte offset (exclusive)
//!   }>
//! }
//! ```
//!
//! Byte offsets index the original source string; a JS consumer maps them to
//! line/column with its own line index (the offsets are UTF-8 byte offsets,
//! matching the compiler's spans).

use lunas_parser::parse;
use lunas_script::{declared_bindings_with_spans, free_identifiers_with_spans};
use lunas_span::{Diagnostic, Severity};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// One diagnostic, flattened for JavaScript: `range` is unwrapped to
/// `start`/`end` byte offsets and `severity` is a lowercase string.
#[derive(Serialize)]
struct JsDiagnostic {
    message: String,
    severity: &'static str,
    start: u32,
    end: u32,
}

impl From<&Diagnostic> for JsDiagnostic {
    fn from(d: &Diagnostic) -> Self {
        JsDiagnostic {
            message: d.message.clone(),
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Hint => "hint",
            },
            start: d.range.start().raw(),
            end: d.range.end().raw(),
        }
    }
}

/// The result of a [`compile`] call, serialized to a plain JS object.
#[derive(Serialize)]
struct CompileResult {
    code: Option<String>,
    diagnostics: Vec<JsDiagnostic>,
}

/// Compiles a `.lunas`/`.lun` source string into an ES module.
///
/// Never throws: parse/resolve problems are returned as `diagnostics`, and a
/// missing/unemittable module yields `code: null`. Mirrors
/// [`lunas_compiler::compile`] exactly — this is only the serialization shim.
///
/// Returns a JS object `{ code, diagnostics }` (see the crate-level docs for
/// the exact shape).
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
    let (code, diags) = lunas_compiler::compile(source);
    let result = CompileResult {
        code,
        diagnostics: diags.iter().map(JsDiagnostic::from).collect(),
    };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// One named symbol occurrence flattened for JavaScript: a `name` plus its
/// file-absolute UTF-8 byte range.
#[derive(Serialize)]
struct JsSymbol {
    name: String,
    start: u32,
    end: u32,
}

/// The result of an [`analyze`] call: the navigation data a language server
/// needs for a `.lunas` source.
///
/// - `bindings` — each binding declared in the `script:` block, at its
///   declaration site.
/// - `references` — each identifier used in a template expression (`${ … }`,
///   `:if`/`:for` headers, event handlers, attribute bindings). Template uses
///   that shadow a local of the same name are excluded (free identifiers only),
///   so a reference matched to a binding by name is a genuine use of it.
///
/// A JS consumer wires these into go-to-definition, find-references, highlight,
/// rename and semantic tokens by matching `references` to `bindings` by name.
#[derive(Serialize)]
struct AnalyzeResult {
    bindings: Vec<JsSymbol>,
    references: Vec<JsSymbol>,
}

/// Pure analysis over a `.lunas` source, split out so it is unit-testable on the
/// host target without the wasm-bindgen `JsValue` bridge.
///
/// Never panics: parse problems yield an empty file, and per-block script parse
/// errors are skipped rather than propagated (mirrors the never-panic contract
/// of the public compiler entry points).
fn analyze_source(source: &str) -> AnalyzeResult {
    let (file, _diags) = parse(source);

    let mut bindings = Vec::new();
    if let Some(script) = &file.script {
        if let Ok(decls) = declared_bindings_with_spans(&script.source.text) {
            let base = script.source.range.start();
            for (name, local) in decls {
                let r = local.shifted(base);
                bindings.push(JsSymbol {
                    name,
                    start: r.start().raw(),
                    end: r.end().raw(),
                });
            }
        }
    }

    let mut references = Vec::new();
    if let Some(html) = &file.html {
        html.template.for_each_expression(|text, expr_range| {
            if let Ok(refs) = free_identifiers_with_spans(text) {
                let base = expr_range.start();
                for (name, local) in refs {
                    let r = local.shifted(base);
                    references.push(JsSymbol {
                        name,
                        start: r.start().raw(),
                        end: r.end().raw(),
                    });
                }
            }
        });
    }

    AnalyzeResult {
        bindings,
        references,
    }
}

/// Analyzes a `.lunas` source for language-server navigation.
///
/// Never throws: returns `{ bindings, references }` (see [`AnalyzeResult`]) with
/// file-absolute UTF-8 byte offsets, which a JS consumer maps to line/column
/// with its own line index. This is only the serialization shim over
/// [`analyze_source`]; all analysis lives in `lunas_parser` / `lunas_script`.
#[wasm_bindgen]
pub fn analyze(source: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&analyze_source(source))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// The compiler package version (the `lunas_wasm` crate version). Useful for a
/// plugin to log which compiler build is loaded.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These run on the host target (`cargo test --workspace`). They exercise the
    // pure-Rust adapter logic without wasm-bindgen's JsValue bridge, so the
    // workspace test job stays green without a wasm runtime.

    #[test]
    fn diagnostic_flattening_lowercases_and_unwraps_range() {
        let d = Diagnostic::warning(lunas_span::TextRange::at(3, 8), "careful");
        let js = JsDiagnostic::from(&d);
        assert_eq!(js.severity, "warning");
        assert_eq!(js.start, 3);
        assert_eq!(js.end, 8);
        assert_eq!(js.message, "careful");
    }

    #[test]
    fn compile_a_valid_component_yields_code_and_no_errors() {
        let (code, diags) = lunas_compiler::compile(
            "html:\n    <button @click=\"inc()\">${count}</button>\nscript:\n    let count = 0\n    function inc(){ count++ }\n",
        );
        assert!(code.is_some());
        assert!(!diags.iter().any(|d| d.is_error()));
    }

    #[test]
    fn compile_reports_errors_without_panicking() {
        // A malformed script block surfaces a diagnostic, not a panic.
        let (_code, diags) =
            lunas_compiler::compile("html:\n    <p>${x}</p>\nscript:\n    let = = =\n");
        // Whatever the outcome, the adapter must serialize cleanly.
        let dtos: Vec<JsDiagnostic> = diags.iter().map(JsDiagnostic::from).collect();
        assert_eq!(dtos.len(), diags.len());
    }

    #[test]
    fn analyze_finds_script_bindings_and_template_references() {
        let src = "\
html:
    <div :class=\"count > 0 ? 'on' : 'off'\">${ count }</div>
script:
    let count = 0
";
        let result = analyze_source(src);

        // The `let count` binding is reported at its declaration site.
        let count_bind = result
            .bindings
            .iter()
            .find(|b| b.name == "count")
            .expect("count binding should be found");
        assert_eq!(
            &src[count_bind.start as usize..count_bind.end as usize],
            "count"
        );

        // `count` is referenced in the template (the `:class` expression and the
        // `${ count }` interpolation), each span slicing back to the name.
        let refs: Vec<_> = result
            .references
            .iter()
            .filter(|r| r.name == "count")
            .collect();
        assert!(
            refs.len() >= 2,
            "expected >= 2 template references, got {}",
            refs.len()
        );
        for r in refs {
            assert_eq!(&src[r.start as usize..r.end as usize], "count");
        }
    }

    #[test]
    fn analyze_never_panics_on_garbage() {
        // Malformed / empty inputs must analyze cleanly with no panic.
        for src in [
            "",
            "not a lunas file",
            "script:\n    let = =",
            "html:\n    ${",
        ] {
            let _ = analyze_source(src);
        }
    }
}
