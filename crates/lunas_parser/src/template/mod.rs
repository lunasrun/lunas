//! Binding-aware post-processing pass over the HTML `Dom`.
//!
//! Walks the (already file-absolute) `Dom` and lowers it into the template IR:
//! splits `${…}` interpolations out of text and attribute values, classifies
//! `:`/`::`/`@` attributes, resolves components against the `@use` table, and
//! groups `:if`/`:elseif`/`:else` cascades and `:for` loops. Purely syntactic
//! and never-panic; embedded JS stays opaque text (see `ir.rs`).

mod ir;
mod scan;

pub use ir::Interpolation;
pub use ir::{
    BranchKind, ComponentUse, Expr, ForBlock, ForHeader, IfBranch, IfChain, StaticValue, Template,
    TemplateAttr, TemplateElement, TemplateNode, TemplateText, TextSegment,
};

use lunas_html_parser::{Dom, Element, Node};
use lunas_span::{Diagnostic, TextRange};
use std::collections::HashSet;

/// Reserved bound-attribute names that the generator does not support.
const RESERVED_BOUND: &[&str] = &["innerHtml", "textContent"];

/// Lowers a parsed `Dom` into the binding-aware [`Template`], appending any
/// problems to `diagnostics`. `components` is the set of names declared via
/// `@use` (case-sensitive).
pub(crate) fn build(
    source: &str,
    dom: &Dom,
    components: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Template {
    Template {
        nodes: process_nodes(source, &dom.children, components, diagnostics),
    }
}

fn process_nodes(
    source: &str,
    nodes: &[Node],
    components: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<TemplateNode> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            Node::Comment(c) => {
                out.push(TemplateNode::Comment(c.clone()));
                i += 1;
            }
            Node::Text(t) => {
                out.push(TemplateNode::Text(TemplateText {
                    segments: scan::scan_segments(&t.value, t.range.start(), diags),
                    range: t.range,
                }));
                i += 1;
            }
            Node::Element(el) => {
                let cf = read_control_flow(el, diags);
                if let Some(header) = cf.for_header {
                    out.push(build_for(source, el, header, cf.branch, components, diags));
                    i += 1;
                } else if let Some(branch) = cf.branch {
                    match branch.kind {
                        BranchKind::If => {
                            let (chain, consumed) =
                                build_if_chain(source, nodes, i, branch, components, diags);
                            out.push(TemplateNode::If(chain));
                            i += consumed;
                        }
                        BranchKind::ElseIf | BranchKind::Else => {
                            diags.push(Diagnostic::error(
                                branch.range,
                                "`:elseif`/`:else` without a matching `:if`",
                            ));
                            out.push(lower_element(source, el, components, diags));
                            i += 1;
                        }
                    }
                } else {
                    out.push(lower_element(source, el, components, diags));
                    i += 1;
                }
            }
        }
    }
    out
}

/// `:for` is the outer block; an accompanying `:if` becomes a single-branch
/// cascade inside it (the documented `:for` outer / `:if` inner precedence).
fn build_for(
    source: &str,
    el: &Element,
    header: ForHeader,
    branch: Option<BranchInfo>,
    components: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) -> TemplateNode {
    let base = lower_element(source, el, components, diags);
    let body = match branch {
        Some(b) if b.kind == BranchKind::If => TemplateNode::If(IfChain {
            range: el.range,
            branches: vec![IfBranch {
                kind: BranchKind::If,
                condition: b.condition,
                body: Box::new(base),
                range: el.range,
            }],
        }),
        Some(b) => {
            diags.push(Diagnostic::error(
                b.range,
                "`:elseif`/`:else` cannot be combined with `:for`",
            ));
            base
        }
        None => base,
    };
    TemplateNode::For(ForBlock {
        header,
        body: Box::new(body),
        range: el.range,
    })
}

/// Builds an `:if` cascade starting at `nodes[start]`, consuming adjacent
/// `:elseif`/`:else` siblings (whitespace-only text between branches is
/// skipped). Returns the chain and how many `nodes` entries it consumed.
fn build_if_chain(
    source: &str,
    nodes: &[Node],
    start: usize,
    first: BranchInfo,
    components: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) -> (IfChain, usize) {
    let first_el = match &nodes[start] {
        Node::Element(e) => e,
        _ => unreachable!("build_if_chain starts on an element"),
    };
    let mut range = first_el.range;
    let mut branches = vec![IfBranch {
        kind: BranchKind::If,
        condition: first.condition,
        body: Box::new(lower_element(source, first_el, components, diags)),
        range: first_el.range,
    }];

    let mut j = start + 1;
    loop {
        // Look past whitespace-only text to the next meaningful node.
        let mut k = j;
        while k < nodes.len() && is_whitespace_text(&nodes[k]) {
            k += 1;
        }
        let el = match nodes.get(k) {
            Some(Node::Element(e)) => e,
            _ => break,
        };
        let cf = read_control_flow(el, diags);
        if cf.for_header.is_some() {
            break;
        }
        match cf.branch {
            Some(b) if b.kind == BranchKind::ElseIf => {
                range = range.cover(el.range);
                branches.push(IfBranch {
                    kind: BranchKind::ElseIf,
                    condition: b.condition,
                    body: Box::new(lower_element(source, el, components, diags)),
                    range: el.range,
                });
                j = k + 1;
            }
            Some(b) if b.kind == BranchKind::Else => {
                range = range.cover(el.range);
                branches.push(IfBranch {
                    kind: BranchKind::Else,
                    condition: None,
                    body: Box::new(lower_element(source, el, components, diags)),
                    range: el.range,
                });
                j = k + 1;
                break; // `:else` terminates the cascade.
            }
            _ => break,
        }
    }

    (IfChain { branches, range }, j - start)
}

fn lower_element(
    source: &str,
    el: &Element,
    components: &HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) -> TemplateNode {
    let attrs: Vec<TemplateAttr> = el
        .attributes
        .iter()
        .filter(|a| !is_control_flow_key(&a.name))
        .filter_map(|a| classify_attr(a, diags))
        .collect();
    let children = process_nodes(source, &el.children, components, diags);

    if components.contains(&el.raw_name) {
        TemplateNode::Component(ComponentUse {
            name: el.raw_name.clone(),
            props: attrs,
            children,
            range: el.range,
            open_tag_range: el.open_tag_range,
        })
    } else {
        TemplateNode::Element(TemplateElement {
            name: el.name.clone(),
            kind: el.kind,
            attrs,
            children,
            range: el.range,
            open_tag_range: el.open_tag_range,
        })
    }
}

fn classify_attr(
    attr: &lunas_html_parser::Attribute,
    diags: &mut Vec<Diagnostic>,
) -> Option<TemplateAttr> {
    let key = attr.name.as_str();
    if let Some(name) = key.strip_prefix("::") {
        Some(TemplateAttr::TwoWay {
            name: name.to_string(),
            lvalue: expr_of(attr, diags),
            range: attr.range,
        })
    } else if let Some(event) = key.strip_prefix('@') {
        Some(TemplateAttr::Event {
            event: event.to_string(),
            handler: expr_of(attr, diags),
            range: attr.range,
        })
    } else if let Some(name) = key.strip_prefix(':') {
        if RESERVED_BOUND.iter().any(|r| r.eq_ignore_ascii_case(name)) {
            diags.push(Diagnostic::error(
                attr.range,
                format!("`:{}` binding is not supported", name),
            ));
            return None;
        }
        Some(TemplateAttr::Bound {
            name: name.to_string(),
            expr: expr_of(attr, diags),
            range: attr.range,
        })
    } else {
        Some(TemplateAttr::Static {
            name: key.to_string(),
            value: static_value(attr, diags),
            range: attr.range,
        })
    }
}

/// Extracts the raw JS expression of a bound/event/two-way attribute. A missing
/// value is reported and yields an empty expression so the node still builds.
fn expr_of(attr: &lunas_html_parser::Attribute, diags: &mut Vec<Diagnostic>) -> Expr {
    match (&attr.value, attr.value_range) {
        (Some(text), Some(range)) => Expr {
            text: text.clone(),
            range,
        },
        _ => {
            diags.push(Diagnostic::error(
                attr.range,
                format!("`{}` expects an expression value", attr.name),
            ));
            Expr {
                text: String::new(),
                range: attr.range,
            }
        }
    }
}

fn static_value(
    attr: &lunas_html_parser::Attribute,
    diags: &mut Vec<Diagnostic>,
) -> Option<StaticValue> {
    let (text, range) = match (&attr.value, attr.value_range) {
        (Some(t), Some(r)) => (t, r),
        _ => return None,
    };
    Some(StaticValue {
        segments: scan::scan_segments(text, range.start(), diags),
        range,
    })
}

/// A control-flow directive read off an element's attributes.
struct ControlFlow {
    branch: Option<BranchInfo>,
    for_header: Option<ForHeader>,
}

struct BranchInfo {
    kind: BranchKind,
    condition: Option<Expr>,
    range: TextRange,
}

fn read_control_flow(el: &Element, diags: &mut Vec<Diagnostic>) -> ControlFlow {
    let mut branch = None;
    let mut for_header = None;

    for attr in &el.attributes {
        match attr.name.as_str() {
            ":if" | ":elseif" => {
                if branch.is_some() {
                    continue;
                }
                let kind = if attr.name == ":if" {
                    BranchKind::If
                } else {
                    BranchKind::ElseIf
                };
                branch = Some(BranchInfo {
                    kind,
                    condition: Some(required_condition(attr, diags)),
                    range: attr.range,
                });
            }
            ":else" => {
                if branch.is_some() {
                    continue;
                }
                if attr.value.is_some() {
                    diags.push(Diagnostic::warning(
                        attr.range,
                        "`:else` does not take a value",
                    ));
                }
                branch = Some(BranchInfo {
                    kind: BranchKind::Else,
                    condition: None,
                    range: attr.range,
                });
            }
            ":for" => {
                if for_header.is_some() {
                    continue;
                }
                match (&attr.value, attr.value_range) {
                    (Some(text), Some(range)) if !text.trim().is_empty() => {
                        for_header = Some(ForHeader {
                            text: text.clone(),
                            range,
                        });
                    }
                    _ => diags.push(Diagnostic::error(
                        attr.range,
                        "`:for` expects a loop header, e.g. `item of items`",
                    )),
                }
            }
            _ => {}
        }
    }

    ControlFlow { branch, for_header }
}

fn required_condition(attr: &lunas_html_parser::Attribute, diags: &mut Vec<Diagnostic>) -> Expr {
    match (&attr.value, attr.value_range) {
        (Some(text), Some(range)) if !text.trim().is_empty() => Expr {
            text: text.clone(),
            range,
        },
        _ => {
            diags.push(Diagnostic::error(
                attr.range,
                format!("`{}` expects a condition expression", attr.name),
            ));
            Expr {
                text: String::new(),
                range: attr.range,
            }
        }
    }
}

fn is_control_flow_key(key: &str) -> bool {
    matches!(key, ":if" | ":elseif" | ":else" | ":for")
}

fn is_whitespace_text(node: &Node) -> bool {
    matches!(node, Node::Text(t) if t.value.trim().is_empty())
}
