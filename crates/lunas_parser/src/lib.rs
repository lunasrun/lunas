//! The Lunas single-file-component parser.
//!
//! Parsing is layered (see `DESIGN.md`): a Pest grammar splits the `.lunas`
//! file into language blocks and directives (`parser1`), then semantic
//! lowering (`lower`) validates them and invokes the HTML and JS sub-parsers
//! to produce a [`ParsedFile`]. The parser never panics and reports problems
//! as [`Diagnostic`]s rather than failing hard.

mod for_parser;
mod ir;
mod lower;
mod parser1;
mod swc_parser;
mod ts_to_js;

#[cfg(test)]
mod tests;

pub use for_parser::{parse_for, ForKind, ParsedFor};
pub use ir::{
    BlockSource, Directive, HtmlBlock, PropInput, ScriptBlock, StyleBlock, UseComponent,
};

pub use lunas_span::{Diagnostic, LineCol, LineIndex, Severity, TextRange, TextSize};

/// A fully parsed `.lunas` file.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub html: Option<HtmlBlock>,
    pub style: Option<StyleBlock>,
    pub script: Option<ScriptBlock>,
    pub directives: Vec<Directive>,
    /// Line index over the original source, for position mapping.
    pub line_index: LineIndex,
}

impl ParsedFile {
    /// Maps a position in the `.lunas` file to the equivalent position inside
    /// the extracted script text. Returns `None` if the position lies outside
    /// the script block. Uses line arithmetic only — no re-parsing.
    pub fn lunas_to_script(&self, pos: LineCol) -> Option<LineCol> {
        let script = self.script.as_ref()?;
        let start = self.line_index.line_col(script.source.range.start());
        let end = self.line_index.line_col(script.source.range.end());
        if pos.line < start.line || pos.line > end.line {
            return None;
        }
        Some(LineCol::new(pos.line - start.line, pos.col))
    }

    /// Inverse of [`lunas_to_script`](Self::lunas_to_script): maps a position
    /// in the extracted script text back to the `.lunas` file.
    pub fn script_to_lunas(&self, pos: LineCol) -> Option<LineCol> {
        let script = self.script.as_ref()?;
        let start = self.line_index.line_col(script.source.range.start());
        Some(LineCol::new(pos.line + start.line, pos.col))
    }
}

/// Parses a `.lunas` source string. Always returns a [`ParsedFile`]; any
/// problems are reported in the diagnostics vector.
pub fn parse(source: &str) -> (ParsedFile, Vec<Diagnostic>) {
    lower::lower(source)
}
