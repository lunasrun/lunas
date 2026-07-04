//! The JS emitter pass: turns a [`crate::ResolvedComponent`] into a runnable ES
//! module that targets the runtime in `packages/lunas` (see
//! `docs/output-design.md`).
//!
//! Scope (Wave 1): plain text binds, attribute binds (incl. interpolated static
//! attrs), and event listeners. Positional refs and runtime text anchors are
//! emitted per §7–8. Control flow (`:if`/`:for`), two-way bindings, and child
//! components are **not** wired yet — they are voided with a `/* TODO */` marker
//! (and, for two-way, a diagnostic) so the module still compiles and runs.
//!
//! The reactive model is compile-time adjacency dispatch (§4): each reactive
//! variable becomes a box at a stable index; references to it are rewritten to
//! `.v`; each dynamic part `bind(c, deps, …)`s with its precomputed dep indices.

use lunas_parser::{Diagnostic, StaticValue, TemplateAttr, TemplateText, TextRange, TextSegment};
use lunas_script::module_binding_references;

use crate::codegen::skeleton::{
    build_skeleton, DynamicElement, InsertPos, Skeleton, Slot, SlotContent,
};
use crate::model::{DynamicKind, ReactiveVar, ResolvedComponent};

/// The root wrapper tag. The skeleton HTML is the wrapper's `innerHTML`, so all
/// positional [`refs`](../../packages/lunas/src/dom.mjs) paths are `childNodes`
/// indices from this element. A neutral `<div>` is used for every component in
/// Wave 1 (single-root / multi-root distinction is a later optimization).
const ROOT_TAG: &str = "div";

/// Compiles a `.lunas` source string into a runnable ES module.
///
/// Never panics. Returns `None` for the module only when there is nothing to
/// emit (no template) or a resolve error prevents emission; diagnostics carry
/// the detail. Unsupported constructs inside a template are voided gracefully,
/// not fatal.
pub fn compile(source: &str) -> (Option<String>, Vec<Diagnostic>) {
    let (component, mut diags) = crate::resolve(source);
    if diags.iter().any(|d| d.is_error()) {
        return (None, diags);
    }
    match emit_module(&component, &mut diags) {
        Some(js) => (Some(js), diags),
        None => (None, diags),
    }
}

/// Emits the module for a resolved component, or `None` if it has no template.
fn emit_module(component: &ResolvedComponent, diags: &mut Vec<Diagnostic>) -> Option<String> {
    let template = component.template.as_ref()?;
    let skeleton = build_skeleton(template);

    let mut e = Emitter::new(component);
    let setup_body = e.setup_body(&skeleton, diags);

    let mut out = String::new();
    out.push_str(&e.import_line());
    out.push('\n');
    out.push('\n');
    out.push_str("const HTML = ");
    push_js_string(&mut out, &skeleton.html);
    out.push_str(";\n\n");
    out.push_str("export default component(");
    push_js_string(&mut out, ROOT_TAG);
    out.push_str(", {}, HTML, (c, props) => {\n");
    out.push_str(&setup_body);
    out.push_str("});\n");
    Some(out)
}

struct Emitter<'a> {
    component: &'a ResolvedComponent,
    /// Runtime helper names referenced by the emitted module, collected while
    /// generating so the import line stays minimal.
    used: std::collections::BTreeSet<&'static str>,
}

impl<'a> Emitter<'a> {
    fn new(component: &'a ResolvedComponent) -> Self {
        Emitter {
            component,
            used: std::collections::BTreeSet::new(),
        }
    }

    /// `import { … } from "lunas";` covering every helper actually referenced.
    fn import_line(&self) -> String {
        // `component` is always used (the module's default export).
        let mut names: Vec<&str> = self.used.iter().copied().collect();
        if !names.contains(&"component") {
            names.insert(0, "component");
        }
        format!("import {{ {} }} from \"lunas\";", names.join(", "))
    }

    fn use_helper(&mut self, name: &'static str) {
        self.used.insert(name);
    }

    /// The body of the `setup` closure: props, boxes, refs, anchors, binds and
    /// event listeners, in the build order of §7.
    fn setup_body(&mut self, skeleton: &Skeleton, diags: &mut Vec<Diagnostic>) -> String {
        let mut b = String::new();

        self.emit_props(&mut b);
        self.emit_boxes(&mut b);
        self.emit_refs(&mut b, &skeleton.dynamic_elements);
        self.emit_text_slots(&mut b, &skeleton.slots, diags);
        self.emit_attr_and_event_wiring(&mut b, &skeleton.dynamic_elements, diags);

        b
    }

    // --- props ------------------------------------------------------------

    fn emit_props(&mut self, b: &mut String) {
        for p in &self.component.props {
            // `@input name = default` -> `let name = props.name ?? default;`
            // Reactive props are still boxed below; the plain `let` seeds them.
            b.push_str("  let ");
            b.push_str(&p.name);
            b.push_str(" = props.");
            b.push_str(&p.name);
            if let Some(d) = &p.default_value {
                b.push_str(" ?? (");
                b.push_str(d.trim());
                b.push(')');
            }
            b.push_str(";\n");
        }
    }

    // --- boxes ------------------------------------------------------------

    fn emit_boxes(&mut self, b: &mut String) {
        if self.component.reactive_vars.is_empty() {
            return;
        }
        let script_text = self
            .component
            .script
            .as_ref()
            .map(|s| s.source.text.as_str())
            .unwrap_or("");
        // Rewrite the script body so reactive declarations become boxes and all
        // references become `.v`.
        let rewritten = rewrite_script(script_text, &self.component.reactive_vars);
        let body = dedent(&rewritten);
        let body = body.trim();
        if !body.is_empty() {
            for (kind, _) in reactive_box_kinds(script_text, &self.component.reactive_vars) {
                self.use_helper(kind);
            }
            for line in body.lines() {
                b.push_str("  ");
                b.push_str(line);
                b.push('\n');
            }
        }
    }

    // --- refs -------------------------------------------------------------

    fn emit_refs(&mut self, b: &mut String, elems: &[DynamicElement]) {
        if elems.is_empty() {
            return;
        }
        self.use_helper("refs");
        b.push_str("  const [");
        for (i, _) in elems.iter().enumerate() {
            if i > 0 {
                b.push_str(", ");
            }
            b.push_str(&ref_name(i));
        }
        b.push_str("] = refs(c.root, [");
        for (i, el) in elems.iter().enumerate() {
            if i > 0 {
                b.push_str(", ");
            }
            push_path(b, &el.path);
        }
        b.push_str("]);\n");
    }

    // --- text slots -> anchors + binds -----------------------------------

    fn emit_text_slots(&mut self, b: &mut String, slots: &[Slot], diags: &mut Vec<Diagnostic>) {
        for (i, slot) in slots.iter().enumerate() {
            match &slot.content {
                SlotContent::Text(t) => self.emit_text_slot(b, i, t, &slot.pos),
                SlotContent::If(_) | SlotContent::For(_) | SlotContent::Component(_) => {
                    b.push_str("  /* TODO(wave2): ");
                    b.push_str(slot_kind_name(&slot.content));
                    b.push_str(" block not yet emitted */\n");
                    let _ = diags; // no diagnostic: these are known deferrals
                }
            }
        }
    }

    fn emit_text_slot(&mut self, b: &mut String, i: usize, t: &TemplateText, pos: &InsertPos) {
        let anchor = format!("t{i}");
        self.emit_anchor(b, &anchor, pos);

        let deps = self.text_run_deps(t);
        let expr = self.text_run_expr(t);

        if deps.is_empty() {
            // Static-once: assign at build, no bind.
            b.push_str("  ");
            b.push_str(&anchor);
            b.push_str(".data = ");
            b.push_str(&expr);
            b.push_str(";\n");
        } else {
            self.use_helper("bind");
            b.push_str("  bind(c, ");
            push_dep_list(b, &deps);
            b.push_str(", () => { ");
            b.push_str(&anchor);
            b.push_str(".data = ");
            b.push_str(&expr);
            b.push_str("; });\n");
        }
    }

    /// Emits the anchor-creation statement for a text run, binding it to a
    /// local const named `name`.
    fn emit_anchor(&mut self, b: &mut String, name: &str, pos: &InsertPos) {
        match pos {
            InsertPos::Before(path) => {
                self.use_helper("anchorBefore");
                b.push_str("  const ");
                b.push_str(name);
                b.push_str(" = anchorBefore(");
                push_node_at(b, path);
                b.push_str(");\n");
            }
            InsertPos::BeforeSplit { path, utf16_offset } => {
                self.use_helper("anchorBeforeSplit");
                b.push_str("  const ");
                b.push_str(name);
                b.push_str(" = anchorBeforeSplit(");
                push_node_at(b, path);
                b.push_str(", ");
                b.push_str(&utf16_offset.to_string());
                b.push_str(");\n");
            }
            InsertPos::Append(path) => {
                self.use_helper("anchorAppend");
                b.push_str("  const ");
                b.push_str(name);
                b.push_str(" = anchorAppend(");
                push_node_at(b, path);
                b.push_str(");\n");
            }
        }
    }

    /// The combined dependency indices of every interpolation in a text run.
    fn text_run_deps(&self, t: &TemplateText) -> Vec<u32> {
        let mut acc = Vec::new();
        for seg in &t.segments {
            if let TextSegment::Interpolation(interp) = seg {
                if let Some(part) =
                    self.find_dynamic(DynamicKind::Text, &interp.expr, interp.expr_range)
                {
                    acc.extend(part.deps.indices().iter().copied());
                }
            }
        }
        acc.sort_unstable();
        acc.dedup();
        acc
    }

    /// A JS template literal that reproduces the whole text run, with reactive
    /// references rewritten to `.v`.
    fn text_run_expr(&self, t: &TemplateText) -> String {
        let mut out = String::from("`");
        for seg in &t.segments {
            match seg {
                TextSegment::Literal { text, .. } => push_template_literal_chunk(&mut out, text),
                TextSegment::Interpolation(interp) => {
                    out.push_str("${");
                    out.push_str(&rewrite_expr(&interp.expr, &self.component.reactive_vars));
                    out.push('}');
                }
            }
        }
        out.push('`');
        out
    }

    // --- attribute / event wiring ----------------------------------------

    fn emit_attr_and_event_wiring(
        &mut self,
        b: &mut String,
        elems: &[DynamicElement],
        diags: &mut Vec<Diagnostic>,
    ) {
        for (i, el) in elems.iter().enumerate() {
            let name = ref_name(i);
            for attr in &el.attrs {
                match attr {
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } => {
                        self.emit_bound_attr(b, &name, attr_name, &expr.text);
                    }
                    TemplateAttr::Static {
                        name: attr_name,
                        value: Some(v),
                        ..
                    } if has_interpolation(v) => {
                        self.emit_attr_text(b, &name, attr_name, v);
                    }
                    TemplateAttr::Event { event, handler, .. } => {
                        self.emit_event(b, &name, event, &handler.text);
                    }
                    TemplateAttr::TwoWay {
                        name: attr_name,
                        range,
                        ..
                    } => {
                        b.push_str("  /* TODO(wave2): two-way ::");
                        b.push_str(attr_name);
                        b.push_str(" not yet emitted */\n");
                        diags.push(Diagnostic::warning(
                            *range,
                            format!("two-way binding ::{attr_name} is not yet emitted (Wave 2)"),
                        ));
                    }
                    TemplateAttr::Static { .. } => {}
                }
            }
        }
    }

    fn emit_bound_attr(&mut self, b: &mut String, node: &str, attr: &str, expr: &str) {
        let deps = self.bound_attr_deps(attr, expr);
        let value = rewrite_expr(expr, &self.component.reactive_vars);
        let set = attr_set_statement(node, attr, &value);
        if deps.is_empty() {
            b.push_str("  ");
            b.push_str(&set);
            b.push('\n');
        } else {
            self.use_helper("bind");
            b.push_str("  bind(c, ");
            push_dep_list(b, &deps);
            b.push_str(", () => { ");
            b.push_str(&set);
            b.push_str(" });\n");
        }
    }

    fn emit_attr_text(&mut self, b: &mut String, node: &str, attr: &str, value: &StaticValue) {
        let mut deps = Vec::new();
        let mut lit = String::from("`");
        for seg in &value.segments {
            match seg {
                TextSegment::Literal { text, .. } => push_template_literal_chunk(&mut lit, text),
                TextSegment::Interpolation(interp) => {
                    lit.push_str("${");
                    lit.push_str(&rewrite_expr(&interp.expr, &self.component.reactive_vars));
                    lit.push('}');
                    if let Some(part) = self.find_dynamic(
                        DynamicKind::AttributeText(attr.to_string()),
                        &interp.expr,
                        interp.expr_range,
                    ) {
                        deps.extend(part.deps.indices().iter().copied());
                    }
                }
            }
        }
        lit.push('`');
        deps.sort_unstable();
        deps.dedup();

        let set = attr_set_statement(node, attr, &lit);
        if deps.is_empty() {
            b.push_str("  ");
            b.push_str(&set);
            b.push('\n');
        } else {
            self.use_helper("bind");
            b.push_str("  bind(c, ");
            push_dep_list(b, &deps);
            b.push_str(", () => { ");
            b.push_str(&set);
            b.push_str(" });\n");
        }
    }

    fn emit_event(&mut self, b: &mut String, node: &str, event: &str, handler: &str) {
        self.use_helper("on");
        let body = rewrite_expr(handler, &self.component.reactive_vars);
        b.push_str("  on(");
        b.push_str(node);
        b.push_str(", ");
        push_js_string(b, event);
        b.push_str(", () => { ");
        b.push_str(&body);
        b.push_str("; });\n");
    }

    // --- dep lookup -------------------------------------------------------

    fn bound_attr_deps(&self, attr: &str, expr: &str) -> Vec<u32> {
        self.find_dynamic_by(|p| {
            matches!(&p.kind, DynamicKind::Attribute(n) if n == attr) && p.expr == expr
        })
        .map(|p| p.deps.indices().to_vec())
        .unwrap_or_default()
    }

    fn find_dynamic(
        &self,
        kind: DynamicKind,
        expr: &str,
        range: TextRange,
    ) -> Option<&crate::model::DynamicPart> {
        self.component
            .dynamics
            .iter()
            .find(|p| p.kind == kind && p.expr == expr && p.range == range)
            .or_else(|| {
                self.component
                    .dynamics
                    .iter()
                    .find(|p| p.kind == kind && p.expr == expr)
            })
    }

    fn find_dynamic_by<F: Fn(&crate::model::DynamicPart) -> bool>(
        &self,
        pred: F,
    ) -> Option<&crate::model::DynamicPart> {
        self.component.dynamics.iter().find(|p| pred(p))
    }
}

// --- reactive box selection & script rewriting ---------------------------

/// The box helper each reactive var should use, in declaration order: `deepBox`
/// if the script deeply mutates the var (member/index write or a mutating array
/// method), else `box`.
fn reactive_box_kinds(script_text: &str, vars: &[ReactiveVar]) -> Vec<(&'static str, u32)> {
    vars.iter()
        .map(|v| {
            let kind = if is_deeply_mutated(script_text, &v.name) {
                "deepBox"
            } else {
                "box"
            };
            (kind, v.index)
        })
        .collect()
}

/// Textual heuristic for deep mutation of `name`: a member/index assignment
/// (`name.x =`, `name[i] =`, `name.x++`) or a mutating array method call
/// (`name.push(`, …). Conservative and dependency-free — Wave 1 has no
/// AST-level deep-mutation analysis. On a false negative the var would use a
/// plain `box`, which still reacts to whole reassignment.
fn is_deeply_mutated(script: &str, name: &str) -> bool {
    const MUTATORS: &[&str] = &[
        "push",
        "pop",
        "shift",
        "unshift",
        "splice",
        "sort",
        "reverse",
        "fill",
        "copyWithin",
        "set",
        "add",
        "delete",
        "clear",
    ];
    for (start, _) in match_identifier_occurrences(script, name) {
        let after = &script[start + name.len()..];
        let after_trim = after.trim_start();
        if let Some(rest) = after_trim.strip_prefix('.') {
            let rest = rest.trim_start();
            // name.method( … )  — mutating method call
            if MUTATORS.iter().any(|m| {
                rest.strip_prefix(m)
                    .map(|r| r.trim_start().starts_with('('))
                    .unwrap_or(false)
            }) {
                return true;
            }
            // name.prop = / += / ++
            if let Some(prop_rest) = skip_ident(rest) {
                let p = prop_rest.trim_start();
                if p.starts_with('=') && !p.starts_with("==")
                    || p.starts_with("++")
                    || p.starts_with("--")
                    || p.starts_with("+=")
                    || p.starts_with("-=")
                    || p.starts_with("*=")
                    || p.starts_with("/=")
                {
                    return true;
                }
            }
        } else if after_trim.starts_with('[') {
            // name[…] = …   — assume index assignment implies deep mutation
            return true;
        }
    }
    false
}

/// Byte offsets where `name` occurs in `script` as a standalone identifier
/// (not part of a longer identifier). Cheap textual scan — good enough for the
/// deep-mutation heuristic.
fn match_identifier_occurrences(script: &str, name: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if name.is_empty() {
        return out;
    }
    let bytes = script.as_bytes();
    let mut i = 0;
    while i <= script.len() {
        let Some(off) = script[i..].find(name) else {
            break;
        };
        let start = i + off;
        let end = start + name.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            out.push((start, end));
        }
        // Advance past this match (matches are ASCII-name-length; `end` is a
        // valid boundary because `name` is a substring ending at `end`).
        i = end;
    }
    out
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Skips a leading identifier, returning the remainder, or `None` if there is
/// no identifier at the front.
fn skip_ident(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_' || bytes[0] == b'$')
    {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && is_ident_byte(bytes[i]) {
        i += 1;
    }
    Some(&s[i..])
}

/// Rewrites the whole script body: reactive `let/const/var name = init`
/// declarations become `const name = box|deepBox(c, i, init)`, and every
/// reference to a reactive variable becomes `name.v`. Uses scope-aware
/// [`module_binding_references`] so shadowed uses are left alone.
fn rewrite_script(script: &str, vars: &[ReactiveVar]) -> String {
    if vars.is_empty() {
        return script.to_string();
    }
    let reactive: std::collections::HashMap<&str, &ReactiveVar> =
        vars.iter().map(|v| (v.name.as_str(), v)).collect();

    let decls = lunas_script::top_level_declarations(script).unwrap_or_default();
    let refs = module_binding_references(script).unwrap_or_default();

    // Edits over the original text: (start, end, replacement). Applied
    // right-to-left so byte offsets stay valid; they must not overlap.
    let mut edits: Vec<(u32, u32, String)> = Vec::new();

    // (1) Reactive declarations become box constructors. Only single-declarator
    // statements are rewritten (the common case); a reactive var in a multi-
    // declarator statement is left as a plain `let` and still reacts to whole
    // reassignment via `.v` reads (it just won't be boxed). The declarator init
    // is itself rewritten so reactive reads inside it get `.v`.
    let mut decl_ranges: Vec<TextRange> = Vec::new();
    for d in &decls {
        let Some(var) = reactive.get(d.name.as_str()) else {
            continue;
        };
        if d.declarators_in_stmt != 1 {
            continue;
        }
        let kind = if is_deeply_mutated(script, &d.name) {
            "deepBox"
        } else {
            "box"
        };
        let init_raw = d
            .init_range
            .and_then(|ir| ir.slice(script))
            .unwrap_or("undefined");
        let init = rewrite_expr(init_raw, vars);
        let text = format!("const {} = {}(c, {}, {})", d.name, kind, var.index, init);
        edits.push((d.stmt_range.start().raw(), d.stmt_range.end().raw(), text));
        decl_ranges.push(d.stmt_range);
    }

    // (2) Every reactive reference becomes `name.v`. Skip references inside a
    // rewritten declaration statement — its init was already rewritten in (1).
    for r in &refs {
        if !reactive.contains_key(r.name.as_str()) {
            continue;
        }
        let end = r.range.end().raw();
        if decl_ranges
            .iter()
            .any(|dr| end > dr.start().raw() && end <= dr.end().raw())
        {
            continue;
        }
        if r.shorthand {
            // `{ count }` -> `{ count: count.v }`
            let start = r.range.start().raw();
            edits.push((start, end, format!("{}: {}.v", r.name, r.name)));
        } else {
            edits.push((end, end, ".v".to_string()));
        }
    }

    apply_edits(script, edits)
}

/// Applies `(start, end, replacement)` edits to `src`, right-to-left. Edits
/// must not overlap (aside from zero-width inserts, which are fine).
fn apply_edits(src: &str, mut edits: Vec<(u32, u32, String)>) -> String {
    edits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut out = src.to_string();
    for (start, end, text) in edits {
        let (s, e) = (start as usize, end as usize);
        if s <= e && e <= out.len() && out.is_char_boundary(s) && out.is_char_boundary(e) {
            out.replace_range(s..e, &text);
        }
    }
    out
}

/// Rewrites a standalone JS expression so reactive references become `name.v`.
/// Uses scope-aware [`free_identifiers_with_spans`] so shadowed locals (e.g. an
/// arrow parameter of the same name) are left alone.
fn rewrite_expr(expr: &str, vars: &[ReactiveVar]) -> String {
    if vars.is_empty() {
        return expr.trim().to_string();
    }
    let reactive: std::collections::HashSet<&str> = vars.iter().map(|v| v.name.as_str()).collect();
    let spans = match lunas_script::free_identifiers_with_spans(expr) {
        Ok(s) => s,
        Err(_) => return expr.trim().to_string(),
    };
    let mut edits: Vec<(u32, u32, String)> = Vec::new();
    for (name, range) in spans {
        if reactive.contains(name.as_str()) {
            edits.push((range.end().raw(), range.end().raw(), ".v".to_string()));
        }
    }
    apply_edits(expr, edits).trim().to_string()
}

// --- attribute set special cases -----------------------------------------

/// The statement that assigns `value` (a JS expression) to `attr` on `node`.
/// Boolean and property attributes get direct property assignment; everything
/// else goes through `setAttribute`.
fn attr_set_statement(node: &str, attr: &str, value: &str) -> String {
    if let Some(prop) = boolean_property(attr) {
        // Boolean attribute: reflect truthiness onto the DOM property.
        format!("{node}.{prop} = !!({value});")
    } else if let Some(prop) = idl_property(attr) {
        format!("{node}.{prop} = {value};")
    } else {
        format!("{node}.setAttribute(\"{attr}\", {value});")
    }
}

/// Attributes best set as a boolean DOM property.
fn boolean_property(attr: &str) -> Option<&'static str> {
    match attr {
        "checked" => Some("checked"),
        "disabled" => Some("disabled"),
        "selected" => Some("selected"),
        "readonly" => Some("readOnly"),
        "multiple" => Some("multiple"),
        "hidden" => Some("hidden"),
        _ => None,
    }
}

/// Attributes best set as an IDL property (value type / reflection quirks).
fn idl_property(attr: &str) -> Option<&'static str> {
    match attr {
        "value" => Some("value"),
        _ => None,
    }
}

// --- small emit utilities ------------------------------------------------

fn ref_name(i: usize) -> String {
    format!("e{i}")
}

/// Strips the common leading whitespace shared by all non-blank lines, so a
/// script block written with source indentation re-emits cleanly.
fn dedent(s: &str) -> String {
    let min_indent = s
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    s.lines()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn slot_kind_name(c: &SlotContent) -> &'static str {
    match c {
        SlotContent::Text(_) => "text",
        SlotContent::If(_) => "if",
        SlotContent::For(_) => "for",
        SlotContent::Component(_) => "component",
    }
}

fn has_interpolation(v: &StaticValue) -> bool {
    v.segments
        .iter()
        .any(|s| matches!(s, TextSegment::Interpolation(_)))
}

/// Emits `n.childNodes[i]…` navigation from `c.root` for a positional path.
/// `[]` is the root itself.
fn push_node_at(b: &mut String, path: &[u32]) {
    b.push_str("c.root");
    for i in path {
        b.push_str(".childNodes[");
        b.push_str(&i.to_string());
        b.push(']');
    }
}

/// Emits a positional path as a JS array literal, e.g. `[0, 1]`.
fn push_path(b: &mut String, path: &[u32]) {
    b.push('[');
    for (i, p) in path.iter().enumerate() {
        if i > 0 {
            b.push_str(", ");
        }
        b.push_str(&p.to_string());
    }
    b.push(']');
}

/// Emits a dependency-index list as a JS array literal, e.g. `[0, 2]`.
fn push_dep_list(b: &mut String, deps: &[u32]) {
    b.push('[');
    for (i, d) in deps.iter().enumerate() {
        if i > 0 {
            b.push_str(", ");
        }
        b.push_str(&d.to_string());
    }
    b.push(']');
}

/// Emits `s` as a double-quoted JS string literal.
fn push_js_string(b: &mut String, s: &str) {
    b.push('"');
    for ch in s.chars() {
        match ch {
            '"' => b.push_str("\\\""),
            '\\' => b.push_str("\\\\"),
            '\n' => b.push_str("\\n"),
            '\r' => b.push_str("\\r"),
            '\t' => b.push_str("\\t"),
            c => b.push(c),
        }
    }
    b.push('"');
}

/// Pushes a literal chunk into a JS template literal, escaping backtick, `${`,
/// and backslash so the literal text round-trips.
fn push_template_literal_chunk(b: &mut String, s: &str) {
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' => b.push_str("\\`"),
            '\\' => b.push_str("\\\\"),
            '$' if chars.peek() == Some(&'{') => b.push_str("\\$"),
            c => b.push(c),
        }
    }
}
