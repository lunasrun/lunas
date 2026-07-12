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

use lunas_parser::{parse, Directive, TemplateNode};
use lunas_script::{declared_bindings_with_spans, free_identifiers_with_spans, parse_for};
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

/// One named symbol occurrence flattened for JavaScript: a `name`, its
/// file-absolute UTF-8 byte range, and a `kind` categorizing it for semantic
/// highlighting — `"variable"` (script bindings, `:for` variables, plain
/// identifier uses), `"prop"` (`@input`), or `"component"` (`@use` and
/// `<Component/>` usages).
#[derive(Serialize)]
struct JsSymbol {
    name: String,
    start: u32,
    end: u32,
    kind: &'static str,
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

/// Byte span of the first occurrence of `name` within `range`, as `(start, end)`
/// file-absolute UTF-8 offsets. Used to pin a directive/component *name* inside
/// a larger span (e.g. the `name` in an `@input name: T` body, whose stored
/// range covers the whole body). Falls back to the full range if not found.
fn ident_span(source: &str, range: lunas_span::TextRange, name: &str) -> (u32, u32) {
    let base = range.start().raw();
    let text = range.slice(source).unwrap_or("");
    match text.find(name) {
        Some(pos) => (base + pos as u32, base + pos as u32 + name.len() as u32),
        None => (range.start().raw(), range.end().raw()),
    }
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
                    kind: "variable",
                });
            }
        }
    }

    // Top-level directive declarations: `@input` props and `@use` components are
    // declarations too (referenced from the template), so navigation can resolve
    // them. Their stored `range` is the directive body; pin the name within it.
    for directive in &file.directives {
        let (name, range, kind) = match directive {
            Directive::Input(prop) => (prop.name.clone(), prop.range, "prop"),
            Directive::UseComponent(uc) => (uc.component_name.clone(), uc.range, "component"),
            _ => continue,
        };
        let (start, end) = ident_span(source, range, &name);
        bindings.push(JsSymbol {
            name,
            start,
            end,
            kind,
        });
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
                        kind: "variable",
                    });
                }
            }
        });

        html.template.visit(&mut |node| match node {
            // Component usages (`<Foo/>`) are references to their `@use Foo`
            // binding. The tag name sits at the start of the open-tag span.
            TemplateNode::Component(component) => {
                let (start, end) = ident_span(source, component.open_tag_range, &component.name);
                references.push(JsSymbol {
                    name: component.name.clone(),
                    start,
                    end,
                    kind: "component",
                });
            }
            // A capitalized tag that isn't registered via `@use` parses as a
            // plain element (only `@use`-known tags become `Component`s). By the
            // component-naming convention it's a component use, so surface it as
            // a component reference too — it resolves to no `@use` binding, which
            // lets a language server flag it as an unknown component. The HTML
            // parser lowercases element names, so read the raw name (and its
            // original case) from the source tag.
            TemplateNode::Element(element) => {
                let raw = element.open_tag_range.slice(source).unwrap_or("");
                let raw_name: String = raw
                    .trim_start_matches('<')
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
                    .collect();
                if raw_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    let start = element.open_tag_range.start().raw() + 1; // past `<`
                    let end = start + raw_name.len() as u32;
                    references.push(JsSymbol {
                        name: raw_name,
                        start,
                        end,
                        kind: "component",
                    });
                }
            }
            // `:for="item of items"` — the loop variable is a declaration
            // (referenced from the loop body, which the expression walk already
            // covers), and the iterable is a reference to an outer binding. The
            // header isn't a plain expression, so split it with `parse_for`.
            TemplateNode::For(for_block) => {
                if let Some(parsed) = parse_for(&for_block.header.text) {
                    let header_text = &for_block.header.text;
                    let base = for_block.header.range.start().raw();
                    // Loop variable declaration(s). Reuse the declaration-pattern
                    // collector so destructuring (`[i, v]`, `{ a }`) yields each
                    // bound name: wrap the binding as a `let`, then shift the
                    // resulting spans back to the pattern's position in the header.
                    if let Some(pat_pos) = header_text.find(&parsed.binding) {
                        let pat_base = base + pat_pos as u32;
                        let wrapped = format!("let {} = 0;", parsed.binding);
                        if let Ok(decls) = declared_bindings_with_spans(&wrapped) {
                            let prefix = "let ".len() as u32;
                            for (name, local) in decls {
                                let start = pat_base + local.start().raw().saturating_sub(prefix);
                                let len = local.end().raw() - local.start().raw();
                                bindings.push(JsSymbol {
                                    name,
                                    start,
                                    end: start + len,
                                    kind: "variable",
                                });
                            }
                        }
                    }
                    // Iterable references (its free identifiers), shifted to the
                    // iterable's position within the header.
                    if let Some(pos) = header_text.rfind(&parsed.iterable) {
                        let iter_base: lunas_span::TextSize = (base + pos as u32).into();
                        if let Ok(refs) = free_identifiers_with_spans(&parsed.iterable) {
                            for (name, local) in refs {
                                let r = local.shifted(iter_base);
                                references.push(JsSymbol {
                                    name,
                                    start: r.start().raw(),
                                    end: r.end().raw(),
                                    kind: "variable",
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
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
    fn analyze_reports_input_prop_declarations() {
        let src = "@input name: string\nhtml:\n    <p>${ name }</p>\n";
        let result = analyze_source(src);

        // `@input name` is a declaration, pinned to the `name` token.
        let decl = result
            .bindings
            .iter()
            .find(|b| b.name == "name")
            .expect("@input binding");
        assert_eq!(&src[decl.start as usize..decl.end as usize], "name");
        assert_eq!(decl.kind, "prop");

        // And it's referenced in the template.
        let r = result.references.iter().find(|r| r.name == "name");
        assert!(r.is_some(), "template reference to the prop");
    }

    #[test]
    fn analyze_reports_use_component_decl_and_tag_reference() {
        let src = "@use Foo from \"./Foo.lunas\"\nhtml:\n    <Foo/>\n";
        let result = analyze_source(src);

        let decl = result
            .bindings
            .iter()
            .find(|b| b.name == "Foo")
            .expect("@use binding");
        assert_eq!(&src[decl.start as usize..decl.end as usize], "Foo");
        assert_eq!(decl.kind, "component");

        // The `<Foo/>` usage is a reference to the `@use` binding, pinned to the
        // tag name.
        let tag = result
            .references
            .iter()
            .find(|r| r.name == "Foo")
            .expect("component-tag reference");
        assert_eq!(&src[tag.start as usize..tag.end as usize], "Foo");
    }

    #[test]
    fn analyze_reports_unregistered_capitalized_tag_as_component_ref() {
        // `<Bar/>` with no `@use Bar` — a component reference with no matching
        // binding, so a language server can flag it as an unknown component.
        let src = "html:\n    <Bar/>\n";
        let result = analyze_source(src);

        let tag = result
            .references
            .iter()
            .find(|r| r.name == "Bar")
            .expect("capitalized tag as component reference");
        assert_eq!(tag.kind, "component");
        assert_eq!(&src[tag.start as usize..tag.end as usize], "Bar");
        // ...and it resolves to no binding.
        assert!(!result.bindings.iter().any(|b| b.name == "Bar"));
    }

    #[test]
    fn analyze_leaves_lowercase_elements_alone() {
        // Ordinary elements are not component references.
        let result = analyze_source("html:\n    <div><p>hi</p></div>\n");
        assert!(result.references.iter().all(|r| r.kind != "component"));
    }

    #[test]
    fn analyze_reports_for_loop_variable_and_iterable() {
        let src =
            "html:\n    <li :for=\"item of items\">${ item }</li>\nscript:\n    let items = []\n";
        let result = analyze_source(src);

        // The loop variable is a declaration, pinned to `item` in the header.
        let decl = result
            .bindings
            .iter()
            .find(|b| b.name == "item")
            .expect("loop variable binding");
        assert_eq!(&src[decl.start as usize..decl.end as usize], "item");

        // The iterable is a reference (to the `items` script binding).
        let iter = result
            .references
            .iter()
            .find(|r| r.name == "items")
            .expect("iterable reference");
        assert_eq!(&src[iter.start as usize..iter.end as usize], "items");

        // The `${ item }` body use is a reference to the loop variable.
        assert!(
            result.references.iter().any(|r| r.name == "item"),
            "loop-body reference to the loop variable",
        );
    }

    #[test]
    fn analyze_reports_destructured_for_loop_variables() {
        let src = "html:\n    <li :for=\"[i, v] of pairs\">${ i }${ v }</li>\n";
        let result = analyze_source(src);

        // Both destructured names become bindings, each pinned to its own token.
        for name in ["i", "v"] {
            let decl = result
                .bindings
                .iter()
                .find(|b| b.name == name)
                .unwrap_or_else(|| panic!("binding for `{name}`"));
            assert_eq!(&src[decl.start as usize..decl.end as usize], name);
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
            "@input\n@use\nhtml:\n    <Bad",
            "html:\n    <li :for=\"= = =\">x</li>",
        ] {
            let _ = analyze_source(src);
        }
    }
}
