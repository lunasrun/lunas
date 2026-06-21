//! A hand-written HTML parser for the Lunas compiler.
//!
//! Two phases: a state-machine [`lexer`] turns source text into a flat token
//! stream, and a recursive-descent tree builder in [`parser`] assembles a
//! [`Dom`]. The parser never panics and never returns `Err` for malformed
//! HTML — it recovers and records [`Diagnostic`]s, mirroring how browsers and
//! production IDE parsers behave.

mod dom;
mod lexer;
mod parser;

pub use dom::{Attribute, Comment, Dom, DomKind, Element, ElementKind, Node, Text};

use lunas_span::Diagnostic;

/// The result of parsing an HTML source string.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub dom: Dom,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses an HTML fragment or document.
///
/// All node ranges are relative to the start of `source`. When the HTML is
/// embedded in a larger file, the caller rebases ranges by adding the byte
/// offset of `source` within that file.
pub fn parse_html(source: &str) -> ParseResult {
    parser::parse(source)
}

/// The set of void elements, which never have children or a close tag.
pub(crate) fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Elements whose content is treated as raw text (no nested markup parsing).
pub(crate) fn is_raw_text_element(name: &str) -> bool {
    matches!(name, "script" | "style" | "title" | "textarea")
}
