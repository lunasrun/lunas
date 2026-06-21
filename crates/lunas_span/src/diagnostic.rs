use crate::text_size::TextRange;
use serde::{Deserialize, Serialize};

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Hint,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn serde_roundtrip() {
        let d = Diagnostic::warning(TextRange::at(2, 6), "careful");
        let json = serde_json::to_string(&d).unwrap();
        let back: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
