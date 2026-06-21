//! State-machine tokenizer. Implemented by the html-parser agent.

use lunas_span::TextRange;

/// A lexical token. String content is kept as ranges into the source; callers
/// slice the original `&str` rather than copying during lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Doctype,
    /// `<name` — the start of an open tag, before any attributes.
    OpenTagStart { name: TextRange },
    /// An attribute inside an open tag.
    Attribute {
        name: TextRange,
        value: Option<TextRange>,
    },
    /// `>` ending an open tag.
    OpenTagEnd,
    /// `/>` ending a self-closing tag.
    SelfCloseTagEnd,
    /// `</name>`.
    CloseTag { name: TextRange },
    /// Text content between tags.
    Text,
    /// `<!-- … -->`; range covers the inner content only.
    Comment { content: TextRange },
    /// Raw text content of a script/style/title/textarea element.
    RawText,
    /// An unexpected character; enables error recovery.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub range: TextRange,
}
