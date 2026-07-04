//! The static-skeleton pass: walks the template IR once and produces
//!
//! 1. the **static HTML string** the compiled component assigns via
//!    `root.innerHTML` — comment-free and whitespace-minified so the browser's
//!    fast-path parser applies and `childNodes` positions are stable;
//! 2. every **dynamic slot** (`${…}` text runs, `:if` cascades, `:for` blocks,
//!    child components) with the exact insertion position expressed in
//!    *skeleton coordinates* (paths of static nodes), so the runtime can create
//!    text anchors with plain `insertBefore`;
//! 3. every **dynamic element** (one with a bound/two-way/event attr or an
//!    interpolated static attr) with its positional path for `refs()`.
//!
//! Positions reference only static skeleton nodes, which exist before any
//! anchor is inserted — so anchors can be created in any order (the emitter
//! keeps template order so sibling anchors line up).

use lunas_parser::{
    ComponentUse, ElementKind, ForBlock, IfChain, Template, TemplateAttr, TemplateElement,
    TemplateNode, TemplateText, TextSegment,
};

/// Where a dynamic slot is inserted, in skeleton coordinates.
///
/// A path is a list of `childNodes` indices from the component root (`[]` is
/// the root itself). Text nodes count: paths are computed against the DOM the
/// skeleton HTML parses into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertPos {
    /// Insert before the static node at this path.
    Before(Vec<u32>),
    /// The static text node at `path` holds the text on *both sides* of this
    /// slot (adjacent text merges when the skeleton is parsed). Split it at
    /// `utf16_offset` (`Text.splitText`) and insert before the tail half.
    BeforeSplit { path: Vec<u32>, utf16_offset: u32 },
    /// Append to the element at this path (`[]` = component root).
    Append(Vec<u32>),
}

/// The template construct occupying a dynamic slot.
#[derive(Debug, Clone)]
pub enum SlotContent {
    /// A text run containing at least one `${…}` interpolation. The whole run
    /// becomes one runtime text node updated as a unit.
    Text(TemplateText),
    If(IfChain),
    For(ForBlock),
    Component(ComponentUse),
}

/// A dynamic slot: what goes there and where "there" is.
#[derive(Debug, Clone)]
pub struct Slot {
    pub content: SlotContent,
    pub pos: InsertPos,
}

/// A static-skeleton element that needs a runtime reference because it carries
/// dynamic attributes (bound / two-way / event / interpolated static value).
#[derive(Debug, Clone)]
pub struct DynamicElement {
    pub path: Vec<u32>,
    pub name: String,
    /// All attributes of the element; the emitter picks the dynamic ones.
    pub attrs: Vec<TemplateAttr>,
}

/// Output of the skeleton pass.
#[derive(Debug, Clone)]
pub struct Skeleton {
    /// Static HTML for `root.innerHTML` — no comments, no inter-element
    /// whitespace, dynamic content excluded.
    pub html: String,
    /// Dynamic slots in template (pre-)order.
    pub slots: Vec<Slot>,
    /// Elements needing refs for attribute/event wiring, in document order.
    pub dynamic_elements: Vec<DynamicElement>,
}

/// Runs the skeleton pass over a parsed template.
pub fn build_skeleton(template: &Template) -> Skeleton {
    let mut ctx = Ctx {
        html: String::new(),
        slots: Vec::new(),
        dynamic_elements: Vec::new(),
    };
    walk(&template.nodes, &[], &mut ctx);
    Skeleton {
        html: ctx.html,
        slots: ctx.slots,
        dynamic_elements: ctx.dynamic_elements,
    }
}

struct Ctx {
    html: String,
    slots: Vec<Slot>,
    dynamic_elements: Vec<DynamicElement>,
}

/// Walks one child list. `parent_path` is the skeleton path of the parent
/// element (`[]` for the component root).
fn walk(children: &[TemplateNode], parent_path: &[u32], ctx: &mut Ctx) {
    // Skeleton childNodes index of the next static child.
    let mut idx: u32 = 0;
    // Slots seen since the last static child; they insert before the next one.
    let mut pending: Vec<SlotContent> = Vec::new();
    // If the previous static child was a text node: (its index, its current
    // UTF-16 length). Adjacent static text merges into that node when parsed.
    let mut last_text: Option<(u32, u32)> = None;

    for node in children {
        match node {
            // Comments never reach the skeleton (a comment node disables the
            // fast-path HTML parser).
            TemplateNode::Comment(_) => {}

            TemplateNode::Text(t) if !t.has_interpolation() => {
                if t.is_whitespace() {
                    continue; // inter-element indentation/newlines
                }
                let text = literal_text(t);
                match last_text {
                    Some((ti, len)) => {
                        // Merges into the previous text node when parsed.
                        if !pending.is_empty() {
                            let path = child_path(parent_path, ti);
                            flush(
                                &mut pending,
                                InsertPos::BeforeSplit {
                                    path,
                                    utf16_offset: len,
                                },
                                ctx,
                            );
                        }
                        ctx.html.push_str(&text);
                        last_text = Some((ti, len + utf16_len(&text)));
                    }
                    None => {
                        let path = child_path(parent_path, idx);
                        if !pending.is_empty() {
                            flush(&mut pending, InsertPos::Before(path), ctx);
                        }
                        ctx.html.push_str(&text);
                        last_text = Some((idx, utf16_len(&text)));
                        idx += 1;
                    }
                }
            }

            TemplateNode::Text(t) => pending.push(SlotContent::Text(t.clone())),

            TemplateNode::Element(e) => {
                let path = child_path(parent_path, idx);
                if !pending.is_empty() {
                    flush(&mut pending, InsertPos::Before(path.clone()), ctx);
                }
                emit_element(e, &path, ctx);
                last_text = None;
                idx += 1;
            }

            TemplateNode::Component(c) => pending.push(SlotContent::Component(c.clone())),
            TemplateNode::If(chain) => pending.push(SlotContent::If(chain.clone())),
            TemplateNode::For(block) => pending.push(SlotContent::For(block.clone())),
        }
    }

    if !pending.is_empty() {
        flush(&mut pending, InsertPos::Append(parent_path.to_vec()), ctx);
    }
}

fn emit_element(e: &TemplateElement, path: &[u32], ctx: &mut Ctx) {
    ctx.html.push('<');
    ctx.html.push_str(&e.name);

    let mut is_dynamic = false;
    for attr in &e.attrs {
        match attr {
            TemplateAttr::Static { name, value, .. } => match value {
                None => {
                    ctx.html.push(' ');
                    ctx.html.push_str(name);
                }
                Some(v) if v.segments.iter().any(is_interpolation) => {
                    // Value depends on state: set at runtime via the ref.
                    is_dynamic = true;
                }
                Some(v) => {
                    ctx.html.push(' ');
                    ctx.html.push_str(name);
                    ctx.html.push_str("=\"");
                    for seg in &v.segments {
                        if let TextSegment::Literal { text, .. } = seg {
                            push_attr_escaped(&mut ctx.html, text);
                        }
                    }
                    ctx.html.push('"');
                }
            },
            TemplateAttr::Bound { .. }
            | TemplateAttr::TwoWay { .. }
            | TemplateAttr::Event { .. } => {
                is_dynamic = true;
            }
        }
    }
    ctx.html.push('>');

    if is_dynamic {
        ctx.dynamic_elements.push(DynamicElement {
            path: path.to_vec(),
            name: e.name.clone(),
            attrs: e.attrs.clone(),
        });
    }

    if matches!(e.kind, ElementKind::Void) {
        return;
    }
    walk(&e.children, path, ctx);
    ctx.html.push_str("</");
    ctx.html.push_str(&e.name);
    ctx.html.push('>');
}

fn flush(pending: &mut Vec<SlotContent>, pos: InsertPos, ctx: &mut Ctx) {
    for content in pending.drain(..) {
        ctx.slots.push(Slot {
            content,
            pos: pos.clone(),
        });
    }
}

fn child_path(parent: &[u32], index: u32) -> Vec<u32> {
    let mut p = parent.to_vec();
    p.push(index);
    p
}

/// Concatenated literal text of a run with no interpolations. Emitted verbatim:
/// the author wrote this region as HTML text, so re-emitting it unchanged
/// (entities included) round-trips through the parser.
fn literal_text(t: &TemplateText) -> String {
    t.segments
        .iter()
        .filter_map(|seg| match seg {
            TextSegment::Literal { text, .. } => Some(text.as_str()),
            TextSegment::Interpolation(_) => None,
        })
        .collect()
}

fn is_interpolation(seg: &TextSegment) -> bool {
    matches!(seg, TextSegment::Interpolation(_))
}

/// Escapes a static attribute value for double-quoted emission.
fn push_attr_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        if ch == '"' {
            out.push_str("&quot;");
        } else {
            out.push(ch);
        }
    }
}

fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}
