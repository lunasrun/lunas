//! The Lunas single-file-component parser.
//!
//! Parsing is layered (see `DESIGN.md`): a Pest grammar splits the `.lunas`
//! file into language blocks and directives (`parser1`), then semantic
//! lowering (`lower`) validates them and invokes the HTML sub-parser to produce
//! a [`ParsedFile`]. The parser never panics and reports problems as
//! [`Diagnostic`]s rather than failing hard.
//!
//! Script blocks are extracted as raw text only; parsing them to an AST and any
//! TypeScript lowering live in the separate `lunas_script` crate, so this crate
//! carries no JS/TS toolchain dependency.

mod ir;
mod lower;
mod parser1;
mod template;

pub use ir::{BlockSource, Directive, HtmlBlock, PropInput, ScriptBlock, StyleBlock, UseComponent};
pub use template::{
    BranchKind, ComponentUse, Expr, ForBlock, ForHeader, IfBranch, IfChain, Interpolation,
    StaticValue, Template, TemplateAttr, TemplateElement, TemplateNode, TemplateText, TextSegment,
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

    /// Returns which language block a byte offset falls in, if any. A language
    /// server uses this to route a request to the right backend (e.g. proxy
    /// into the TypeScript LS when the cursor is in the `script:` block).
    pub fn block_at(&self, offset: TextSize) -> Option<BlockKind> {
        if let Some(s) = &self.script {
            if s.source.range.contains_inclusive(offset) {
                return Some(BlockKind::Script);
            }
        }
        if let Some(h) = &self.html {
            if h.source.range.contains_inclusive(offset) {
                return Some(BlockKind::Html);
            }
        }
        if let Some(s) = &self.style {
            if s.source.range.contains_inclusive(offset) {
                return Some(BlockKind::Style);
            }
        }
        None
    }

    /// Like [`block_at`](Self::block_at) but takes a line/column position.
    pub fn block_at_line_col(&self, pos: LineCol) -> Option<BlockKind> {
        self.block_at(self.line_index.offset(pos))
    }
}

/// A language block in a `.lunas` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Html,
    Style,
    Script,
}

/// Parses a `.lunas` source string. Always returns a [`ParsedFile`]; any
/// problems are reported in the diagnostics vector (never `Err`, never panics).
///
/// ```
/// use lunas_parser::{parse, Directive};
///
/// let src = "\
/// @input count:number = 0
/// html:
///     <div>${count}</div>
/// script:
///     let count = 0
/// ";
/// let (file, diagnostics) = parse(src);
/// assert!(diagnostics.is_empty());
/// assert!(file.html.is_some() && file.script.is_some());
/// assert!(matches!(file.directives[0], Directive::Input(_)));
/// ```
pub fn parse(source: &str) -> (ParsedFile, Vec<Diagnostic>) {
    lower::lower(source)
}
