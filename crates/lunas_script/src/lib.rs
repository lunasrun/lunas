//! JavaScript/TypeScript handling for the Lunas compiler, built on SWC.
//!
//! This crate is the **AST parser** layer, deliberately separate from the
//! `.lunas` syntax parser (`lunas_parser`). It does three things:
//!
//! - [`parse_to_ast_json`] — parse a script (TypeScript *or* JavaScript) into an
//!   AST. TypeScript is parsed natively; there is no need to strip types first.
//! - [`transform_ts_to_js`] — a downstream transform that lowers TypeScript to
//!   JavaScript. This operates *after* parsing and is not a prerequisite for
//!   obtaining an AST.
//! - [`parse_for`] — parse a Lunas `for` loop header's JS binding/iterable.
//! - [`declared_bindings`] — list the top-level names a script declares.
//!
//! Keeping this separate means `lunas_parser` has no SWC dependency and stays
//! focused on the single-file-component grammar.

mod analysis;
mod ast;
mod for_header;
mod transform;

pub use analysis::{
    analyze_script, assigned_identifiers, declared_bindings, free_identifiers, function_mutations,
    referenced_identifiers, referenced_identifiers_with_spans, ScriptAnalysis,
};
pub use ast::{parse_to_ast_json, ScriptParseError};
pub use for_header::{parse_for, ForKind, ParsedFor};
pub use transform::{transform_ts_to_js, TsToJsError};
