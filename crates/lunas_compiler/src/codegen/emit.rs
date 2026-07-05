//! The JS emitter pass: turns a [`crate::ResolvedComponent`] into a runnable ES
//! module that targets the runtime in `packages/lunas` (see
//! `docs/output-design.md` and `docs/for-diff-design.md`).
//!
//! Scope (Wave 2): text binds, attribute binds, event listeners, two-way
//! bindings (`::name`), `:if`/`:elseif`/`:else` cascades (`ifBlock`/`ifChain`),
//! and keyed `:for` (`forBlock` in compiled html/wire mode: bulk `innerHTML`
//! initial render + keyed diff updates). Control flow is emitted recursively,
//! so nested combinations (`:if` in `:for`, `:for` in `:if`, …) work; each
//! branch/item is its own *fragment* with its own static HTML string (hoisted
//! at module scope), positional refs, anchors, and binds.
//!
//! Child components, `:for`+`:if` on the same element, and `:for` over a
//! non-element body are not wired yet — they are voided with a comment (and a
//! warning diagnostic where the author would otherwise be surprised) so the
//! module still compiles and runs.
//!
//! The reactive model is compile-time adjacency dispatch (§4): each reactive
//! variable becomes a box at a stable index; references to it are rewritten to
//! `.v`; each dynamic part `bind(c, deps, …)`s with its precomputed dep
//! indices. Inside a `:for` item, expressions that read the loop bindings are
//! registered as `bind(c, [], …)` so the item's patch path (`runScope`) can
//! refresh them when the item's data cell changes.

use lunas_parser::{
    ComponentUse, Diagnostic, ForBlock, IfChain, StaticValue, Template, TemplateAttr,
    TemplateElement, TemplateNode, TemplateText, TextRange, TextSegment,
};
use lunas_script::{module_binding_references, parse_for, ForKind};

use crate::codegen::skeleton::{
    build_skeleton, DynamicElement, InsertPos, Skeleton, Slot, SlotContent,
};
use crate::model::{DynamicKind, ReactiveVar, ResolvedComponent};

/// The root wrapper tag. The skeleton HTML is the wrapper's `innerHTML`, so all
/// positional [`refs`](../../packages/lunas/src/dom.mjs) paths are `childNodes`
/// indices from this element. A neutral `<div>` is used for every component
/// (single-root / multi-root distinction is a later optimization).
const ROOT_TAG: &str = "div";

/// Hard cap on control-flow nesting; beyond it blocks are voided with a
/// diagnostic instead of recursing (never-panic guarantee).
const MAX_BLOCK_DEPTH: u32 = 32;

/// Every runtime helper the emitter can reference by a bare name. Used to
/// detect collisions with user bindings (a reactive var named `on` would
/// shadow the `on` helper); a colliding helper is imported under an alias.
const ALL_HELPERS: &[&str] = &[
    "refs",
    "on",
    "bind",
    "box",
    "deepBox",
    "prop",
    "anchorBefore",
    "anchorBeforeSplit",
    "anchorAppend",
    "ifBlock",
    "ifChain",
    "forBlock",
    "fromHTML",
    "mountChild",
    "dynamicBlock",
    "teleportBlock",
    "setClass",
    "setStyle",
    // `component` is intentionally excluded: it is only referenced at module
    // scope (the default export), where user bindings — emitted inside the
    // setup closure — cannot shadow it.
];

/// A collision-proof local alias for a runtime helper whose canonical name is
/// taken by a user binding. Prefixes with `$` (rare in hand-written state) and
/// appends underscores until the name is unused.
fn alias_name(helper: &str, reserved: &std::collections::HashSet<String>) -> String {
    let mut name = format!("${helper}");
    while reserved.contains(&name) {
        name.push('_');
    }
    name
}

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
    warn_html_with_children(template, diags);
    let skeleton = build_skeleton(template);

    let mut e = Emitter::new(component);
    let setup_body = e.setup_body(&skeleton, diags);
    let multi_root = is_multi_root(template);

    let mut out = String::new();
    out.push_str(&e.import_line(multi_root));
    out.push('\n');
    // Child-component imports from the `@use` table (path as written). Emitted
    // in tag-name order after the runtime import, before the module body.
    for (local, path) in e.child_imports.values() {
        out.push_str("import ");
        out.push_str(local);
        out.push_str(" from ");
        push_js_string(&mut out, path);
        out.push_str(";\n");
    }
    out.push('\n');
    out.push_str("const HTML = ");
    push_js_string(&mut out, &skeleton.html);
    out.push_str(";\n");
    // Hoisted branch/item skeletons (one per :if branch / :for item template),
    // defined once per module like HTML itself.
    for (name, html) in &e.hoisted {
        out.push_str("const ");
        out.push_str(name);
        out.push_str(" = ");
        push_js_string(&mut out, html);
        out.push_str(";\n");
    }
    out.push('\n');
    // Multi-root components (output-design.md §7): a template whose top level has
    // more than one rendered node compiles WITHOUT the wrapper element, via the
    // `fragment(...)` factory. It parses the same HTML into a throwaway host,
    // wires against it (positional refs still navigate `c.root` = the host), and
    // returns the child-node group as the mountable unit. Single-root stays on
    // the cheap `component(...)` path.
    if multi_root {
        out.push_str("export default fragment({}, HTML, (c, props) => {\n");
    } else {
        out.push_str("export default component(");
        push_js_string(&mut out, ROOT_TAG);
        out.push_str(", {}, HTML, (c, props) => {\n");
    }
    out.push_str(&setup_body);
    out.push_str("});\n");
    Some(out)
}

/// Whether the template has more than one rendered top-level node (elements,
/// components, text with content, `:if`/`:for`/`<component>`/`<teleport>`).
/// Insignificant whitespace and comments do not count. Such a template has no
/// single wrapper and compiles as a multi-root fragment (§7).
fn is_multi_root(template: &Template) -> bool {
    let mut count = 0usize;
    for node in &template.nodes {
        let counts = match node {
            TemplateNode::Text(t) => !t.is_whitespace(),
            TemplateNode::Comment(_) => false,
            _ => true,
        };
        if counts {
            count += 1;
            if count > 1 {
                return true;
            }
        }
    }
    false
}

/// The lexical context of one emitted fragment (the component root, an `:if`
/// branch, or a `:for` item).
struct Frag<'x> {
    /// JS expression for the fragment's root node (`c.root`, or a scratch /
    /// item root local like `r0`).
    base: &'x str,
    /// Loop-binding names in scope: they shadow same-named reactive variables
    /// (no `.v` rewrite, no dep registration) and expressions reading them are
    /// re-run by the enclosing item's patch path.
    shadowed: &'x [String],
    /// Indentation level (1 = the setup body).
    indent: usize,
    /// Control-flow nesting depth (for the recursion cap).
    depth: u32,
    /// How many leading path segments to strip: `:for` item fragments receive
    /// the item ROOT node (not a scratch container), so the skeleton's `[0, …]`
    /// paths become `[…]` relative to it.
    strip: usize,
}

/// A reactive prop passed to a child/dynamic component: an initial getter plus
/// a parent-side driving bind keyed on `deps` (or item-coupled for `:for`).
struct ReactiveProp {
    name: String,
    expr: String,
    deps: Vec<u32>,
    /// Reads an enclosing `:for` loop binding: no reactive deps of its own, but
    /// the item's patch path (`runScope`) must re-run its driving bind.
    coupled: bool,
}

struct Emitter<'a> {
    component: &'a ResolvedComponent,
    /// Runtime helper names referenced by the emitted module, collected while
    /// generating so the import line stays minimal.
    used: std::collections::BTreeSet<&'static str>,
    /// Local aliases for runtime helpers whose canonical name collides with a
    /// user binding (e.g. a reactive var named `on` would shadow the `on`
    /// helper). Maps the canonical helper name to the local name to import it
    /// as and reference. Empty in the common no-collision case.
    aliases: std::collections::BTreeMap<&'static str, String>,
    /// Hoisted static HTML strings for control-flow fragments: (const name,
    /// html), emitted at module scope in creation order.
    hoisted: Vec<(String, String)>,
    /// Child-component imports referenced in the template: component tag name
    /// (as written) -> (local import identifier, module path as written in
    /// `@use`). Emitted as `import Local from "path";` at module scope.
    child_imports: std::collections::BTreeMap<String, (String, String)>,
    /// Text used for deep-mutation detection: the script plus one synthetic
    /// assignment per two-way member/index lvalue (`::value="o.k"` deep-writes
    /// `o`, so `o` needs a `deepBox` even if the script never mutates it).
    deep_hint: String,
    n_ref: usize,    // e{n}
    n_text: usize,   // t{n}
    n_anchor: usize, // a{n}
    n_root: usize,   // r{n}
    n_data: usize,   // d{n}
    n_html: usize,   // HTML_{n}
    n_child: usize,  // c{n} (child-component instance handle)
}

impl<'a> Emitter<'a> {
    fn new(component: &'a ResolvedComponent) -> Self {
        let script_text = component
            .script
            .as_ref()
            .map(|s| s.source.text.as_str())
            .unwrap_or("");
        let mut deep_hint = String::from(script_text);
        if let Some(template) = &component.template {
            for lv in two_way_lvalues(template) {
                // Only member/index writes imply deep mutation; a plain
                // identifier lvalue is whole reassignment (a plain `box`).
                if lv.contains('.') || lv.contains('[') {
                    deep_hint.push('\n');
                    deep_hint.push_str(&lv);
                    deep_hint.push_str(" = 0;");
                }
            }
        }
        // Names that would shadow a runtime helper if imported bare: every
        // top-level script binding plus every prop. A helper whose canonical
        // name appears here is imported under a collision-proof alias instead.
        let mut reserved: std::collections::HashSet<String> = std::collections::HashSet::new();
        if let Some(script) = &component.script {
            if let Ok(names) = lunas_script::declared_bindings(&script.source.text) {
                reserved.extend(names);
            }
        }
        for p in &component.props {
            reserved.insert(p.name.clone());
        }
        let mut aliases = std::collections::BTreeMap::new();
        for &h in ALL_HELPERS {
            if reserved.contains(h) {
                aliases.insert(h, alias_name(h, &reserved));
            }
        }

        // Child-component imports: only for `@use` entries whose tag is actually
        // used in the template (avoids dead imports). Each gets a collision-proof
        // local identifier — its own name unless that collides with a helper
        // import, a user binding, or a generated local.
        let used_component_tags = component
            .template
            .as_ref()
            .map(component_tags_in_template)
            .unwrap_or_default();
        let mut import_reserved: std::collections::HashSet<String> = reserved.clone();
        for a in aliases.values() {
            import_reserved.insert(a.clone());
        }
        for &h in ALL_HELPERS {
            import_reserved.insert(h.to_string());
        }
        import_reserved.insert("component".to_string());
        // A `<component :is="expr"/>` in the template can reference ANY `@use`
        // factory by name from script/expression, so the compiler cannot know
        // which are used: when a dynamic component is present, import every
        // `@use` entry. These imports keep their raw name (the `:is` expression
        // refers to them by it), so they are added to the reserved set first.
        let has_dynamic = component
            .template
            .as_ref()
            .is_some_and(template_has_dynamic_component);

        let mut child_imports = std::collections::BTreeMap::new();
        for u in &component.imports {
            let used = used_component_tags.contains(&u.component_name) || has_dynamic;
            if !used {
                continue;
            }
            if child_imports.contains_key(&u.component_name) {
                continue;
            }
            let local = child_local_name(&u.component_name, &import_reserved);
            import_reserved.insert(local.clone());
            child_imports.insert(u.component_name.clone(), (local, u.path.clone()));
        }

        Emitter {
            component,
            used: std::collections::BTreeSet::new(),
            aliases,
            hoisted: Vec::new(),
            child_imports,
            deep_hint,
            n_ref: 0,
            n_text: 0,
            n_anchor: 0,
            n_root: 0,
            n_data: 0,
            n_html: 0,
            n_child: 0,
        }
    }

    /// `import { … } from "lunas";` covering every helper actually referenced.
    /// A helper whose canonical name collides with a user binding is imported
    /// under its alias (`on as _on$`).
    fn import_line(&self, multi_root: bool) -> String {
        // The module's default export uses `fragment` for a multi-root
        // component, else `component`. Both are only referenced at module scope,
        // so user bindings inside the setup closure cannot shadow them.
        let default_factory = if multi_root { "fragment" } else { "component" };
        let mut names: Vec<&str> = self.used.iter().copied().collect();
        if !names.contains(&default_factory) {
            names.insert(0, default_factory);
        }
        let specs: Vec<String> = names
            .iter()
            .map(|&n| match self.aliases.get(n) {
                Some(alias) => format!("{n} as {alias}"),
                None => n.to_string(),
            })
            .collect();
        format!("import {{ {} }} from \"lunas\";", specs.join(", "))
    }

    fn use_helper(&mut self, name: &'static str) {
        self.used.insert(name);
    }

    /// The local name to *reference* a helper by — its alias if the canonical
    /// name collides with a user binding, else the canonical name.
    fn helper(&self, name: &'static str) -> &str {
        self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name)
    }

    // --- name allocation ----------------------------------------------------

    fn alloc_ref(&mut self) -> String {
        let n = self.n_ref;
        self.n_ref += 1;
        format!("e{n}")
    }
    fn alloc_text(&mut self) -> String {
        let n = self.n_text;
        self.n_text += 1;
        format!("t{n}")
    }
    fn alloc_anchor(&mut self) -> String {
        let n = self.n_anchor;
        self.n_anchor += 1;
        format!("a{n}")
    }
    fn alloc_root(&mut self) -> String {
        let n = self.n_root;
        self.n_root += 1;
        format!("r{n}")
    }
    fn alloc_data(&mut self) -> String {
        let n = self.n_data;
        self.n_data += 1;
        format!("d{n}")
    }
    fn alloc_child(&mut self) -> String {
        let n = self.n_child;
        self.n_child += 1;
        format!("ch{n}")
    }
    fn hoist_html(&mut self, html: String) -> String {
        self.n_html += 1;
        let name = format!("HTML_{}", self.n_html);
        self.hoisted.push((name.clone(), html));
        name
    }

    /// The body of the `setup` closure: props, boxes, refs, anchors, binds and
    /// event listeners, in the build order of §7.
    fn setup_body(&mut self, skeleton: &Skeleton, diags: &mut Vec<Diagnostic>) -> String {
        let mut b = String::new();

        self.emit_props(&mut b);
        self.emit_boxes(&mut b);
        let frag = Frag {
            base: "c.root",
            shadowed: &[],
            indent: 1,
            depth: 0,
            strip: 0,
        };
        self.emit_fragment(&mut b, skeleton, &frag, diags);

        b
    }

    // --- props ------------------------------------------------------------

    fn emit_props(&mut self, b: &mut String) {
        for p in &self.component.props {
            // Every `@input` prop is reactive (a parent can change it), so it is
            // adopted as a reactive box at its index via the `prop` helper and
            // referenced through `.v` everywhere. The parent seeds it (a getter
            // for a reactive prop, a plain value for a static prop) and drives
            // later changes via the mountChild handle's setProp — which writes
            // this box, re-running the child's own binds (output-design.md §6).
            let index = self
                .component
                .reactive_index(&p.name)
                .expect("every @input prop is numbered as a reactive var");
            let deep = is_deeply_mutated(&self.deep_hint, &p.name);
            self.use_helper("prop");
            let prop_fn = self.helper("prop").to_string();
            let name = self.rewrite_binding_name(&p.name);
            b.push_str("  const ");
            b.push_str(&name);
            b.push_str(" = ");
            b.push_str(&prop_fn);
            b.push_str("(c, ");
            push_js_string(b, &p.name);
            b.push_str(", ");
            b.push_str(&index.to_string());
            b.push_str(", props.");
            b.push_str(&p.name);
            b.push_str(", ");
            match &p.default_value {
                Some(d) => {
                    b.push('(');
                    b.push_str(&rewrite_expr(d.trim(), &self.component.reactive_vars, &[]));
                    b.push(')');
                }
                None => b.push_str("undefined"),
            }
            if deep {
                b.push_str(", true");
            }
            b.push_str(");\n");
        }
    }

    /// The local name a reactive binding is emitted under. A prop whose name
    /// collides with a runtime helper would, when boxed as `const name = …`,
    /// need no alias (only *helper* references are aliased); the binding keeps
    /// its own name. This exists as a single point in case that changes.
    fn rewrite_binding_name(&self, name: &str) -> String {
        name.to_string()
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
        let rewritten = rewrite_script(
            script_text,
            &self.deep_hint,
            &self.component.reactive_vars,
            &self.aliases,
        );
        let body = dedent(&rewritten);
        let body = body.trim();
        if !body.is_empty() {
            // Import a box helper only for reactive vars actually DECLARED in the
            // script (props are boxed by the `prop` helper, not here, so they
            // must not pull in an unused `box`/`deepBox` import).
            let declared: std::collections::HashSet<String> =
                lunas_script::declared_bindings(script_text)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
            for (kind, index) in reactive_box_kinds(&self.deep_hint, &self.component.reactive_vars)
            {
                if self
                    .component
                    .reactive_vars
                    .iter()
                    .any(|v| v.index == index && declared.contains(&v.name))
                {
                    self.use_helper(kind);
                }
            }
            for line in body.lines() {
                b.push_str("  ");
                b.push_str(line);
                b.push('\n');
            }
        }
    }

    // --- fragments ----------------------------------------------------------

    /// Emits one fragment: refs, slots (text anchors + binds, control-flow
    /// blocks — recursively), then attribute/event wiring, per §7 build order.
    fn emit_fragment(
        &mut self,
        b: &mut String,
        skeleton: &Skeleton,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        let ref_names = self.emit_refs(b, &skeleton.dynamic_elements, frag);
        self.emit_slots(b, &skeleton.slots, frag, diags);
        self.emit_attr_and_event_wiring(b, &skeleton.dynamic_elements, &ref_names, frag, diags);
    }

    // --- refs -------------------------------------------------------------

    /// Emits the positional-nav `refs(...)` destructuring for a fragment's
    /// dynamic elements and returns the allocated local names (parallel to
    /// `elems`).
    fn emit_refs(&mut self, b: &mut String, elems: &[DynamicElement], frag: &Frag) -> Vec<String> {
        if elems.is_empty() {
            return Vec::new();
        }
        self.use_helper("refs");
        let refs_fn = self.helper("refs").to_string();
        let names: Vec<String> = elems.iter().map(|_| self.alloc_ref()).collect();
        push_indent(b, frag.indent);
        b.push_str("const [");
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                b.push_str(", ");
            }
            b.push_str(name);
        }
        b.push_str("] = ");
        b.push_str(&refs_fn);
        b.push('(');
        b.push_str(frag.base);
        b.push_str(", [");
        for (i, el) in elems.iter().enumerate() {
            if i > 0 {
                b.push_str(", ");
            }
            push_path(b, strip_path(&el.path, frag.strip));
        }
        b.push_str("]);\n");
        names
    }

    // --- slots --------------------------------------------------------------

    fn emit_slots(
        &mut self,
        b: &mut String,
        slots: &[Slot],
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        for slot in slots {
            match &slot.content {
                SlotContent::Text(t) => self.emit_text_slot(b, t, &slot.pos, frag),
                SlotContent::If(chain) => self.emit_if_slot(b, chain, &slot.pos, frag, diags),
                SlotContent::For(block) => self.emit_for_slot(b, block, &slot.pos, frag, diags),
                SlotContent::Component(comp) => {
                    self.emit_component_slot(b, comp, &slot.pos, frag, diags)
                }
                SlotContent::Dynamic(el) => self.emit_dynamic_slot(b, el, &slot.pos, frag, diags),
                SlotContent::Teleport(el) => self.emit_teleport_slot(b, el, &slot.pos, frag, diags),
            }
        }
    }

    // --- text slots -> anchors + binds -----------------------------------

    fn emit_text_slot(&mut self, b: &mut String, t: &TemplateText, pos: &InsertPos, frag: &Frag) {
        let anchor = self.alloc_text();
        self.emit_anchor(b, &anchor, pos, frag);

        let deps = self.filter_deps(self.text_run_deps(t), frag.shadowed);
        let expr = self.text_run_expr(t, frag.shadowed);
        let coupled = t.segments.iter().any(|seg| match seg {
            TextSegment::Interpolation(i) => reads_any(&i.expr, frag.shadowed),
            TextSegment::Literal { .. } => false,
        });

        let stmt = format!("{anchor}.data = {expr};");
        self.emit_update(b, frag, &deps, coupled, &stmt);
    }

    /// Emits `stmt` in the mode its dependencies demand: reactive deps →
    /// `bind(c, deps, …)`; loop-data-coupled (inside a `:for` item) →
    /// `bind(c, [], …)` so the item's patch path re-runs it; otherwise a bare
    /// build-time statement.
    fn emit_update(
        &mut self,
        b: &mut String,
        frag: &Frag,
        deps: &[u32],
        item_coupled: bool,
        stmt: &str,
    ) {
        push_indent(b, frag.indent);
        if deps.is_empty() && !item_coupled {
            b.push_str(stmt);
            b.push('\n');
        } else {
            self.use_helper("bind");
            b.push_str(self.helper("bind"));
            b.push_str("(c, ");
            push_dep_list(b, deps);
            b.push_str(", () => { ");
            b.push_str(stmt);
            b.push_str(" });\n");
        }
    }

    /// Emits the anchor-creation statement for a slot, binding it to a local
    /// const named `name`.
    fn emit_anchor(&mut self, b: &mut String, name: &str, pos: &InsertPos, frag: &Frag) {
        push_indent(b, frag.indent);
        match pos {
            InsertPos::Before(path) => {
                self.use_helper("anchorBefore");
                b.push_str("const ");
                b.push_str(name);
                b.push_str(" = ");
                b.push_str(self.helper("anchorBefore"));
                b.push('(');
                push_node_at(b, frag.base, strip_path(path, frag.strip));
                b.push_str(");\n");
            }
            InsertPos::BeforeSplit { path, utf16_offset } => {
                self.use_helper("anchorBeforeSplit");
                b.push_str("const ");
                b.push_str(name);
                b.push_str(" = ");
                b.push_str(self.helper("anchorBeforeSplit"));
                b.push('(');
                push_node_at(b, frag.base, strip_path(path, frag.strip));
                b.push_str(", ");
                b.push_str(&utf16_offset.to_string());
                b.push_str(");\n");
            }
            InsertPos::Append(path) => {
                self.use_helper("anchorAppend");
                b.push_str("const ");
                b.push_str(name);
                b.push_str(" = ");
                b.push_str(self.helper("anchorAppend"));
                b.push('(');
                push_node_at(b, frag.base, strip_path(path, frag.strip));
                b.push_str(");\n");
            }
        }
    }

    // --- child components -------------------------------------------------

    /// Classifies a list of component/dynamic-component props into the initial
    /// props-object member strings and the reactive props that need a driving
    /// bind (output-design.md §6). Shared by `mountChild` and `dynamicBlock`
    /// emission. `range` is used for diagnostics on unsupported prop shapes.
    fn build_child_props(
        &mut self,
        props: &[TemplateAttr],
        frag: &Frag,
        range: TextRange,
        diags: &mut Vec<Diagnostic>,
    ) -> (Vec<String>, Vec<ReactiveProp>) {
        let mut members: Vec<String> = Vec::new();
        let mut reactive: Vec<ReactiveProp> = Vec::new();
        for attr in props {
            match attr {
                TemplateAttr::Bound { name, expr, .. } => {
                    let deps =
                        self.filter_deps(self.bound_attr_deps(name, &expr.text), frag.shadowed);
                    let value =
                        rewrite_expr(&expr.text, &self.component.reactive_vars, frag.shadowed);
                    members.push(format!("{}: () => ({})", prop_key(name), value));
                    let coupled = reads_any(&expr.text, frag.shadowed);
                    if !deps.is_empty() || coupled {
                        reactive.push(ReactiveProp {
                            name: name.clone(),
                            expr: value,
                            deps,
                            coupled,
                        });
                    }
                }
                TemplateAttr::Static {
                    name,
                    value: Some(v),
                    ..
                } if has_interpolation(v) => {
                    let (lit, deps, coupled) = self.attr_text_value(name, v, frag);
                    members.push(format!("{}: () => ({})", prop_key(name), lit));
                    if !deps.is_empty() || coupled {
                        reactive.push(ReactiveProp {
                            name: name.clone(),
                            expr: lit,
                            deps,
                            coupled,
                        });
                    }
                }
                TemplateAttr::Static { name, value, .. } => {
                    let val = match value {
                        Some(v) => {
                            let mut s = String::from("`");
                            for seg in &v.segments {
                                if let TextSegment::Literal { text, .. } = seg {
                                    push_template_literal_chunk(&mut s, text);
                                }
                            }
                            s.push('`');
                            s
                        }
                        None => "true".to_string(),
                    };
                    members.push(format!("{}: {}", prop_key(name), val));
                }
                TemplateAttr::TwoWay { name, .. } => {
                    diags.push(Diagnostic::warning(
                        range,
                        format!("two-way binding `::{name}` on a component is not supported yet"),
                    ));
                }
                TemplateAttr::Event { event, .. } => {
                    diags.push(Diagnostic::warning(
                        range,
                        format!("component event binding `@{event}` is not supported yet"),
                    ));
                }
            }
        }
        (members, reactive)
    }

    /// Emits a child-component mount at a text anchor (output-design.md §6):
    /// an anchor, `mountChild(c, anchor, Child, initialProps)`, and one parent
    /// `bind` per reactive prop that drives the child via `setProp`. Reactive
    /// props seed the child through getters in the initial object and stay live
    /// through the binds; static props pass as plain values. The two contexts
    /// are independent — pushing a prop marks the child dirty, never the parent.
    fn emit_component_slot(
        &mut self,
        b: &mut String,
        comp: &ComponentUse,
        pos: &InsertPos,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        // Resolve the child factory local. An unknown tag (not in the `@use`
        // table) is voided: nothing to mount.
        let Some((local, _)) = self.child_imports.get(&comp.name).cloned() else {
            self.void_block(
                b,
                frag,
                comp.open_tag_range,
                "child component tag has no matching `@use` import",
                diags,
            );
            return;
        };

        // A `:ref="name"` on a component exposes the mountChild handle to the
        // script (output-design.md §6). Pull it out of the props so it is not
        // passed as a child prop.
        let ref_target = comp.props.iter().find_map(|a| match a {
            TemplateAttr::Bound { name, expr, .. } if name == "ref" => Some(expr.text.clone()),
            _ => None,
        });
        let props: Vec<TemplateAttr> = comp
            .props
            .iter()
            .filter(|a| !matches!(a, TemplateAttr::Bound { name, .. } if name == "ref"))
            .cloned()
            .collect();

        // Classify props into an initial-object + driving binds.
        let (members, reactive) = self.build_child_props(&props, frag, comp.open_tag_range, diags);

        let anchor = self.alloc_anchor();
        self.emit_anchor(b, &anchor, pos, frag);

        let handle = self.alloc_child();
        self.use_helper("mountChild");
        push_indent(b, frag.indent);
        b.push_str("const ");
        b.push_str(&handle);
        b.push_str(" = ");
        b.push_str(self.helper("mountChild"));
        b.push_str("(c, ");
        b.push_str(&anchor);
        b.push_str(", ");
        b.push_str(&local);
        b.push_str(", {");
        for (i, m) in members.iter().enumerate() {
            if i > 0 {
                b.push(',');
            }
            b.push(' ');
            b.push_str(m);
        }
        if !members.is_empty() {
            b.push(' ');
        }
        b.push_str("});\n");

        // Component ref: assign the mount handle into the reactive box `name`.
        if let Some(target) = ref_target {
            self.emit_ref_assign(b, &handle, &target, frag, comp.open_tag_range, diags);
        }

        // Drive each reactive prop from the parent. The bind's initial run seeds
        // the same value (a box no-ops on equal), later parent changes flow in.
        for rp in &reactive {
            let stmt = format!("{handle}.setProp({}, {});", js_string(&rp.name), rp.expr);
            self.emit_update(b, frag, &rp.deps, rp.coupled, &stmt);
        }
    }

    /// Emits `name.v = <handle>;` for a `:ref="name"` on a component, validating
    /// that `name` is a reactive box (a plain `let name;` in script). Shared
    /// shape with the element-ref path.
    fn emit_ref_assign(
        &mut self,
        b: &mut String,
        handle: &str,
        target: &str,
        frag: &Frag,
        range: TextRange,
        diags: &mut Vec<Diagnostic>,
    ) {
        let target = target.trim();
        let is_reactive = self
            .component
            .reactive_vars
            .iter()
            .any(|v| v.name == target)
            && !frag.shadowed.iter().any(|s| s == target);
        if !is_reactive {
            self.void_block(
                b,
                frag,
                range,
                &format!(
                    "`:ref` target `{target}` is not a reactive variable \
                     (declare it with `let {target};` in script)"
                ),
                diags,
            );
            return;
        }
        push_indent(b, frag.indent);
        b.push_str(target);
        b.push_str(".v = ");
        b.push_str(handle);
        b.push_str(";\n");
    }

    /// `<component :is="expr" :p="e" static="x"/>` (output-design.md §6): a
    /// dynamic component. `dynamicBlock` remounts the child at the anchor when
    /// the `:is` factory changes, and forwards reactive props via `setProp`.
    fn emit_dynamic_slot(
        &mut self,
        b: &mut String,
        el: &TemplateElement,
        pos: &InsertPos,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        // Find the `:is` binding (the factory expression). Without it there is
        // nothing to mount.
        let is_expr = el.attrs.iter().find_map(|a| match a {
            TemplateAttr::Bound { name, expr, .. } if name == "is" => Some(expr.text.clone()),
            _ => None,
        });
        let Some(is_expr) = is_expr else {
            self.void_block(
                b,
                frag,
                el.open_tag_range,
                "`<component>` requires an `:is` binding (e.g. `:is=\"view\"`)",
                diags,
            );
            return;
        };

        // Classify the remaining props (everything but `:is`) into an initial
        // object + driving binds, reusing the child-prop machinery.
        let props: Vec<TemplateAttr> = el
            .attrs
            .iter()
            .filter(|a| !matches!(a, TemplateAttr::Bound { name, .. } if name == "is"))
            .cloned()
            .collect();
        let (members, reactive) = self.build_child_props(&props, frag, el.open_tag_range, diags);

        let anchor = self.alloc_anchor();
        self.emit_anchor(b, &anchor, pos, frag);

        let is_deps = self.filter_deps(self.bound_attr_deps("is", &is_expr), frag.shadowed);
        let is_value = rewrite_expr(&is_expr, &self.component.reactive_vars, frag.shadowed);
        let is_coupled = reads_any(&is_expr, frag.shadowed);
        let mut all_deps = is_deps.clone();
        all_deps.sort_unstable();
        all_deps.dedup();

        let handle = self.alloc_child();
        self.use_helper("dynamicBlock");
        push_indent(b, frag.indent);
        b.push_str("const ");
        b.push_str(&handle);
        b.push_str(" = ");
        b.push_str(self.helper("dynamicBlock"));
        b.push_str("(c, ");
        b.push_str(&anchor);
        b.push_str(", ");
        // When the :is expr is item-coupled but has no reactive deps, pass an
        // empty dep list; the enclosing item's patch path re-runs the block.
        push_dep_list(b, &all_deps);
        b.push_str(", () => (");
        b.push_str(&is_value);
        b.push_str("), {");
        for (i, m) in members.iter().enumerate() {
            if i > 0 {
                b.push(',');
            }
            b.push(' ');
            b.push_str(m);
        }
        if !members.is_empty() {
            b.push(' ');
        }
        b.push_str("});\n");
        let _ = is_coupled;

        for rp in &reactive {
            let stmt = format!("{handle}.setProp({}, {});", js_string(&rp.name), rp.expr);
            self.emit_update(b, frag, &rp.deps, rp.coupled, &stmt);
        }
    }

    /// `<teleport to="selector">…children…</teleport>` (output-design.md §6):
    /// the children are built as their own fragment and inserted into the
    /// target (`document.querySelector(to)` or an element expression) rather
    /// than inline. A permanent anchor marks the inline slot for teardown order.
    fn emit_teleport_slot(
        &mut self,
        b: &mut String,
        el: &TemplateElement,
        pos: &InsertPos,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        if frag.depth >= MAX_BLOCK_DEPTH {
            self.void_block(
                b,
                frag,
                el.open_tag_range,
                "control flow nested too deeply",
                diags,
            );
            return;
        }
        // The target: a static `to="selector"` (a string) or a bound `:to="e"`
        // (an element/selector expression).
        let target: String = el
            .attrs
            .iter()
            .find_map(|a| match a {
                TemplateAttr::Static {
                    name,
                    value: Some(v),
                    ..
                } if name == "to" => {
                    let mut s = String::from("`");
                    for seg in &v.segments {
                        if let TextSegment::Literal { text, .. } = seg {
                            push_template_literal_chunk(&mut s, text);
                        }
                    }
                    s.push('`');
                    Some(s)
                }
                TemplateAttr::Bound { name, expr, .. } if name == "to" => Some(rewrite_expr(
                    &expr.text,
                    &self.component.reactive_vars,
                    frag.shadowed,
                )),
                _ => None,
            })
            .unwrap_or_else(|| "null".to_string());

        let anchor = self.alloc_anchor();
        self.emit_anchor(b, &anchor, pos, frag);

        // Build the teleport body as a fragment: its own hoisted skeleton +
        // recursive wiring, returning the node group (like an :if branch body).
        let tpl = Template {
            nodes: el.children.clone(),
        };
        let skel = build_skeleton(&tpl);
        let html_name = self.hoist_html(skel.html.clone());
        let root = self.alloc_root();

        self.use_helper("teleportBlock");
        push_indent(b, frag.indent);
        b.push_str(self.helper("teleportBlock"));
        b.push_str("(c, ");
        b.push_str(&anchor);
        b.push_str(", () => (");
        b.push_str(&target);
        b.push_str("), () => {\n");
        self.use_helper("fromHTML");
        push_indent(b, frag.indent + 1);
        b.push_str("const ");
        b.push_str(&root);
        b.push_str(" = ");
        b.push_str(self.helper("fromHTML"));
        b.push('(');
        b.push_str(&html_name);
        b.push_str(", ");
        b.push_str(&anchor);
        b.push_str(");\n");
        let inner = Frag {
            base: &root,
            shadowed: frag.shadowed,
            indent: frag.indent + 1,
            depth: frag.depth + 1,
            strip: 0,
        };
        self.emit_fragment(b, &skel, &inner, diags);
        push_indent(b, frag.indent + 1);
        b.push_str("return Array.from(");
        b.push_str(&root);
        b.push_str(".childNodes);\n");
        push_indent(b, frag.indent);
        b.push_str("});\n");
    }

    /// Builds the template-literal value, deps, and item-coupling flag for an
    /// interpolated static attribute/prop value (`a ${x} b`). Shared by element
    /// attribute-text wiring and component string-prop wiring.
    fn attr_text_value(
        &self,
        attr: &str,
        value: &StaticValue,
        frag: &Frag,
    ) -> (String, Vec<u32>, bool) {
        let mut deps = Vec::new();
        let mut coupled = false;
        let mut lit = String::from("`");
        for seg in &value.segments {
            match seg {
                TextSegment::Literal { text, .. } => push_template_literal_chunk(&mut lit, text),
                TextSegment::Interpolation(interp) => {
                    lit.push_str("${");
                    lit.push_str(&rewrite_expr(
                        &interp.expr,
                        &self.component.reactive_vars,
                        frag.shadowed,
                    ));
                    lit.push('}');
                    coupled = coupled || reads_any(&interp.expr, frag.shadowed);
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
        let deps = self.filter_deps(deps, frag.shadowed);
        (lit, deps, coupled)
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
    fn text_run_expr(&self, t: &TemplateText, shadowed: &[String]) -> String {
        let mut out = String::from("`");
        for seg in &t.segments {
            match seg {
                TextSegment::Literal { text, .. } => push_template_literal_chunk(&mut out, text),
                TextSegment::Interpolation(interp) => {
                    out.push_str("${");
                    out.push_str(&rewrite_expr(
                        &interp.expr,
                        &self.component.reactive_vars,
                        shadowed,
                    ));
                    out.push('}');
                }
            }
        }
        out.push('`');
        out
    }

    // --- :if ------------------------------------------------------------------

    fn emit_if_slot(
        &mut self,
        b: &mut String,
        chain: &IfChain,
        pos: &InsertPos,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        if frag.depth >= MAX_BLOCK_DEPTH {
            self.void_block(
                b,
                frag,
                chain.range,
                "control flow nested too deeply",
                diags,
            );
            return;
        }
        // Every branch body must be an element (the parser guarantees this for
        // plain cascades; a component body means child-component mounting,
        // which is a later wave).
        for branch in &chain.branches {
            if !matches!(&*branch.body, TemplateNode::Element(_)) {
                self.void_block(
                    b,
                    frag,
                    branch.range,
                    "`:if` on a component is not supported yet",
                    diags,
                );
                return;
            }
        }

        let anchor = self.alloc_anchor();
        self.emit_anchor(b, &anchor, pos, frag);

        // Union of every condition's deps drives the whole cascade.
        let mut deps: Vec<u32> = Vec::new();
        let mut conds: Vec<String> = Vec::new();
        for branch in &chain.branches {
            if let Some(cond) = &branch.condition {
                if let Some(part) =
                    self.find_dynamic(DynamicKind::IfCondition, &cond.text, cond.range)
                {
                    deps.extend(part.deps.indices().iter().copied());
                }
                conds.push(rewrite_expr(
                    &cond.text,
                    &self.component.reactive_vars,
                    frag.shadowed,
                ));
            }
        }
        deps.sort_unstable();
        deps.dedup();
        let deps = self.filter_deps(deps, frag.shadowed);
        let has_else = chain
            .branches
            .last()
            .is_some_and(|br| br.condition.is_none());

        if chain.branches.len() == 1 {
            // Plain :if — the cheap single-block path.
            self.use_helper("ifBlock");
            push_indent(b, frag.indent);
            b.push_str(self.helper("ifBlock"));
            b.push_str("(c, ");
            b.push_str(&anchor);
            b.push_str(", ");
            push_dep_list(b, &deps);
            b.push_str(", () => (");
            b.push_str(&conds[0]);
            b.push_str("), () => {\n");
            self.emit_block_fragment(b, &chain.branches[0].body, &anchor, frag, diags);
            push_indent(b, frag.indent);
            b.push_str("});\n");
        } else {
            // Cascade: one ifChain; which() maps conditions to a branch index
            // (or -1 when no :else and nothing matched).
            self.use_helper("ifChain");
            push_indent(b, frag.indent);
            b.push_str(self.helper("ifChain"));
            b.push_str("(c, ");
            b.push_str(&anchor);
            b.push_str(", ");
            push_dep_list(b, &deps);
            b.push_str(", () => ");
            for (i, cond) in conds.iter().enumerate() {
                b.push('(');
                b.push_str(cond);
                b.push_str(") ? ");
                b.push_str(&i.to_string());
                b.push_str(" : ");
            }
            if has_else {
                b.push_str(&conds.len().to_string());
            } else {
                b.push_str("-1");
            }
            b.push_str(", [\n");
            for branch in &chain.branches {
                push_indent(b, frag.indent + 1);
                b.push_str("() => {\n");
                let inner = Frag {
                    indent: frag.indent + 1,
                    ..*frag
                };
                self.emit_block_fragment(b, &branch.body, &anchor, &inner, diags);
                push_indent(b, frag.indent + 1);
                b.push_str("},\n");
            }
            push_indent(b, frag.indent);
            b.push_str("]);\n");
        }
    }

    /// Emits the body of a branch `make` closure: build the branch's own
    /// skeleton detached (`fromHTML`), wire it recursively, return its root
    /// node(s). `frag.indent` is the indent of the closure's braces.
    fn emit_block_fragment(
        &mut self,
        b: &mut String,
        body: &TemplateNode,
        anchor: &str,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        let tpl = Template {
            nodes: vec![body.clone()],
        };
        let skel = build_skeleton(&tpl);
        let multi_root = has_top_level_slot(&skel);
        let html_name = self.hoist_html(skel.html.clone());
        let root = self.alloc_root();
        self.use_helper("fromHTML");
        push_indent(b, frag.indent + 1);
        b.push_str("const ");
        b.push_str(&root);
        b.push_str(" = ");
        b.push_str(self.helper("fromHTML"));
        b.push('(');
        b.push_str(&html_name);
        b.push_str(", ");
        b.push_str(anchor);
        b.push_str(");\n");
        let inner = Frag {
            base: &root,
            shadowed: frag.shadowed,
            indent: frag.indent + 1,
            depth: frag.depth + 1,
            strip: 0,
        };
        self.emit_fragment(b, &skel, &inner, diags);
        push_indent(b, frag.indent + 1);
        if multi_root {
            // Top-level anchors travel with the branch as a node group.
            b.push_str("return Array.from(");
            b.push_str(&root);
            b.push_str(".childNodes);\n");
        } else {
            b.push_str("return ");
            b.push_str(&root);
            b.push_str(".childNodes[0];\n");
        }
    }

    // --- :for -------------------------------------------------------------------

    fn emit_for_slot(
        &mut self,
        b: &mut String,
        block: &ForBlock,
        pos: &InsertPos,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        if frag.depth >= MAX_BLOCK_DEPTH {
            self.void_block(
                b,
                frag,
                block.range,
                "control flow nested too deeply",
                diags,
            );
            return;
        }
        let Some(parsed) = parse_for(&block.header.text) else {
            self.void_block(
                b,
                frag,
                block.header.range,
                "unrecognized `:for` header (expected e.g. `item of items`)",
                diags,
            );
            return;
        };
        let body_el: &TemplateElement = match &*block.body {
            TemplateNode::Element(e) => e,
            TemplateNode::If(_) => {
                self.void_block(
                    b,
                    frag,
                    block.range,
                    "`:if` on the same element as `:for` is not supported yet \
                     (wrap the element in the `:if` instead)",
                    diags,
                );
                return;
            }
            _ => {
                self.void_block(
                    b,
                    frag,
                    block.range,
                    "`:for` on a component is not supported yet",
                    diags,
                );
                return;
            }
        };

        // The loop binding names shadow reactive vars inside the item.
        let bindings = binding_names(&parsed.binding);
        if bindings.is_empty() {
            self.void_block(
                b,
                frag,
                block.header.range,
                "could not resolve the `:for` binding pattern",
                diags,
            );
            return;
        }
        if bindings.iter().any(|n| is_generated_name(n)) {
            self.void_block(
                b,
                frag,
                block.header.range,
                "`:for` binding name collides with a compiler-generated \
                 identifier (e0/t0/a0/r0/d0 style); please rename it",
                diags,
            );
            return;
        }
        let mut child_shadowed: Vec<String> = frag.shadowed.to_vec();
        for n in &bindings {
            if !child_shadowed.contains(n) {
                child_shadowed.push(n.clone());
            }
        }

        // Strip the `:key` bound attribute off the item root: it configures
        // the reconciler and is never written to the DOM.
        let mut item_el = body_el.clone();
        let key_expr = take_key_attr(&mut item_el);

        let anchor = self.alloc_anchor();
        self.emit_anchor(b, &anchor, pos, frag);

        // The iterable is evaluated in the ENCLOSING scope (outer shadowing).
        let iterable = rewrite_expr(
            &parsed.iterable,
            &self.component.reactive_vars,
            frag.shadowed,
        );
        let items_expr = match parsed.kind {
            ForKind::Of => format!("Array.from(({iterable}) || [])"),
            ForKind::In => format!("Object.keys(({iterable}) || {{}})"),
        };
        let deps = self.filter_deps(
            self.find_dynamic_by(|p| {
                p.kind == DynamicKind::ForIterable && p.expr == parsed.iterable
            })
            .map(|p| p.deps.indices().to_vec())
            .unwrap_or_default(),
            frag.shadowed,
        );

        // The item's own skeleton (single-root: the item element).
        let tpl = Template {
            nodes: vec![TemplateNode::Element(item_el)],
        };
        let skel = build_skeleton(&tpl);
        let html_name = self.hoist_html(skel.html.clone());
        let root = self.alloc_root();
        let d_wire = self.alloc_data();
        let d_patch = self.alloc_data();

        self.use_helper("forBlock");
        push_indent(b, frag.indent);
        b.push_str(self.helper("forBlock"));
        b.push_str("(c, ");
        b.push_str(&anchor);
        b.push_str(", ");
        push_dep_list(b, &deps);
        b.push_str(", () => ");
        b.push_str(&items_expr);
        b.push_str(", {\n");
        push_indent(b, frag.indent + 1);
        b.push_str("html: ");
        b.push_str(&html_name);
        b.push_str(",\n");
        push_indent(b, frag.indent + 1);
        b.push_str("wire: (");
        b.push_str(&root);
        b.push_str(", ");
        b.push_str(&d_wire);
        b.push_str(") => {\n");
        push_indent(b, frag.indent + 2);
        b.push_str("let ");
        b.push_str(parsed.binding.trim());
        b.push_str(" = ");
        b.push_str(&d_wire);
        b.push_str(";\n");
        let inner = Frag {
            base: &root,
            shadowed: &child_shadowed,
            indent: frag.indent + 2,
            depth: frag.depth + 1,
            strip: 1,
        };
        self.emit_fragment(b, &skel, &inner, diags);
        push_indent(b, frag.indent + 2);
        b.push_str("return (");
        b.push_str(&d_patch);
        b.push_str(") => { (");
        b.push_str(parsed.binding.trim());
        b.push_str(" = ");
        b.push_str(&d_patch);
        b.push_str("); };\n");
        push_indent(b, frag.indent + 1);
        b.push_str("},\n");
        if let Some(key) = key_expr {
            let d_key = self.alloc_data();
            let key_js = rewrite_expr(&key, &self.component.reactive_vars, &child_shadowed);
            push_indent(b, frag.indent + 1);
            b.push_str("keyOf: (");
            b.push_str(&d_key);
            b.push_str(") => { const ");
            b.push_str(parsed.binding.trim());
            b.push_str(" = ");
            b.push_str(&d_key);
            b.push_str("; return (");
            b.push_str(&key_js);
            b.push_str("); },\n");
        }
        push_indent(b, frag.indent);
        b.push_str("});\n");
    }

    /// Voids an unsupported block with a comment and a warning diagnostic. The
    /// module still compiles and runs (never-panic, never-invalid-JS).
    fn void_block(
        &mut self,
        b: &mut String,
        frag: &Frag,
        range: TextRange,
        why: &str,
        diags: &mut Vec<Diagnostic>,
    ) {
        push_indent(b, frag.indent);
        b.push_str("/* lunas: block skipped — ");
        // Keep the comment safe: never allow a comment terminator through.
        b.push_str(&why.replace("*/", "* /"));
        b.push_str(" */\n");
        diags.push(Diagnostic::warning(range, why.to_string()));
    }

    // --- attribute / event wiring ----------------------------------------

    fn emit_attr_and_event_wiring(
        &mut self,
        b: &mut String,
        elems: &[DynamicElement],
        ref_names: &[String],
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        for (i, el) in elems.iter().enumerate() {
            let name = &ref_names[i];
            // Static `class`/`style` literal text of this element, merged into a
            // `:class`/`:style` binding on the same element.
            let static_class = static_attr_literal(&el.attrs, "class");
            let static_style = static_attr_literal(&el.attrs, "style");
            for attr in &el.attrs {
                match attr {
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } if attr_name == "class" => {
                        self.emit_class_style(b, name, "class", &expr.text, &static_class, frag);
                    }
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } if attr_name == "style" => {
                        self.emit_class_style(b, name, "style", &expr.text, &static_style, frag);
                    }
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } if attr_name == "html" => {
                        self.emit_html_bind(b, name, &expr.text, frag);
                    }
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } if attr_name == "ref" => {
                        self.emit_ref(b, name, &expr.text, frag, diags);
                    }
                    TemplateAttr::Bound {
                        name: attr_name,
                        expr,
                        ..
                    } => {
                        self.emit_bound_attr(b, name, attr_name, &expr.text, frag);
                    }
                    TemplateAttr::Static {
                        name: attr_name,
                        value: Some(v),
                        ..
                    } if has_interpolation(v) => {
                        self.emit_attr_text(b, name, attr_name, v, frag);
                    }
                    TemplateAttr::Event { event, handler, .. } => {
                        self.emit_event(b, name, event, &handler.text, frag);
                    }
                    TemplateAttr::TwoWay {
                        name: attr_name,
                        lvalue,
                        ..
                    } => {
                        self.emit_two_way(b, name, attr_name, &lvalue.text, frag);
                    }
                    TemplateAttr::Static { .. } => {}
                }
            }
        }
    }

    fn emit_bound_attr(&mut self, b: &mut String, node: &str, attr: &str, expr: &str, frag: &Frag) {
        let deps = self.filter_deps(self.bound_attr_deps(attr, expr), frag.shadowed);
        let value = rewrite_expr(expr, &self.component.reactive_vars, frag.shadowed);
        let set = attr_set_statement(node, attr, &value);
        self.emit_update(b, frag, &deps, reads_any(expr, frag.shadowed), &set);
    }

    /// `:class="e"` / `:style="e"` (output-design.md §6): the dynamic value is
    /// normalized and merged with the element's static `class`/`style` literal
    /// via the `setClass`/`setStyle` runtime helper. `helper` is `"class"` or
    /// `"style"`; `static_lit` is the static attribute text (empty if none).
    fn emit_class_style(
        &mut self,
        b: &mut String,
        node: &str,
        which: &str,
        expr: &str,
        static_lit: &str,
        frag: &Frag,
    ) {
        let deps = self.filter_deps(self.bound_attr_deps(which, expr), frag.shadowed);
        let value = rewrite_expr(expr, &self.component.reactive_vars, frag.shadowed);
        let helper: &'static str = if which == "class" {
            "setClass"
        } else {
            "setStyle"
        };
        self.use_helper(helper);
        let fn_name = self.helper(helper).to_string();
        let set = format!("{fn_name}({node}, {}, {value});", js_string(static_lit));
        self.emit_update(b, frag, &deps, reads_any(expr, frag.shadowed), &set);
    }

    /// `:html="e"` (output-design.md §6): raw innerHTML insertion. XSS caveat is
    /// on the author — the expression is inserted verbatim. Children of an
    /// element carrying `:html` are diagnosed elsewhere (they would be clobbered
    /// by the innerHTML write).
    fn emit_html_bind(&mut self, b: &mut String, node: &str, expr: &str, frag: &Frag) {
        // `html` is not a real bound-attr dep kind; look it up as an Attribute
        // named "html" (the classifier stores it as a Bound attr).
        let deps = self.filter_deps(self.bound_attr_deps("html", expr), frag.shadowed);
        let value = rewrite_expr(expr, &self.component.reactive_vars, frag.shadowed);
        let set = format!("{node}.innerHTML = {value};");
        self.emit_update(b, frag, &deps, reads_any(expr, frag.shadowed), &set);
    }

    /// `:ref="name"` (output-design.md §6): expose the element to the script by
    /// assigning it into the reactive box `name` (a plain `let name;` in the
    /// script, numbered as a reactive var). Emitted as `name.v = node;` at wire
    /// time — no bind, the reference is fixed for the element's lifetime.
    fn emit_ref(
        &mut self,
        b: &mut String,
        node: &str,
        expr: &str,
        frag: &Frag,
        diags: &mut Vec<Diagnostic>,
    ) {
        self.emit_ref_assign(
            b,
            node,
            expr,
            frag,
            TextRange::new(0.into(), 0.into()),
            diags,
        );
    }

    fn emit_attr_text(
        &mut self,
        b: &mut String,
        node: &str,
        attr: &str,
        value: &StaticValue,
        frag: &Frag,
    ) {
        let (lit, deps, coupled) = self.attr_text_value(attr, value, frag);
        let set = attr_set_statement(node, attr, &lit);
        self.emit_update(b, frag, &deps, coupled, &set);
    }

    fn emit_event(&mut self, b: &mut String, node: &str, event: &str, handler: &str, frag: &Frag) {
        self.use_helper("on");
        let body = rewrite_expr(handler, &self.component.reactive_vars, frag.shadowed);
        push_indent(b, frag.indent);
        b.push_str(self.helper("on"));
        b.push('(');
        b.push_str(node);
        b.push_str(", ");
        push_js_string(b, event);
        b.push_str(", () => { ");
        b.push_str(&body);
        b.push_str("; });\n");
    }

    /// Two-way binding `::name="lvalue"` (§6): the read side is a normal bound
    /// attribute of `lvalue`; the write side is an event listener assigning the
    /// element state back into the lvalue. `::checked` listens on `change`
    /// (checkbox/radio semantics); everything else on `input`.
    fn emit_two_way(&mut self, b: &mut String, node: &str, attr: &str, lvalue: &str, frag: &Frag) {
        // Read side: element reflects the lvalue.
        let deps = self.filter_deps(self.two_way_deps(attr, lvalue), frag.shadowed);
        let value = rewrite_expr(lvalue, &self.component.reactive_vars, frag.shadowed);
        let set = attr_set_statement(node, attr, &value);
        self.emit_update(b, frag, &deps, reads_any(lvalue, frag.shadowed), &set);

        // Write side: element state flows back into the lvalue.
        let (event, read) = two_way_read(node, attr);
        self.use_helper("on");
        push_indent(b, frag.indent);
        b.push_str(self.helper("on"));
        b.push('(');
        b.push_str(node);
        b.push_str(", ");
        push_js_string(b, event);
        b.push_str(", () => { ");
        b.push_str(&value);
        b.push_str(" = ");
        b.push_str(&read);
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

    fn two_way_deps(&self, attr: &str, lvalue: &str) -> Vec<u32> {
        self.find_dynamic_by(|p| {
            matches!(&p.kind, DynamicKind::TwoWay(n) if n == attr) && p.expr == lvalue
        })
        .map(|p| p.deps.indices().to_vec())
        .unwrap_or_default()
    }

    /// Drops dep indices whose reactive variable is shadowed by a loop binding
    /// in the current fragment.
    fn filter_deps(&self, mut deps: Vec<u32>, shadowed: &[String]) -> Vec<u32> {
        if shadowed.is_empty() {
            return deps;
        }
        deps.retain(|i| {
            self.component
                .reactive_vars
                .iter()
                .find(|v| v.index == *i)
                .map(|v| !shadowed.contains(&v.name))
                .unwrap_or(true)
        });
        deps
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

// --- template helpers ------------------------------------------------------

/// Warns when an element carries a `:html="…"` binding *and* has non-whitespace
/// children: the raw-HTML write clobbers those children, so writing both is
/// almost certainly a mistake (output-design.md §6).
fn warn_html_with_children(template: &Template, diags: &mut Vec<Diagnostic>) {
    template.visit(&mut |node: &TemplateNode| {
        if let TemplateNode::Element(e) = node {
            let has_html = e
                .attrs
                .iter()
                .any(|a| matches!(a, TemplateAttr::Bound { name, .. } if name == "html"));
            if !has_html {
                return;
            }
            let has_content = e.children.iter().any(|c| match c {
                TemplateNode::Text(t) => !t.is_whitespace(),
                TemplateNode::Comment(_) => false,
                _ => true,
            });
            if has_content {
                diags.push(Diagnostic::warning(
                    e.open_tag_range,
                    "element has both `:html` and children; the children are \
                     overwritten by the raw-HTML insertion"
                        .to_string(),
                ));
            }
        }
    });
}

/// The set of component tag names actually used anywhere in the template
/// (branch/item bodies included). Drives which `@use` imports are emitted.
fn component_tags_in_template(template: &Template) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    template.visit(&mut |node: &TemplateNode| {
        if let TemplateNode::Component(c) = node {
            out.insert(c.name.clone());
        }
    });
    out
}

/// Whether the template contains a `<component :is=…/>` dynamic component
/// anywhere (branch/item bodies included). When true, every `@use` import is
/// emitted, since a `:is` expression can reference any factory by name.
fn template_has_dynamic_component(template: &Template) -> bool {
    let mut found = false;
    template.visit(&mut |node: &TemplateNode| {
        if let TemplateNode::Element(e) = node {
            if e.name == "component" {
                found = true;
            }
        }
    });
    found
}

/// A collision-proof local import identifier for a child component. Its own tag
/// name when free; otherwise `<Name>$`, adding underscores until unused.
fn child_local_name(tag: &str, reserved: &std::collections::HashSet<String>) -> String {
    if !reserved.contains(tag) && !is_generated_name(tag) {
        return tag.to_string();
    }
    let mut name = format!("{tag}$");
    while reserved.contains(&name) {
        name.push('_');
    }
    name
}

/// Every two-way lvalue expression in the template (branch/item bodies
/// included).
fn two_way_lvalues(template: &Template) -> Vec<String> {
    let mut out = Vec::new();
    template.visit(&mut |node: &TemplateNode| {
        let attrs = match node {
            TemplateNode::Element(e) => &e.attrs,
            TemplateNode::Component(c) => &c.props,
            _ => return,
        };
        for attr in attrs {
            if let TemplateAttr::TwoWay { lvalue, .. } = attr {
                out.push(lvalue.text.clone());
            }
        }
    });
    out
}

/// The concatenated static literal text of a plain `class`/`style` attribute on
/// an element, or `""` if absent. Interpolations are ignored (a `:class`/`:style`
/// binding on the same element handles the dynamic part; a static value with an
/// interpolation is an unusual combination and its interpolated part is dropped
/// from the merge base). Used to merge the static base with a `:class`/`:style`.
fn static_attr_literal(attrs: &[TemplateAttr], name: &str) -> String {
    for attr in attrs {
        if let TemplateAttr::Static {
            name: n,
            value: Some(v),
            ..
        } = attr
        {
            if n == name {
                let mut out = String::new();
                for seg in &v.segments {
                    if let TextSegment::Literal { text, .. } = seg {
                        out.push_str(text);
                    }
                }
                return out;
            }
        }
    }
    String::new()
}

/// Removes a `:key="expr"` bound attribute from a `:for` item root and returns
/// the expression. The key configures the reconciler; it is not DOM state.
fn take_key_attr(el: &mut TemplateElement) -> Option<String> {
    let idx = el.attrs.iter().position(
        |a| matches!(a, TemplateAttr::Bound { name, .. } if name.eq_ignore_ascii_case("key")),
    )?;
    match el.attrs.remove(idx) {
        TemplateAttr::Bound { expr, .. } => Some(expr.text),
        _ => None,
    }
}

/// The identifier names bound by a `:for` binding pattern (`item`, `[i, v]`,
/// `{a, b}`). Uses the scope-aware free-identifier scan: for a pattern text
/// read as an expression, the free identifiers are exactly the bound names
/// (default-value expressions may add extras; that only widens shadowing,
/// which is safe).
fn binding_names(pattern: &str) -> Vec<String> {
    let p = pattern.trim();
    if p.is_empty() {
        return Vec::new();
    }
    // Object patterns parse as blocks when bare; parenthesize.
    let as_expr = format!("({p})");
    lunas_script::free_identifiers(&as_expr).unwrap_or_default()
}

/// Whether a user name would collide with the emitter's generated locals.
fn is_generated_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('e' | 't' | 'a' | 'r' | 'd'))
        && chars.as_str().chars().all(|c| c.is_ascii_digit())
        && name.len() > 1
}

/// Whether `expr` reads any of the (loop-binding) `names`. Conservative: an
/// unparseable expression is assumed to read them (extra re-runs are safe).
fn reads_any(expr: &str, names: &[String]) -> bool {
    if names.is_empty() {
        return false;
    }
    match lunas_script::free_identifiers(expr) {
        Ok(free) => free.iter().any(|n| names.contains(n)),
        Err(_) => true,
    }
}

/// Whether a fragment skeleton has slots inserted at its top level (the
/// fragment root group then includes runtime anchors, so the whole child list
/// must travel as the block handle).
fn has_top_level_slot(skel: &Skeleton) -> bool {
    skel.slots.iter().any(|s| match &s.pos {
        InsertPos::Before(p) => p.len() <= 1,
        InsertPos::BeforeSplit { path, .. } => path.len() <= 1,
        InsertPos::Append(p) => p.is_empty(),
    })
}

/// Drops the first `strip` segments of a positional path (used for `:for`
/// items, whose wiring receives the item root rather than a container).
fn strip_path(path: &[u32], strip: usize) -> &[u32] {
    if path.len() >= strip {
        &path[strip..]
    } else {
        &[]
    }
}

// --- two-way write-back ------------------------------------------------------

/// The (event, element-read expression) pair for a two-way binding's write-back
/// listener.
fn two_way_read(node: &str, attr: &str) -> (&'static str, String) {
    if let Some(prop) = boolean_property(attr) {
        // checkbox/radio style state commits on change
        ("change", format!("{node}.{prop}"))
    } else if idl_property(attr).is_some() {
        ("input", format!("{node}.value"))
    } else if attr
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        ("input", format!("{node}.{attr}"))
    } else {
        ("input", format!("{node}.getAttribute(\"{attr}\")"))
    }
}

// --- reactive box selection & script rewriting ---------------------------

/// The box helper each reactive var should use, in declaration order: `deepBox`
/// if the component deeply mutates the var (member/index write or a mutating
/// array method in the script, or a two-way member lvalue), else `box`.
/// `hint` is the script text plus synthetic template-derived writes.
fn reactive_box_kinds(hint: &str, vars: &[ReactiveVar]) -> Vec<(&'static str, u32)> {
    vars.iter()
        .map(|v| {
            let kind = if is_deeply_mutated(hint, &v.name) {
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
/// (`name.push(`, …). Conservative and dependency-free — there is no AST-level
/// deep-mutation analysis yet. On a false negative the var would use a plain
/// `box`, which still reacts to whole reassignment.
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
/// [`module_binding_references`] so shadowed uses are left alone. `hint` is
/// the deep-mutation detection text (script + template-derived writes).
fn rewrite_script(
    script: &str,
    hint: &str,
    vars: &[ReactiveVar],
    aliases: &std::collections::BTreeMap<&'static str, String>,
) -> String {
    if vars.is_empty() {
        return script.to_string();
    }
    let box_name = |k: &'static str| aliases.get(k).map(|s| s.as_str()).unwrap_or(k);
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
        let kind = if is_deeply_mutated(hint, &d.name) {
            "deepBox"
        } else {
            "box"
        };
        let init_raw = d
            .init_range
            .and_then(|ir| ir.slice(script))
            .unwrap_or("undefined");
        let init = rewrite_expr(init_raw, vars, &[]);
        let text = format!(
            "const {} = {}(c, {}, {})",
            d.name,
            box_name(kind),
            var.index,
            init
        );
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
/// arrow parameter of the same name) are left alone. `shadowed` names (loop
/// bindings of enclosing `:for` items) are excluded from rewriting.
fn rewrite_expr(expr: &str, vars: &[ReactiveVar], shadowed: &[String]) -> String {
    if vars.is_empty() {
        return expr.trim().to_string();
    }
    let reactive: std::collections::HashSet<&str> = vars
        .iter()
        .map(|v| v.name.as_str())
        .filter(|n| !shadowed.iter().any(|s| s == n))
        .collect();
    if reactive.is_empty() {
        return expr.trim().to_string();
    }
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

fn has_interpolation(v: &StaticValue) -> bool {
    v.segments
        .iter()
        .any(|s| matches!(s, TextSegment::Interpolation(_)))
}

fn push_indent(b: &mut String, level: usize) {
    for _ in 0..level {
        b.push_str("  ");
    }
}

/// Emits `base.childNodes[i]…` navigation for a positional path. `[]` is the
/// base itself.
fn push_node_at(b: &mut String, base: &str, path: &[u32]) {
    b.push_str(base);
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

/// A double-quoted JS string literal for `s`.
fn js_string(s: &str) -> String {
    let mut out = String::new();
    push_js_string(&mut out, s);
    out
}

/// A JS object-literal key for a prop name: the bare name if it is a valid
/// identifier, else a quoted string key (an attribute name like `data-x` is not
/// a valid bare key).
fn prop_key(name: &str) -> String {
    let mut chars = name.chars();
    let ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if ok {
        name.to_string()
    } else {
        js_string(name)
    }
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
