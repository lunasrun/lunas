//! Brace/string-balanced scanner that splits a text run into literal and
//! `${…}` interpolation segments.
//!
//! Unlike the old `main` implementation (which stopped at the first `}`), this
//! tracks brace depth and skips over string/template literals so expressions
//! like `${ {a:1}.a }` or `${ "}" }` terminate correctly.

use crate::template::ir::{Interpolation, TextSegment};
use lunas_span::{Diagnostic, TextRange, TextSize};

/// Scans `text`, whose first byte is at file offset `base`, into segments.
/// Appends any problems to `diagnostics`. Never panics.
pub(super) fn scan_segments(
    text: &str,
    base: TextSize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TextSegment> {
    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut literal_start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
            // Flush the literal run that precedes this interpolation.
            if i > literal_start {
                segments.push(literal_segment(text, base, literal_start, i));
            }

            let open = i;
            let expr_start = i + 2;
            match find_close(bytes, expr_start) {
                Some(close) => {
                    let expr = text[expr_start..close].to_string();
                    let range = abs(base, open, close + 1);
                    let expr_range = abs(base, expr_start, close);
                    if expr.trim().is_empty() {
                        diagnostics.push(Diagnostic::warning(range, "empty interpolation `${}`"));
                    }
                    segments.push(TextSegment::Interpolation(Interpolation {
                        expr,
                        range,
                        expr_range,
                    }));
                    i = close + 1;
                    literal_start = i;
                }
                None => {
                    // Unterminated: report and keep the rest as literal text so
                    // the tree still builds.
                    diagnostics.push(Diagnostic::error(
                        abs(base, open, bytes.len()),
                        "unterminated interpolation: missing `}` for `${`",
                    ));
                    break;
                }
            }
        } else {
            i += 1;
        }
    }

    if literal_start < bytes.len() {
        segments.push(literal_segment(text, base, literal_start, bytes.len()));
    }
    segments
}

fn literal_segment(text: &str, base: TextSize, start: usize, end: usize) -> TextSegment {
    TextSegment::Literal {
        text: text[start..end].to_string(),
        range: abs(base, start, end),
    }
}

fn abs(base: TextSize, start: usize, end: usize) -> TextRange {
    TextRange::new(
        base + TextSize::new(start as u32),
        base + TextSize::new(end as u32),
    )
}

/// Finds the byte index of the `}` that closes an interpolation opened just
/// before `from`, balancing nested braces and skipping string/template
/// literals, regex literals, and comments. Returns `None` if no balanced close
/// exists.
///
/// Regex-vs-division is disambiguated by `prev_value`: a `/` starts a regex
/// unless the previous significant token could end a value (an identifier, a
/// closing `) ] }`, a string/regex). This is the standard heuristic and covers
/// the common cases; the rare `return /re/` (regex right after a keyword) is
/// treated as division but is not expected in interpolations.
fn find_close(bytes: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = from;
    // Whether the previous significant token could be the end of a value.
    let mut prev_value = false;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'}' if depth == 0 => return Some(i),
            b'}' => {
                depth -= 1;
                prev_value = true;
                i += 1;
            }
            b'{' => {
                depth += 1;
                prev_value = false;
                i += 1;
            }
            b'(' | b'[' => {
                prev_value = false;
                i += 1;
            }
            b')' | b']' => {
                prev_value = true;
                i += 1;
            }
            b'"' | b'\'' | b'`' => {
                i = skip_string(bytes, i + 1, b);
                prev_value = true;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_block_comment(bytes, i + 2);
            }
            b'/' if !prev_value => {
                i = skip_regex(bytes, i + 1);
                prev_value = true;
            }
            b'/' => {
                prev_value = false;
                i += 1;
            }
            _ if b.is_ascii_whitespace() => i += 1,
            _ if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' => {
                prev_value = true;
                i += 1;
            }
            _ => {
                prev_value = false;
                i += 1;
            }
        }
    }
    None
}

/// Skips a regex literal whose opening `/` is at `i-1`. Handles `\` escapes and
/// `[...]` character classes (where `/` does not terminate), then any flags.
fn skip_regex(bytes: &[u8], mut i: usize) -> usize {
    let mut in_class = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                return i;
            }
            b'\n' => return i, // a regex literal cannot span lines; bail
            _ => {}
        }
        i += 1;
    }
    i
}

/// Skips a `/* … */` block comment whose body starts at `from`.
fn skip_block_comment(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

/// Skips to just past the closing quote of a string literal opened at `i-1`
/// with delimiter `quote`. Handles backslash escapes; for template literals,
/// skips over `${…}` substitutions so their `}` is not mistaken for the close.
fn skip_string(bytes: &[u8], mut i: usize, quote: u8) -> usize {
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' {
            i += 2;
            continue;
        }
        if c == quote {
            return i + 1;
        }
        if quote == b'`' && c == b'$' && bytes.get(i + 1) == Some(&b'{') {
            // Nested template substitution: skip its balanced braces.
            i = skip_template_subst(bytes, i + 2);
            continue;
        }
        i += 1;
    }
    i
}

/// Skips a `${…}` substitution body (starting just after `${`) inside a
/// template literal, returning the index just past its closing `}`.
fn skip_template_subst(bytes: &[u8], from: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'}' if depth == 0 => return i + 1,
            b'}' => depth -= 1,
            b'{' => depth += 1,
            q @ (b'"' | b'\'' | b'`') => {
                i = skip_string(bytes, i + 1, q);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    i
}
