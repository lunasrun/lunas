//! State-machine tokenizer.
//!
//! `tokenize`, [`Token`], and [`TokenKind`] are `pub` so the integration test
//! suite in `tests/` can exercise the lexer directly; they are re-exported
//! under the hidden `crate::internals` facade and are not part of the stable
//! public API.

use lunas_span::{TextRange, TextSize};

/// A lexical token. String content is kept as ranges into the source; callers
/// slice the original `&str` rather than copying during lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
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
pub struct Token {
    pub kind: TokenKind,
    pub range: TextRange,
}

/// Tokenizes `source` into a flat token stream. Never panics; unexpected input
/// is surfaced as `Text` or `Error` tokens so the tree builder can recover.
pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32))
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

/// Characters that may begin a tag name. We accept anything that is not
/// whitespace, `/`, `>`, or `<`, mirroring the lenient name handling browsers
/// use so PascalCase components and custom elements lex correctly.
fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_name_char(b: u8) -> bool {
    !is_ascii_whitespace(b) && !matches!(b, b'/' | b'>' | b'<' | b'=')
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Lexer {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn starts_with_ci(&self, needle: &str) -> bool {
        let end = self.pos + needle.len();
        if end > self.bytes.len() {
            return false;
        }
        self.bytes[self.pos..end].eq_ignore_ascii_case(needle.as_bytes())
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            range: range(start, end),
        });
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if is_ascii_whitespace(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn run(mut self) -> Vec<Token> {
        while self.pos < self.bytes.len() {
            if self.peek() == Some(b'<') {
                self.lex_lt();
            } else {
                self.lex_text();
            }
        }
        self.tokens
    }

    /// Dispatch on what follows a `<`.
    fn lex_lt(&mut self) {
        if self.starts_with_ci("<!--") {
            self.lex_comment();
        } else if self.starts_with_ci("<!doctype") {
            self.lex_doctype();
        } else if self.peek_at(1) == Some(b'/') {
            self.lex_close_tag();
        } else if self.peek_at(1).is_some_and(is_name_start) {
            self.lex_open_tag();
        } else {
            // A stray `<` that does not begin a tag is plain text.
            self.lex_text();
        }
    }

    fn lex_text(&mut self) {
        let start = self.pos;
        // Always consume at least one byte so we make progress even on a lone
        // `<`. After the first byte, stop at the next `<`.
        self.pos += 1;
        while let Some(b) = self.peek() {
            if b == b'<' {
                break;
            }
            self.pos += 1;
        }
        self.push(TokenKind::Text, start, self.pos);
    }

    fn lex_comment(&mut self) {
        let start = self.pos;
        self.pos += 4; // consume "<!--"
        let content_start = self.pos;
        let mut content_end = self.bytes.len();
        let mut end = self.bytes.len();
        while self.pos < self.bytes.len() {
            if self.starts_with_ci("-->") {
                content_end = self.pos;
                end = self.pos + 3;
                self.pos = end;
                break;
            }
            self.pos += 1;
        }
        // Unterminated: content runs to EOF and pos is already at EOF.
        self.push(
            TokenKind::Comment {
                content: range(content_start, content_end),
            },
            start,
            end,
        );
    }

    fn lex_doctype(&mut self) {
        let start = self.pos;
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'>' {
                break;
            }
        }
        self.push(TokenKind::Doctype, start, self.pos);
    }

    fn lex_close_tag(&mut self) {
        let start = self.pos;
        self.pos += 2; // consume "</"
        self.skip_whitespace();
        let name_start = self.pos;
        while self.peek().is_some_and(is_name_char) {
            self.pos += 1;
        }
        let name = range(name_start, self.pos);
        // Tolerate whitespace then consume up to and including `>`.
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'>' {
                break;
            }
        }
        self.push(TokenKind::CloseTag { name }, start, self.pos);
    }

    fn lex_open_tag(&mut self) {
        let start = self.pos;
        self.pos += 1; // consume "<"
        let name_start = self.pos;
        while self.peek().is_some_and(is_name_char) {
            self.pos += 1;
        }
        let name = range(name_start, self.pos);
        let name_text = self.source.get(name_start..self.pos).unwrap_or("");
        let is_raw = crate::is_raw_text_element(&name_text.to_ascii_lowercase());
        self.push(TokenKind::OpenTagStart { name }, start, self.pos);

        // Attributes until the tag closes.
        let self_closed = self.lex_attributes();

        if is_raw && !self_closed {
            self.lex_raw_text(name_text);
        }
    }

    /// Lexes attributes and the closing delimiter of the current open tag.
    /// Returns true if the tag was self-closing (`/>`).
    fn lex_attributes(&mut self) -> bool {
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => return false,
                Some(b'>') => {
                    let s = self.pos;
                    self.pos += 1;
                    self.push(TokenKind::OpenTagEnd, s, self.pos);
                    return false;
                }
                Some(b'/') if self.peek_at(1) == Some(b'>') => {
                    let s = self.pos;
                    self.pos += 2;
                    self.push(TokenKind::SelfCloseTagEnd, s, self.pos);
                    return true;
                }
                Some(b'/') => {
                    // Stray slash inside a tag; skip it.
                    self.pos += 1;
                }
                Some(_) => self.lex_attribute(),
            }
        }
    }

    fn lex_attribute(&mut self) {
        let start = self.pos;
        let name_start = self.pos;
        while self.peek().is_some_and(is_name_char) {
            self.pos += 1;
        }
        if self.pos == name_start {
            // Not a valid name character (e.g. a lone `=`); consume one byte as
            // an error token so we make progress.
            self.pos += 1;
            self.push(TokenKind::Error, start, self.pos);
            return;
        }
        let name = range(name_start, self.pos);

        // Optional `=` (with surrounding whitespace) introduces a value.
        let mut value = None;
        let save = self.pos;
        self.skip_whitespace();
        if self.peek() == Some(b'=') {
            self.pos += 1;
            self.skip_whitespace();
            value = self.lex_attribute_value();
        } else {
            // No `=`: this is a boolean attribute; do not consume trailing ws.
            self.pos = save;
        }

        self.push(TokenKind::Attribute { name, value }, start, self.pos);
    }

    fn lex_attribute_value(&mut self) -> Option<TextRange> {
        match self.peek() {
            Some(q @ (b'"' | b'\'')) => {
                self.pos += 1;
                let v_start = self.pos;
                while let Some(b) = self.peek() {
                    if b == q {
                        break;
                    }
                    self.pos += 1;
                }
                let v_end = self.pos;
                if self.peek() == Some(q) {
                    self.pos += 1;
                }
                Some(range(v_start, v_end))
            }
            _ => {
                let v_start = self.pos;
                while let Some(b) = self.peek() {
                    if is_ascii_whitespace(b) || b == b'>' || (b == b'/' && self.peek_at(1) == Some(b'>')) {
                        break;
                    }
                    self.pos += 1;
                }
                Some(range(v_start, self.pos))
            }
        }
    }

    /// Consumes raw element content up to (but not including) the matching
    /// `</name>` close tag, case-insensitively.
    fn lex_raw_text(&mut self, name: &str) {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            if self.peek() == Some(b'<') && self.peek_at(1) == Some(b'/') {
                // Check for `</name` ignoring case, allowing leading whitespace
                // after `</`.
                let mut probe = self.pos + 2;
                while self.bytes.get(probe).copied().is_some_and(is_ascii_whitespace) {
                    probe += 1;
                }
                let end = probe + name.len();
                if end <= self.bytes.len()
                    && self.bytes[probe..end].eq_ignore_ascii_case(name.as_bytes())
                {
                    // Confirm the name is followed by a delimiter, not a longer
                    // name (e.g. `</scriptable>` must not match `</script>`).
                    let after = self.bytes.get(end).copied();
                    if after.is_none_or(|b| is_ascii_whitespace(b) || b == b'>') {
                        break;
                    }
                }
            }
            self.pos += 1;
        }
        if self.pos > start {
            self.push(TokenKind::RawText, start, self.pos);
        }
    }
}

