//! Recursive-descent tree builder. Implemented by the html-parser agent.

use crate::dom::{Dom, DomKind};
use crate::ParseResult;

pub(crate) fn parse(_source: &str) -> ParseResult {
    // Placeholder until the html-parser agent fills this in.
    ParseResult {
        dom: Dom {
            kind: DomKind::Empty,
            children: Vec::new(),
        },
        diagnostics: Vec::new(),
    }
}
