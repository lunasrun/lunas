//! Parses JavaScript source into an SWC AST and projects it to a compact JSON
//! representation.
//!
//! The full SWC AST cannot be serialized here: enabling swc's `serde-impl`
//! feature is incompatible with the serde version the rest of the workspace
//! resolves to. Rather than pin the whole graph to an old serde, this module
//! walks the parsed module and emits a shallow, span-annotated JSON tree of
//! top-level statements. That is enough for the code generator and language
//! server to locate declarations; consumers needing the full AST can re-parse
//! the `ScriptBlock.js` text directly with swc.

use swc_common::{sync::Lrc, FileName, SourceMap, Spanned};
use swc_ecma_ast::{Decl, ModuleDecl, ModuleItem, Stmt};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum SwcParseError {
    #[error("failed to parse JavaScript: {0}")]
    Parse(String),
}

/// Parses JavaScript `code` and returns a shallow JSON projection of its AST.
pub(crate) fn parse_to_ast_json(code: &str) -> Result<serde_json::Value, SwcParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(Lrc::new(FileName::Anon), code.to_string());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| SwcParseError::Parse(format!("{:?}", e)))?;

    let body: Vec<serde_json::Value> = module.body.iter().map(item_json).collect();

    Ok(serde_json::json!({
        "type": "Module",
        "body": body,
    }))
}

fn item_json(item: &ModuleItem) -> serde_json::Value {
    let (kind, span) = match item {
        ModuleItem::ModuleDecl(decl) => (module_decl_kind(decl), decl.span()),
        ModuleItem::Stmt(stmt) => (stmt_kind(stmt), stmt.span()),
    };
    // SWC spans are 1-based with a synthetic file offset; expose the raw lo/hi
    // so callers can correlate, without claiming `.lunas`-absolute positions.
    serde_json::json!({
        "type": kind,
        "span": { "lo": span.lo.0, "hi": span.hi.0 },
    })
}

fn module_decl_kind(decl: &ModuleDecl) -> &'static str {
    match decl {
        ModuleDecl::Import(_) => "ImportDeclaration",
        ModuleDecl::ExportDecl(_) => "ExportDeclaration",
        ModuleDecl::ExportNamed(_) => "ExportNamedDeclaration",
        ModuleDecl::ExportDefaultDecl(_) => "ExportDefaultDeclaration",
        ModuleDecl::ExportDefaultExpr(_) => "ExportDefaultExpression",
        ModuleDecl::ExportAll(_) => "ExportAllDeclaration",
        ModuleDecl::TsImportEquals(_) => "TsImportEquals",
        ModuleDecl::TsExportAssignment(_) => "TsExportAssignment",
        ModuleDecl::TsNamespaceExport(_) => "TsNamespaceExport",
    }
}

fn stmt_kind(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::Decl(Decl::Var(_)) => "VariableDeclaration",
        Stmt::Decl(Decl::Fn(_)) => "FunctionDeclaration",
        Stmt::Decl(Decl::Class(_)) => "ClassDeclaration",
        Stmt::Decl(_) => "Declaration",
        Stmt::Expr(_) => "ExpressionStatement",
        Stmt::Block(_) => "BlockStatement",
        Stmt::If(_) => "IfStatement",
        Stmt::For(_) => "ForStatement",
        Stmt::ForIn(_) => "ForInStatement",
        Stmt::ForOf(_) => "ForOfStatement",
        Stmt::While(_) => "WhileStatement",
        Stmt::Return(_) => "ReturnStatement",
        _ => "Statement",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_js() {
        let ast = parse_to_ast_json("let count = 0;").expect("parse ok");
        assert_eq!(ast["type"], "Module");
        assert!(ast["body"].is_array());
        assert_eq!(ast["body"][0]["type"], "VariableDeclaration");
    }

    #[test]
    fn empty_input_is_ok() {
        let ast = parse_to_ast_json("").expect("parse ok");
        assert_eq!(ast["type"], "Module");
        assert_eq!(ast["body"].as_array().expect("array").len(), 0);
    }

    #[test]
    fn captures_function_and_import() {
        let ast = parse_to_ast_json("import x from 'y';\nfunction f(){}").expect("ok");
        assert_eq!(ast["body"][0]["type"], "ImportDeclaration");
        assert_eq!(ast["body"][1]["type"], "FunctionDeclaration");
    }

    #[test]
    fn invalid_js_is_error() {
        assert!(parse_to_ast_json("let = = =").is_err());
    }
}
