//! Low-level CSS scanning helpers.
//!
//! CSS scoping does not need a full CSS grammar. It needs to walk the source
//! byte-by-byte while respecting the four constructs that can hide otherwise
//! significant characters:
//!
//! * strings (`"…"` / `'…'`) — a `{`, `}`, `,`, `]` or `/*` inside a string is
//!   literal text, not structure;
//! * comments (`/* … */`) — likewise inert;
//! * escapes (`\`) — the next byte is consumed verbatim, so `\]` inside an
//!   attribute selector does not close the bracket;
//! * bracket / paren nesting — `,` inside `:not(a, b)` or `[x="a,b"]` does not
//!   split a selector list.
//!
//! The helpers here are deliberately small and total: every function makes
//! forward progress and never indexes out of bounds, which is what lets the
//! higher layers guarantee they never panic.

/// A cursor over CSS source bytes. All positions are byte offsets into the
/// original `&str`, so they can be handed straight to [`lunas_span::TextRange`].
pub(crate) struct Scanner<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Scanner {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn src(&self) -> &'a str {
        self.src
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// The byte at the current position, or `None` at EOF.
    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Advances one byte. Returns the byte consumed, or `None` at EOF.
    pub(crate) fn bump(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    /// Sets the cursor position (clamped to the input length).
    pub(crate) fn seek(&mut self, pos: usize) {
        self.pos = pos.min(self.bytes.len());
    }

    /// True if the remaining input starts with `s`.
    pub(crate) fn starts_with(&self, s: &str) -> bool {
        self.bytes[self.pos.min(self.bytes.len())..].starts_with(s.as_bytes())
    }
}

/// Consumes a `/* … */` comment, assuming the cursor is on the opening `/*`.
/// An unterminated comment consumes to EOF. Returns `true` if a comment was
/// consumed. This never panics and always makes progress when it returns true.
pub(crate) fn consume_comment(sc: &mut Scanner) -> bool {
    if !sc.starts_with("/*") {
        return false;
    }
    sc.bump();
    sc.bump();
    while !sc.is_eof() {
        if sc.starts_with("*/") {
            sc.bump();
            sc.bump();
            return true;
        }
        sc.bump();
    }
    true
}

/// Consumes a string literal, assuming the cursor is on the opening quote
/// (`"` or `'`). Handles backslash escapes; an unterminated string consumes to
/// EOF. Returns `true` if a string was consumed.
pub(crate) fn consume_string(sc: &mut Scanner) -> bool {
    let quote = match sc.peek() {
        Some(q @ (b'"' | b'\'')) => q,
        _ => return false,
    };
    sc.bump();
    while let Some(b) = sc.bump() {
        if b == b'\\' {
            // Escape: consume the next byte too (if any).
            sc.bump();
        } else if b == quote {
            return true;
        }
    }
    true
}

/// The four kinds of nesting-balanced brackets CSS uses.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bracket {
    Paren,
    Square,
}

impl Bracket {
    fn open(self) -> u8 {
        match self {
            Bracket::Paren => b'(',
            Bracket::Square => b'[',
        }
    }

    fn close(self) -> u8 {
        match self {
            Bracket::Paren => b')',
            Bracket::Square => b']',
        }
    }
}

/// Consumes a balanced bracket group, assuming the cursor is on the opening
/// bracket. Respects nested brackets of the same kind, strings, comments, and
/// escapes. An unbalanced group consumes to EOF. Returns `true` if a group was
/// consumed (i.e. the cursor was on the opening bracket).
pub(crate) fn consume_bracketed(sc: &mut Scanner, kind: Bracket) -> bool {
    if sc.peek() != Some(kind.open()) {
        return false;
    }
    sc.bump();
    let mut depth = 1usize;
    while !sc.is_eof() {
        if consume_comment(sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b) if b == kind.open() => {
                depth += 1;
                sc.bump();
            }
            Some(b) if b == kind.close() => {
                depth -= 1;
                sc.bump();
                if depth == 0 {
                    return true;
                }
            }
            _ => {
                sc.bump();
            }
        }
    }
    true
}
