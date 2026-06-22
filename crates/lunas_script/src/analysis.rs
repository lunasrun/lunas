//! Lightweight static analysis over a parsed script, for consumers that need to
//! know what a `script:` block declares (e.g. reactivity analysis must know
//! which identifiers in template expressions refer to component bindings).

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{
    Decl, Ident, ImportSpecifier, ModuleDecl, ModuleItem, ObjectPatProp, Pat, Stmt, VarDecl,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use crate::ast::ScriptParseError;

/// Returns the names of all top-level bindings declared by `code`: `let`/
/// `const`/`var` (including destructured names), `function` and `class`
/// declarations, and `import` locals. Order follows source order; duplicates
/// are preserved (the caller dedups if needed).
///
/// ```
/// use lunas_script::declared_bindings;
///
/// let names = declared_bindings("let count = 0\nconst { x } = p\nfunction f(){}").unwrap();
/// assert_eq!(names, ["count", "x", "f"]);
/// ```
pub fn declared_bindings(code: &str) -> Result<Vec<String>, ScriptParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(Lrc::new(FileName::Anon), code.to_string());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| ScriptParseError::Parse(format!("{:?}", e)))?;

    let mut names = Vec::new();
    for item in &module.body {
        match item {
            ModuleItem::Stmt(Stmt::Decl(decl)) => collect_decl(decl, &mut names),
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                collect_decl(&export.decl, &mut names)
            }
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                for spec in &import.specifiers {
                    let local = match spec {
                        ImportSpecifier::Named(n) => &n.local,
                        ImportSpecifier::Default(d) => &d.local,
                        ImportSpecifier::Namespace(n) => &n.local,
                    };
                    names.push(local.sym.to_string());
                }
            }
            _ => {}
        }
    }
    Ok(names)
}

/// Returns the identifiers *referenced* (read) by a JS expression or program,
/// in source order. Static member properties (`a.b` → only `a`) and object
/// literal keys are excluded; computed members (`a[k]` → `a`, `k`) and shorthand
/// properties (`{x}` → `x`) are included.
///
/// This does **not** perform scope analysis: a name bound locally inside the
/// expression (e.g. an arrow parameter) is still reported. Callers typically
/// intersect the result with [`declared_bindings`] of the `script:` block to
/// find which component bindings an expression depends on.
///
/// ```
/// use lunas_script::referenced_identifiers;
///
/// let ids = referenced_identifiers("a.b ? f(c) : d[e]").unwrap();
/// assert_eq!(ids, ["a", "f", "c", "d", "e"]);
/// ```
pub fn referenced_identifiers(code: &str) -> Result<Vec<String>, ScriptParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    // Wrap as a parenthesized expression statement so a bare expression parses.
    let fm = cm.new_source_file(Lrc::new(FileName::Anon), format!("({});", code));
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| ScriptParseError::Parse(format!("{:?}", e)))?;

    let mut collector = RefCollector { names: Vec::new() };
    module.visit_with(&mut collector);
    Ok(collector.names)
}

struct RefCollector {
    names: Vec<String>,
}

impl Visit for RefCollector {
    fn visit_ident(&mut self, n: &Ident) {
        // `undefined`/`NaN`/etc. are idents too, but harmless to report; callers
        // intersect with the binding set anyway.
        self.names.push(n.sym.to_string());
    }
}

fn collect_decl(decl: &Decl, out: &mut Vec<String>) {
    match decl {
        Decl::Var(var) => collect_var(var, out),
        Decl::Fn(f) => out.push(f.ident.sym.to_string()),
        Decl::Class(c) => out.push(c.ident.sym.to_string()),
        _ => {}
    }
}

fn collect_var(var: &VarDecl, out: &mut Vec<String>) {
    for decl in &var.decls {
        collect_pat(&decl.name, out);
    }
}

fn collect_pat(pat: &Pat, out: &mut Vec<String>) {
    match pat {
        Pat::Ident(ident) => out.push(ident.id.sym.to_string()),
        Pat::Array(arr) => {
            for elem in arr.elems.iter().flatten() {
                collect_pat(elem, out);
            }
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::KeyValue(kv) => collect_pat(&kv.value, out),
                    ObjectPatProp::Assign(a) => out.push(a.key.id.sym.to_string()),
                    ObjectPatProp::Rest(r) => collect_pat(&r.arg, out),
                }
            }
        }
        Pat::Rest(rest) => collect_pat(&rest.arg, out),
        Pat::Assign(assign) => collect_pat(&assign.left, out),
        _ => {}
    }
}
