//! Stage 2: semantic lowering of raw items into a [`ParsedFile`].
//! Implemented by the lunas-parser agent.

use crate::ParsedFile;
use lunas_span::{Diagnostic, LineIndex};

pub(crate) fn lower(source: &str) -> (ParsedFile, Vec<Diagnostic>) {
    // Placeholder until the lunas-parser agent fills this in.
    let parsed = ParsedFile {
        html: None,
        style: None,
        script: None,
        directives: Vec::new(),
        line_index: LineIndex::new(source),
    };
    (parsed, Vec::new())
}
