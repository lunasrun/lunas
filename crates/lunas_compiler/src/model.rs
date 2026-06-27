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

/// A set of reactive-variable dependencies, as sorted, unique
/// [`ReactiveVar::index`] values. A code generator turns this into whatever
/// runtime representation it likes (e.g. a bitmask via [`Deps::mask_u128`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deps {
    indices: Vec<u32>,
}

impl Deps {
    /// The reactive indices, sorted ascending, deduplicated.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn contains(&self, index: u32) -> bool {
        self.indices.binary_search(&index).is_ok()
    }

    /// The dependencies as a bitmask (bit `i` set iff index `i` is present), or
    /// `None` if any index is `>= 128` (a component with that many reactive
    /// vars needs a wider mask representation).
    pub fn mask_u128(&self) -> Option<u128> {
        let mut mask = 0u128;
        for &i in &self.indices {
            mask |= 1u128.checked_shl(i)?;
        }
        Some(mask)
    }

    pub(crate) fn from_indices(mut indices: Vec<u32>) -> Self {
        indices.sort_unstable();
        indices.dedup();
        Deps { indices }
    }
}

/// Where in the template a dynamic (reactive) expression appears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicKind {
    /// A `${…}` interpolation in element text.
    Text,
    /// A bound attribute `:name="expr"`. Holds the attribute name.
    Attribute(String),
    /// A `${…}` interpolation inside a static attribute value. Holds the name.
    AttributeText(String),
    /// The l-value of a two-way binding `::name="lvalue"`. Holds the name.
    TwoWay(String),
    /// A `:if` / `:elseif` condition.
    IfCondition,
    /// The iterable of a `:for` loop (the right side of `of`/`in`).
    ForIterable,
}

/// A reactive expression in the template, annotated with the reactive variables
/// it reads. The generator emits an update for this part when any dependency
/// changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicPart {
    pub kind: DynamicKind,
    /// The expression text (raw JS).
    pub expr: String,
    /// File-absolute span of the expression.
    pub range: TextRange,
    /// Reactive variables this expression reads (transitively through any
    /// top-level functions it calls).
    pub deps: Deps,
}

/// An `@event="handler"` binding, annotated with the reactive variables the
/// handler writes (so the generator knows what to mark dirty when it fires).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHandler {
    pub event: String,
    /// The handler expression text (raw JS).
    pub handler: String,
    /// File-absolute span of the handler expression.
    pub range: TextRange,
    /// Reactive variables the handler mutates (transitively through any
    /// top-level functions it calls).
    pub writes: Deps,
}

/// A fully resolved component, ready for code generation.
///
/// This is the boundary the project is built up to: everything a generator
/// needs is here — props, child components, the template IR, the numbered
/// reactive variables, the dependency set of each dynamic part, and the write
/// set of each event handler. Generating code from this is the next, separate
/// phase.
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
    /// Reactive expressions in the template, each with its dependency set.
    pub dynamics: Vec<DynamicPart>,
    /// Event handlers, each with the reactive variables it writes.
    pub handlers: Vec<ResolvedHandler>,
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
