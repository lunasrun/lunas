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
    Ok(collect_bindings(&parse_program(code)?))
}

fn collect_bindings(module: &swc_ecma_ast::Module) -> Vec<String> {
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
    names
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

/// Like [`referenced_identifiers`] but also returns each identifier's byte
/// `TextRange` *within `code`* (0-based). The language server adds the
/// expression's file-absolute start to these to locate references for
/// highlight / rename across a template.
///
/// ```
/// use lunas_script::referenced_identifiers_with_spans;
///
/// let ids = referenced_identifiers_with_spans("a + bb").unwrap();
/// let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
/// assert_eq!(names, ["a", "bb"]);
/// assert_eq!(ids[1].1.slice("a + bb"), Some("bb"));
/// ```
pub fn referenced_identifiers_with_spans(
    code: &str,
) -> Result<Vec<(String, lunas_span::TextRange)>, ScriptParseError> {
    // Wrap as `(code);` so a bare expression parses; the `(` shifts offsets by 1.
    let (module, fm) = parse_source_with_fm(format!("({});", code))?;
    let mut collector = SpanRefCollector { items: Vec::new() };
    module.visit_with(&mut collector);

    let base = fm.start_pos.0; // BytePos of the file's first byte
    const PREFIX: u32 = 1; // the leading "("
    let code_len = code.len() as u32;
    let out = collector
        .items
        .into_iter()
        .filter_map(|(name, lo, hi)| {
            let lo = lo.checked_sub(base)?.checked_sub(PREFIX)?;
            let hi = hi.checked_sub(base)?.checked_sub(PREFIX)?;
            (hi <= code_len && lo <= hi).then(|| (name, lunas_span::TextRange::at(lo, hi)))
        })
        .collect();
    Ok(out)
}

struct SpanRefCollector {
    items: Vec<(String, u32, u32)>,
}

impl Visit for SpanRefCollector {
    fn visit_ident(&mut self, n: &Ident) {
        self.items
            .push((n.sym.to_string(), n.span.lo.0, n.span.hi.0));
    }
}

/// Parses `code` as a program (a sequence of statements / declarations).
fn parse_program(code: &str) -> Result<swc_ecma_ast::Module, ScriptParseError> {
    parse_source(code.to_string())
}

/// Parses `code` as an expression by wrapping it in `(…);` so a bare expression
/// (an interpolation / attribute value) parses as a module.
fn parse_expr_module(code: &str) -> Result<swc_ecma_ast::Module, ScriptParseError> {
    parse_source(format!("({});", code))
}

fn parse_source(text: String) -> Result<swc_ecma_ast::Module, ScriptParseError> {
    Ok(parse_source_with_fm(text)?.0)
}

fn parse_source_with_fm(
    text: String,
) -> Result<(swc_ecma_ast::Module, Lrc<swc_common::SourceFile>), ScriptParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(Lrc::new(FileName::Anon), text);
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
    Ok((module, fm))
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
/// Limitation: bound names are treated as a flat set rather than with full
/// lexical scoping, so a name that is *both* free in an outer scope and a
/// parameter of an inner one (e.g. `a + (a => a)`) is over-excluded. This does
/// not occur in practice for template expressions; full scope resolution can be
/// added if it ever does.
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
    let module = parse_program(code)?;
    let mut collector = AssignCollector { names: Vec::new() };
    module.visit_with(&mut collector);
    Ok(collector.names)
}

/// For each top-level function (a `function` declaration or a `const f = …`
/// arrow/function expression), returns its name and the identifiers its body
/// mutates (deduplicated). This is what reactivity needs for the common pattern
/// of an event handler that *calls* a function which mutates state — e.g.
/// `@click="add(x)"` where `function add(){ items = … }`: the click depends on
/// `add`'s mutation set, not on any direct assignment in the handler text.
///
/// ```
/// use lunas_script::function_mutations;
///
/// let muts = function_mutations(
///     "function add(){ items = items.concat(x); count++ }\nconst noop = () => 0"
/// ).unwrap();
/// assert_eq!(muts, vec![("add".to_string(), vec!["items".to_string(), "count".to_string()]),
///                       ("noop".to_string(), vec![])]);
/// ```
pub fn function_mutations(code: &str) -> Result<Vec<(String, Vec<String>)>, ScriptParseError> {
    Ok(collect_function_mutations(&parse_program(code)?))
}

fn collect_function_mutations(module: &swc_ecma_ast::Module) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for item in &module.body {
        let decl = match item {
            ModuleItem::Stmt(Stmt::Decl(d)) => d,
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(e)) => &e.decl,
            _ => continue,
        };
        match decl {
            Decl::Fn(f) => {
                let mut c = AssignCollector { names: Vec::new() };
                f.function.visit_with(&mut c);
                out.push((f.ident.sym.to_string(), dedup(c.names)));
            }
            Decl::Var(var) => {
                for d in &var.decls {
                    if let (Pat::Ident(name), Some(init)) = (&d.name, &d.init) {
                        if is_callable(init) {
                            let mut c = AssignCollector { names: Vec::new() };
                            init.visit_with(&mut c);
                            out.push((name.id.sym.to_string(), dedup(c.names)));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// A whole-`script:`-block analysis computed in a single parse: the names the
/// script declares and, for each top-level function, what it mutates. This is
/// the per-component analysis the orchestrator runs once (rather than parsing
/// the script twice via [`declared_bindings`] + [`function_mutations`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptAnalysis {
    pub bindings: Vec<String>,
    pub function_mutations: Vec<(String, Vec<String>)>,
}

/// Analyzes a `script:` block in one parse. See [`ScriptAnalysis`].
///
/// ```
/// use lunas_script::analyze_script;
///
/// let a = analyze_script("let n = 0\nfunction inc(){ n++ }").unwrap();
/// assert_eq!(a.bindings, ["n", "inc"]);
/// assert_eq!(a.function_mutations, vec![("inc".to_string(), vec!["n".to_string()])]);
/// ```
pub fn analyze_script(code: &str) -> Result<ScriptAnalysis, ScriptParseError> {
    let module = parse_program(code)?;
    Ok(ScriptAnalysis {
        bindings: collect_bindings(&module),
        function_mutations: collect_function_mutations(&module),
    })
}

fn is_callable(expr: &Expr) -> bool {
    matches!(expr, Expr::Arrow(_) | Expr::Fn(_))
}

fn dedup(names: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    names
        .into_iter()
        .filter(|n| seen.insert(n.clone()))
        .collect()
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
