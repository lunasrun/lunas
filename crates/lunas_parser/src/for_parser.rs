//! Parser for `for … of` / `for … in` loop headers used in Lunas templates.
//! Implemented by the lunas-parser agent.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForKind {
    Of,
    In,
}

/// The parsed pieces of a `for` loop header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedFor {
    pub kind: ForKind,
    /// The binding pattern on the left of `of`/`in`, e.g. `item` or `[i, v]`.
    pub binding: String,
    /// The iterable expression on the right.
    pub iterable: String,
}

/// Parses a `for` loop header such as `item of items` or `[i, v] in obj`.
/// Returns `None` if the input is not a recognizable header.
pub fn parse_for(_input: &str) -> Option<ParsedFor> {
    // Placeholder until the lunas-parser agent fills this in.
    None
}
