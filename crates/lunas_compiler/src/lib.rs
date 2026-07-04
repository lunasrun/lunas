//! The Lunas **resolution layer**: it parses a `.lunas` file (via
//! `lunas_parser`) and runs the static analyses (via `lunas_script`) needed to
//! produce a [`ResolvedComponent`] — the framework-agnostic model a code
//! generator consumes.
//!
//! This crate stops *just before* code generation: it answers "what must be
//! rendered and what reacts to what", not "what JS to emit". The generator is a
//! separate phase that takes a [`ResolvedComponent`] as input.
//!
//! ```
//! use lunas_compiler::resolve;
//!
//! let (component, diags) = resolve(
//!     "@input start:number = 0\n\
//!      html:\n\
//!     \x20   <button @click=\"inc()\">${count}</button>\n\
//!      script:\n\
//!     \x20   let count = start\n\
//!     \x20   function inc(){ count++ }\n",
//! );
//! assert!(diags.is_empty());
//! // `count` is reassigned (by `inc`), so it is reactive and numbered.
//! assert!(component.is_reactive("count"));
//! ```

use std::collections::HashSet;

use lunas_parser::{parse, Diagnostic, Directive};
use lunas_script::{analyze_script, assigned_identifiers, declared_bindings_with_spans};

pub mod codegen;
mod model;
mod reactivity;

pub use model::{Deps, DynamicKind, DynamicPart, ReactiveVar, ResolvedComponent, ResolvedHandler};

/// Parses and resolves a `.lunas` source string into a [`ResolvedComponent`].
///
/// Like [`lunas_parser::parse`], this never panics; problems (including a
/// `script:` block that fails to parse) are reported in the diagnostics vector.
pub fn resolve(source: &str) -> (ResolvedComponent, Vec<Diagnostic>) {
    let (file, mut diags) = parse(source);

    let props = file
        .directives
        .iter()
        .filter_map(|d| match d {
            Directive::Input(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    let imports = file
        .directives
        .iter()
        .filter_map(|d| match d {
            Directive::UseComponent(u) => Some(u.clone()),
            _ => None,
        })
        .collect();

    let reactive_vars = match &file.script {
        Some(script) => resolve_reactive_vars(script, &mut diags),
        None => Vec::new(),
    };

    // Annotate every dynamic template expression with the reactive variables it
    // reads, and every handler with what it writes.
    let (dynamics, handlers) = match &file.html {
        Some(html) => {
            let script_text = file
                .script
                .as_ref()
                .map(|s| s.source.text.as_str())
                .unwrap_or("");
            let graph = reactivity::DependencyGraph::build(script_text, &reactive_vars);
            reactivity::collect(&html.template, &graph)
        }
        None => (Vec::new(), Vec::new()),
    };

    let component = ResolvedComponent {
        props,
        imports,
        template: file.html.map(|h| h.template),
        script: file.script,
        style: file.style,
        reactive_vars,
        dynamics,
        handlers,
    };
    (component, diags)
}

/// Determines which top-level bindings are reactive (declared *and* mutated
/// somewhere) and numbers them in declaration order.
fn resolve_reactive_vars(
    script: &lunas_parser::ScriptBlock,
    diags: &mut Vec<Diagnostic>,
) -> Vec<ReactiveVar> {
    let text = &script.source.text;
    let base = script.source.range.start();

    let analysis = match analyze_script(text) {
        Ok(a) => a,
        Err(e) => {
            diags.push(Diagnostic::error(
                script.source.range,
                format!("could not analyze script block: {e}"),
            ));
            return Vec::new();
        }
    };

    // A binding is reactive if it can change after init: it is mutated inside
    // some function, or reassigned at the top level.
    let mut mutated: HashSet<String> = analysis
        .function_mutations
        .iter()
        .flat_map(|(_, vars)| vars.iter().cloned())
        .collect();
    if let Ok(top_level) = assigned_identifiers(text) {
        mutated.extend(top_level);
    }

    let decl_spans = declared_bindings_with_spans(text).unwrap_or_default();
    let span_of = |name: &str| {
        decl_spans
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.shifted(base))
    };

    let mut seen = HashSet::new();
    let mut vars = Vec::new();
    let mut index = 0;
    for name in &analysis.bindings {
        if mutated.contains(name) && seen.insert(name.clone()) {
            vars.push(ReactiveVar {
                name: name.clone(),
                index,
                decl_range: span_of(name),
            });
            index += 1;
        }
    }
    vars
}
