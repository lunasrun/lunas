//! Shared span and diagnostic primitives for the Lunas parser crates.
//!
//! This crate is the frozen interface boundary between `lunas_parser` and
//! `lunas_html_parser`: both depend on these types so ranges and diagnostics
//! produced by one are directly usable by the other. It contains no parsing
//! logic and depends only on `serde`.
//!
//! ```
//! use lunas_span::{LineCol, LineIndex, TextRange, TextSize};
//!
//! let src = "ab\ncd";
//! let index = LineIndex::new(src);
//!
//! // A byte range slices the source and maps to a line/column.
//! let range = TextRange::at(3, 5);
//! assert_eq!(range.slice(src), Some("cd"));
//! assert_eq!(index.line_col(range.start()), LineCol::new(1, 0));
//!
//! // Rebasing shifts a range parsed against a substring onto the whole file.
//! assert_eq!(range.shifted(TextSize::new(10)), TextRange::at(13, 15));
//! ```

mod diagnostic;
mod line_index;
mod text_size;

pub use diagnostic::{Diagnostic, Severity};
pub use line_index::{LineCol, LineIndex};
pub use text_size::{TextRange, TextSize};
