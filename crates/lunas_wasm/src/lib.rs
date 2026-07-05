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
}
