//! The resolved component model: the framework-agnostic description of a
//! `.lunas` component that a code generator consumes. It contains no generated
//! code and no output-format decisions — only *what* must be rendered and
//! *what* reacts to *what*.

use lunas_parser::{PropInput, ScriptBlock, StyleBlock, Template, TextRange, UseComponent};

/// A top-level binding that can change after initialization and therefore needs
/// reactive tracking. Each is assigned a stable [`index`](ReactiveVar::index);
/// a code generator turns that into a runtime bit (`1 << index`) so a set of
/// dependencies is a bitmask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactiveVar {
    pub name: String,
    /// Stable bit position, assigned in declaration order among reactive vars.
    pub index: u32,
    /// File-absolute range of the declaration site, when known.
    pub decl_range: Option<TextRange>,
}

/// A fully resolved component, ready for code generation.
///
/// This is the boundary the project is built up to: everything a generator
/// needs is here (props, child components, the template IR, the numbered
/// reactive variables, and — added by later resolution passes — the dependency
/// masks of each dynamic part and event handler). Generating code from this is
/// the next, separate phase.
#[derive(Debug, Clone)]
pub struct ResolvedComponent {
    /// `@input` props, in source order.
    pub props: Vec<PropInput>,
    /// `@use` child component imports, in source order.
    pub imports: Vec<UseComponent>,
    /// The `style:` block (raw CSS), if any.
    pub style: Option<StyleBlock>,
    /// The `script:` block (raw JS/TS text + span), if any.
    pub script: Option<ScriptBlock>,
    /// The binding-aware template IR, if there is an `html:` block.
    pub template: Option<Template>,
    /// Reactive variables, numbered in declaration order.
    pub reactive_vars: Vec<ReactiveVar>,
}

impl ResolvedComponent {
    /// The reactive bit index of `name`, if it is a tracked reactive variable.
    pub fn reactive_index(&self, name: &str) -> Option<u32> {
        self.reactive_vars
            .iter()
            .find(|v| v.name == name)
            .map(|v| v.index)
    }

    /// Whether `name` is a tracked reactive variable.
    pub fn is_reactive(&self, name: &str) -> bool {
        self.reactive_index(name).is_some()
    }
}
