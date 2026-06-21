//! Shared span and diagnostic primitives for the Lunas parser crates.
//!
//! This crate is the frozen interface boundary between `lunas_parser` and
//! `lunas_html_parser`: both depend on these types so ranges and diagnostics
//! produced by one are directly usable by the other. It contains no parsing
//! logic and depends only on `serde`.

mod diagnostic;
mod line_index;
mod text_size;

pub use diagnostic::{Diagnostic, Severity};
pub use line_index::{LineCol, LineIndex};
pub use text_size::{TextRange, TextSize};
