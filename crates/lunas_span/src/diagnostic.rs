use crate::line_index::LineIndex;
use crate::text_size::TextRange;
use serde::{Deserialize, Serialize};

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

impl Severity {
    /// Lowercase label, e.g. for rendering (`error`, `warning`, `hint`).
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Hint => "hint",
        }
    }
}

/// A single problem found while parsing, attached to a source range.
///
/// Parsers accumulate diagnostics rather than aborting, so a `Diagnostic` is
/// never an error in the `Result` sense — it is data describing a recoverable
/// problem at a location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: TextRange,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(range: TextRange, message: impl Into<String>) -> Self {
        Diagnostic {
            range,
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn warning(range: TextRange, message: impl Into<String>) -> Self {
        Diagnostic {
            range,
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn hint(range: TextRange, message: impl Into<String>) -> Self {
        Diagnostic {
            range,
            severity: Severity::Hint,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders the diagnostic in a compact rustc-like form against `source`:
    ///
    /// ```text
    /// error: <message>
    ///  --> 3:5
    ///   | <the offending source line>
    ///   |     ^^^^
    /// ```
    ///
    /// Line/column are 1-based in the output. The caret run spans the
    /// diagnostic range, clamped to the line.
    pub fn render(&self, source: &str, index: &LineIndex) -> String {
        let start = index.line_col(self.range.start());
        let line_start = index.offset(crate::LineCol::new(start.line, 0));
        let line_text = source[line_start.as_usize()..]
            .split(['\n', '\r'])
            .next()
            .unwrap_or("");

        let caret_col = start.col as usize;
        // Caret length: range length, but at least 1 and clamped to the line.
        let span_len = (self.range.end().raw() - self.range.start().raw()).max(1) as usize;
        let max_len = line_text.len().saturating_sub(caret_col).max(1);
        let caret_len = span_len.min(max_len);

        format!(
            "{}: {}\n --> {}:{}\n  | {}\n  | {}{}",
            self.severity.label(),
            self.message,
            start.line + 1,
            start.col + 1,
            line_text,
            " ".repeat(caret_col),
            "^".repeat(caret_len),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_labels() {
        assert_eq!(Severity::Error.label(), "error");
        assert_eq!(Severity::Warning.label(), "warning");
        assert_eq!(Severity::Hint.label(), "hint");
    }

    #[test]
    fn constructors_set_severity() {
        let r = TextRange::at(0, 1);
        assert!(Diagnostic::error(r, "boom").is_error());
        assert!(!Diagnostic::warning(r, "meh").is_error());
        assert_eq!(Diagnostic::hint(r, "fyi").severity, Severity::Hint);
    }

    #[test]
    fn message_accepts_string_and_str() {
        let r = TextRange::at(0, 1);
        let a = Diagnostic::error(r, "literal");
        let b = Diagnostic::error(r, String::from("owned"));
        assert_eq!(a.message, "literal");
        assert_eq!(b.message, "owned");
    }

    #[test]
    fn render_points_at_the_range() {
        let src = "html:\n    <div ::=\"x\">\n";
        let index = LineIndex::new(src);
        // The `::=` two-way attr with an empty name spans bytes 10..13 (line 1).
        let diag = Diagnostic::error(TextRange::at(10, 13), "bad attribute");
        let out = diag.render(src, &index);
        assert!(out.starts_with("error: bad attribute\n --> 2:5\n"), "{out}");
        assert!(out.contains("    <div ::=\"x\">"));
        // Caret line: 4 leading spaces + 3 carets under `::=`.
        assert!(out.contains("\n  |     ^^^"), "{out}");
    }

    #[test]
    fn render_clamps_caret_to_line() {
        let src = "abc";
        let index = LineIndex::new(src);
        let diag = Diagnostic::warning(TextRange::at(1, 99), "x");
        let out = diag.render(src, &index);
        // Caret length clamped so it does not exceed the line.
        assert!(out.contains("\n  | abc\n  |  ^^"), "{out}");
    }

    #[test]
    fn serde_roundtrip() {
        let d = Diagnostic::warning(TextRange::at(2, 6), "careful");
        let json = serde_json::to_string(&d).unwrap();
        let back: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
