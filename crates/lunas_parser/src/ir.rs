//! The public output of the parser: the lowered representation of a `.lunas`
//! file consumed by the code generator and the language server.

use lunas_html_parser::Dom;
use lunas_span::TextRange;
use serde::{Deserialize, Serialize};

/// A language block's extracted body together with its location in the source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSource {
    /// The block body with its common indentation stripped.
    pub text: String,
    /// The range of the body within the original `.lunas` file.
    pub range: TextRange,
}

/// The parsed `html:` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HtmlBlock {
    pub source: BlockSource,
    pub dom: Dom,
}

/// The parsed `style:` block (kept as raw CSS text for now).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleBlock {
    pub source: BlockSource,
}

/// The parsed `script:` block.
///
/// The parser only locates and extracts the raw script text; parsing it to an
/// AST and any TypeScript-to-JavaScript lowering are separate concerns handled
/// by the `lunas_script` crate, so the `.lunas` syntax parser stays free of any
/// JS/TS toolchain dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptBlock {
    pub source: BlockSource,
}

/// A metadata directive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Directive {
    Input(PropInput),
    UseComponent(UseComponent),
    UseAutoRouting,
    UseRouting,
}

/// `@input` — a component prop declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropInput {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub nullable: bool,
    pub range: TextRange,
}

/// `@use` — a child component import.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseComponent {
    pub component_name: String,
    pub path: String,
    pub range: TextRange,
}
