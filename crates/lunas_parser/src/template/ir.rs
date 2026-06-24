//! The binding-aware template IR — the analogue of `lunas_html_parser`'s plain
//! `Dom`, enriched with interpolations, bound/event attributes, components, and
//! grouped control flow.
//!
//! This layer is **purely syntactic**: every embedded JS expression is stored
//! as raw text plus a `.lunas`-file-absolute span. Parsing those expressions
//! (and all reactivity work) is left to the downstream orchestrator, keeping
//! this crate free of any JS/TS toolchain — exactly how the `script:` block is
//! handled.

use lunas_html_parser::{Comment, ElementKind};
use lunas_span::TextRange;
use serde::{Deserialize, Serialize};

/// The binding-aware template tree for an `html:` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub nodes: Vec<TemplateNode>,
}

impl Template {
    /// Visits every node in the tree in pre-order (parents before children),
    /// descending through element/component children, `:if` branch bodies, and
    /// `:for` bodies. Saves consumers (the orchestrator, language server) from
    /// re-implementing the recursive walk.
    pub fn visit<F: FnMut(&TemplateNode)>(&self, f: &mut F) {
        for node in &self.nodes {
            node.visit(f);
        }
    }

    /// Calls `f(text, range)` for every embedded JS expression in the template:
    /// `${…}` interpolations (in text and static attribute values), `:`/`::`/`@`
    /// attribute expressions, `:if`/`:elseif` conditions, and `:for` headers.
    /// Each `range` is `.lunas`-file-absolute. This is the iteration the code
    /// generator runs reactivity analysis over.
    pub fn for_each_expression<F: FnMut(&str, TextRange)>(&self, mut f: F) {
        for node in &self.nodes {
            node.for_each_expression(&mut f);
        }
    }
}

impl TemplateNode {
    fn for_each_expression<F: FnMut(&str, TextRange)>(&self, f: &mut F) {
        match self {
            TemplateNode::Text(t) => segments_expressions(&t.segments, f),
            TemplateNode::Element(e) => {
                attrs_expressions(&e.attrs, f);
                for c in &e.children {
                    c.for_each_expression(f);
                }
            }
            TemplateNode::Component(c) => {
                attrs_expressions(&c.props, f);
                for n in &c.children {
                    n.for_each_expression(f);
                }
            }
            TemplateNode::If(chain) => {
                for b in &chain.branches {
                    if let Some(cond) = &b.condition {
                        f(&cond.text, cond.range);
                    }
                    b.body.for_each_expression(f);
                }
            }
            TemplateNode::For(block) => {
                f(&block.header.text, block.header.range);
                block.body.for_each_expression(f);
            }
            TemplateNode::Comment(_) => {}
        }
    }
}

fn segments_expressions<F: FnMut(&str, TextRange)>(segments: &[TextSegment], f: &mut F) {
    for seg in segments {
        if let TextSegment::Interpolation(i) = seg {
            f(&i.expr, i.expr_range);
        }
    }
}

fn attrs_expressions<F: FnMut(&str, TextRange)>(attrs: &[TemplateAttr], f: &mut F) {
    for attr in attrs {
        match attr {
            TemplateAttr::Bound { expr, .. } => f(&expr.text, expr.range),
            TemplateAttr::TwoWay { lvalue, .. } => f(&lvalue.text, lvalue.range),
            TemplateAttr::Event { handler, .. } => f(&handler.text, handler.range),
            TemplateAttr::Static { value, .. } => {
                if let Some(v) = value {
                    segments_expressions(&v.segments, f);
                }
            }
        }
    }
}

/// A node in the template tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateNode {
    Element(TemplateElement),
    Component(ComponentUse),
    Text(TemplateText),
    Comment(Comment),
    /// A grouped `:if` / `:elseif` / `:else` cascade.
    If(IfChain),
    /// A `:for` loop wrapping a single body node.
    For(ForBlock),
}

impl TemplateNode {
    /// Visits this node and its descendants in pre-order.
    pub fn visit<F: FnMut(&TemplateNode)>(&self, f: &mut F) {
        f(self);
        match self {
            TemplateNode::Element(e) => e.children.iter().for_each(|c| c.visit(f)),
            TemplateNode::Component(c) => c.children.iter().for_each(|n| n.visit(f)),
            TemplateNode::If(chain) => chain.branches.iter().for_each(|b| b.body.visit(f)),
            TemplateNode::For(block) => block.body.visit(f),
            TemplateNode::Text(_) | TemplateNode::Comment(_) => {}
        }
    }
}

/// A run of text that interleaves static literals and `${…}` interpolations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateText {
    pub segments: Vec<TextSegment>,
    pub range: TextRange,
}

impl TemplateText {
    /// Whether this run is insignificant whitespace: no interpolations and only
    /// whitespace in its literal text. The code generator skips such nodes
    /// (they come from indentation/newlines between elements).
    pub fn is_whitespace(&self) -> bool {
        self.segments.iter().all(|seg| match seg {
            TextSegment::Literal { text, .. } => text.trim().is_empty(),
            TextSegment::Interpolation(_) => false,
        })
    }

    /// Whether this run contains at least one `${…}` interpolation.
    pub fn has_interpolation(&self) -> bool {
        self.segments
            .iter()
            .any(|seg| matches!(seg, TextSegment::Interpolation(_)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextSegment {
    Literal { text: String, range: TextRange },
    Interpolation(Interpolation),
}

/// `${ expr }`. `range` covers the whole `${…}`; `expr_range` covers only the
/// inner expression text (for diagnostics / hand-off into JS tooling).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interpolation {
    pub expr: String,
    pub range: TextRange,
    pub expr_range: TextRange,
}

/// A raw JS expression: opaque text plus its file-absolute span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expr {
    pub text: String,
    pub range: TextRange,
}

/// An ordinary HTML element with classified attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateElement {
    pub name: String,
    pub kind: ElementKind,
    pub attrs: Vec<TemplateAttr>,
    pub children: Vec<TemplateNode>,
    pub range: TextRange,
    pub open_tag_range: TextRange,
}

/// A component use: a tag whose name is in the `@use` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentUse {
    pub name: String,
    pub props: Vec<TemplateAttr>,
    pub children: Vec<TemplateNode>,
    pub range: TextRange,
    pub open_tag_range: TextRange,
}

/// An element/component attribute after binding classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateAttr {
    /// A plain attribute; its value may still contain `${…}` interpolations.
    Static {
        name: String,
        value: Option<StaticValue>,
        range: TextRange,
    },
    /// `:name="expr"` — a reactively bound attribute/prop.
    Bound {
        name: String,
        expr: Expr,
        range: TextRange,
    },
    /// `::name="lvalue"` — two-way binding (sugar for `:name` + writeback).
    TwoWay {
        name: String,
        lvalue: Expr,
        range: TextRange,
    },
    /// `@event="handler"` — an event handler.
    Event {
        event: String,
        handler: Expr,
        range: TextRange,
    },
}

/// A static attribute value, which may interleave literals and interpolations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticValue {
    pub segments: Vec<TextSegment>,
    pub range: TextRange,
}

/// A complete `:if` / `:elseif` / `:else` cascade, grouped at parse time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfChain {
    pub branches: Vec<IfBranch>,
    pub range: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchKind {
    If,
    ElseIf,
    Else,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfBranch {
    pub kind: BranchKind,
    /// `None` only for the `:else` branch.
    pub condition: Option<Expr>,
    pub body: Box<TemplateNode>,
    pub range: TextRange,
}

/// A `:for` loop. The header is stored raw; `lunas_script::parse_for` is run
/// downstream by the orchestrator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForBlock {
    pub header: ForHeader,
    pub body: Box<TemplateNode>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForHeader {
    pub text: String,
    pub range: TextRange,
}
