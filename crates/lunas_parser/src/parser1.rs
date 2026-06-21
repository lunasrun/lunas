//! Stage 1: split a `.lunas` file into raw blocks and directives using the
//! Pest grammar. Implemented by the lunas-parser agent.

use lunas_span::TextRange;

/// A raw, un-analyzed top-level item produced by the Pest grammar.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawItem {
    LanguageBlock(RawLanguageBlock),
    Directive(RawDirective),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawLanguageBlock {
    pub name: String,
    /// Range of the block body (excluding the `name:` keyword line).
    pub body_range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawDirective {
    pub keyword: String,
    pub params: Option<String>,
    /// Range of the directive body line(s), if any.
    pub body_range: Option<TextRange>,
}
