//! Lightweight static analysis over a parsed script, for consumers that need to
//! know what a `script:` block declares (e.g. reactivity analysis must know
//! which identifiers in template expressions refer to component bindings).

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{
    ArrayPat, AssignExpr, AssignTarget, AssignTargetPat, Decl, Expr, Ident, ImportSpecifier,
    ModuleDecl, ModuleItem, ObjectPat, ObjectPatProp, Pat, SimpleAssignTarget, Stmt, UpdateExpr,
    VarDecl,
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
    let module = parse_expr_module(code)?;
    let mut collector = RefCollector { names: Vec::new() };
    module.visit_with(&mut collector);
    Ok(collector.names)
}

/// Parses `code` as an expression by wrapping it in `(…);` so a bare expression
/// (an interpolation / attribute value) parses as a module.
fn parse_expr_module(code: &str) -> Result<swc_ecma_ast::Module, ScriptParseError> {
    let cm: Lrc<SourceMap> = Default::default();
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
    parser
        .parse_module()
        .map_err(|e| ScriptParseError::Parse(format!("{:?}", e)))
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

/// Like [`referenced_identifiers`] but excludes names bound *locally* within the
/// expression — function/arrow parameters and local declarations. So
/// `items.map(x => x.active)` reports `items`, not `x`. This is the accurate
/// input for reactivity: the free variables an expression actually depends on.
///
/// ```
/// use lunas_script::free_identifiers;
///
/// assert_eq!(free_identifiers("items.map(x => x.active)").unwrap(), ["items"]);
/// assert_eq!(free_identifiers("() => count + 1").unwrap(), ["count"]);
/// ```
pub fn free_identifiers(code: &str) -> Result<Vec<String>, ScriptParseError> {
    let module = parse_expr_module(code)?;
    let mut refs = RefCollector { names: Vec::new() };
    module.visit_with(&mut refs);
    let mut bound = BoundCollector {
        names: Default::default(),
    };
    module.visit_with(&mut bound);
    Ok(refs
        .names
        .into_iter()
        .filter(|n| !bound.names.contains(n))
        .collect())
}

/// Collects names bound locally inside an expression: function/arrow params and
/// nested local declarations.
struct BoundCollector {
    names: std::collections::HashSet<String>,
}

impl Visit for BoundCollector {
    fn visit_arrow_expr(&mut self, n: &swc_ecma_ast::ArrowExpr) {
        for p in &n.params {
            collect_pat_names(p, &mut self.names);
        }
        n.visit_children_with(self);
    }

    fn visit_function(&mut self, n: &swc_ecma_ast::Function) {
        for p in &n.params {
            collect_pat_names(&p.pat, &mut self.names);
        }
        n.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, n: &swc_ecma_ast::VarDeclarator) {
        collect_pat_names(&n.name, &mut self.names);
        n.visit_children_with(self);
    }
}

fn collect_pat_names(pat: &Pat, out: &mut std::collections::HashSet<String>) {
    let mut v = Vec::new();
    collect_pat(pat, &mut v);
    out.extend(v);
}

/// Returns the identifiers *assigned to* (mutated) by `code`: targets of `=`
/// and compound assignments, and of `++`/`--`. For a member target the root
/// object is reported (`obj.x = 1` → `obj`), since mutating a property mutates
/// the binding. Combined with [`declared_bindings`], this tells the orchestrator
/// which component state a handler changes (so it can trigger reactive updates).
///
/// ```
/// use lunas_script::assigned_identifiers;
///
/// assert_eq!(assigned_identifiers("count = count + 1; obj.x = 2; n++").unwrap(),
///            ["count", "obj", "n"]);
/// ```
pub fn assigned_identifiers(code: &str) -> Result<Vec<String>, ScriptParseError> {
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

    let mut collector = AssignCollector { names: Vec::new() };
    module.visit_with(&mut collector);
    Ok(collector.names)
}

struct AssignCollector {
    names: Vec<String>,
}

impl Visit for AssignCollector {
    fn visit_assign_expr(&mut self, n: &AssignExpr) {
        match &n.left {
            AssignTarget::Simple(s) => self.collect_simple(s),
            AssignTarget::Pat(AssignTargetPat::Array(a)) => self.collect_array(a),
            AssignTarget::Pat(AssignTargetPat::Object(o)) => self.collect_object(o),
            AssignTarget::Pat(_) => {}
        }
        // Recurse into the right-hand side to catch nested assignments / updates.
        n.right.visit_with(self);
    }

    fn visit_update_expr(&mut self, n: &UpdateExpr) {
        if let Some(id) = root_ident(&n.arg) {
            self.names.push(id.sym.to_string());
        }
        n.arg.visit_with(self);
    }
}

impl AssignCollector {
    fn collect_simple(&mut self, target: &SimpleAssignTarget) {
        match target {
            SimpleAssignTarget::Ident(b) => self.names.push(b.id.sym.to_string()),
            SimpleAssignTarget::Member(m) => {
                if let Some(id) = root_ident(&m.obj) {
                    self.names.push(id.sym.to_string());
                }
            }
            SimpleAssignTarget::Paren(p) => {
                if let Some(id) = root_ident(&p.expr) {
                    self.names.push(id.sym.to_string());
                }
            }
            _ => {}
        }
    }

    fn collect_array(&mut self, pat: &ArrayPat) {
        let mut names = Vec::new();
        collect_array_pat(pat, &mut names);
        self.names.extend(names);
    }

    fn collect_object(&mut self, pat: &ObjectPat) {
        let mut names = Vec::new();
        collect_object_pat(pat, &mut names);
        self.names.extend(names);
    }
}

/// The leftmost identifier of a (possibly nested member / parenthesized) expr.
fn root_ident(expr: &Expr) -> Option<&Ident> {
    match expr {
        Expr::Ident(id) => Some(id),
        Expr::Member(m) => root_ident(&m.obj),
        Expr::Paren(p) => root_ident(&p.expr),
        Expr::OptChain(o) => o.base.as_member().and_then(|m| root_ident(&m.obj)),
        _ => None,
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
        Pat::Array(arr) => collect_array_pat(arr, out),
        Pat::Object(obj) => collect_object_pat(obj, out),
        Pat::Rest(rest) => collect_pat(&rest.arg, out),
        Pat::Assign(assign) => collect_pat(&assign.left, out),
        _ => {}
    }
}

fn collect_array_pat(arr: &ArrayPat, out: &mut Vec<String>) {
    for elem in arr.elems.iter().flatten() {
        collect_pat(elem, out);
    }
}

fn collect_object_pat(obj: &ObjectPat, out: &mut Vec<String>) {
    for prop in &obj.props {
        match prop {
            ObjectPatProp::KeyValue(kv) => collect_pat(&kv.value, out),
            ObjectPatProp::Assign(a) => out.push(a.key.id.sym.to_string()),
            ObjectPatProp::Rest(r) => collect_pat(&r.arg, out),
        }
    }
}
