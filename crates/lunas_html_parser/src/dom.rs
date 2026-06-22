//! The DOM tree produced by the HTML parser.
//!
//! This module defines the *interface* between the HTML parser and its
//! consumers. The types are intentionally simple owned trees with spans on
//! every node so downstream tooling (the Lunas compiler and language server)
//! can map any node back to its source location.

use lunas_span::{TextRange, TextSize};
use serde::{Deserialize, Serialize};

/// The kind of document a parsed tree represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomKind {
    /// A full document beginning with `<!DOCTYPE html>`.
    Document,
    /// A fragment: one or more nodes with no doctype (the Lunas common case).
    Fragment,
    /// No nodes at all (empty or whitespace-only input).
    Empty,
}

/// The root of a parsed HTML tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dom {
    pub kind: DomKind,
    pub children: Vec<Node>,
}

impl Dom {
    /// Translates every node range forward by `by` bytes.
    ///
    /// [`crate::parse_html`] produces ranges relative to the start of its
    /// input. When the HTML was a slice of a larger file, the caller rebases
    /// the whole tree onto the file's coordinate space with this method.
    pub fn shift_ranges(&mut self, by: TextSize) {
        for child in &mut self.children {
            child.shift_ranges(by);
        }
    }
}

impl Node {
    fn shift_ranges(&mut self, by: TextSize) {
        match self {
            Node::Element(e) => e.shift_ranges(by),
            Node::Text(t) => t.range = t.range.shifted(by),
            Node::Comment(c) => c.range = c.range.shifted(by),
        }
    }
}

impl Element {
    fn shift_ranges(&mut self, by: TextSize) {
        self.range = self.range.shifted(by);
        self.open_tag_range = self.open_tag_range.shifted(by);
        for attr in &mut self.attributes {
            attr.range = attr.range.shifted(by);
        }
        for child in &mut self.children {
            child.shift_ranges(by);
        }
    }
}

/// A node in the DOM tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Element(Element),
    Text(Text),
    Comment(Comment),
}

impl Node {
    /// The source range this node spans.
    pub fn range(&self) -> TextRange {
        match self {
            Node::Element(e) => e.range,
            Node::Text(t) => t.range,
            Node::Comment(c) => c.range,
        }
    }
}

/// Whether an element can have children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    /// A normal element with an open and close tag, e.g. `<div>…</div>`.
    Normal,
    /// A void element with no close tag, e.g. `<br>` or `<img>`.
    Void,
}

/// An HTML element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub name: String,
    pub kind: ElementKind,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
    /// The range of the entire element, from `<` of the open tag to `>` of the
    /// close tag (or the end of the open tag for void elements).
    pub range: TextRange,
    /// The range of just the open tag, `<name …>`.
    pub open_tag_range: TextRange,
}

/// An attribute on an element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    /// `None` for boolean attributes like `disabled`.
    pub value: Option<String>,
    pub range: TextRange,
}

/// A run of text content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub value: String,
    pub range: TextRange,
}

/// An HTML comment. `value` excludes the `<!--` and `-->` delimiters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub value: String,
    pub range: TextRange,
}
