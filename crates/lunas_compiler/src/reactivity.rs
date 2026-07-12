//! Reactivity resolution: builds the script's function dependency/mutation
//! graph, then walks the template annotating every dynamic expression with the
//! reactive variables it reads (and every handler with what it writes).

use std::collections::{BTreeSet, HashMap, HashSet};

use lunas_parser::{
    Expr, ForBlock, IfChain, StaticValue, Template, TemplateAttr, TemplateNode, TemplateText,
    TextSegment,
};
use lunas_script::{
    assigned_identifiers, free_identifiers, function_dependencies, function_mutations, parse_for,
};

use crate::model::{Deps, DynamicKind, DynamicPart, ReactiveVar, ResolvedHandler};

/// The script's reactive dependency graph, with per-function transitive read
/// and write sets already reduced to reactive-variable indices.
pub(crate) struct DependencyGraph {
    reactive: HashMap<String, u32>,
    func_reads: HashMap<String, Vec<u32>>,
    func_writes: HashMap<String, Vec<u32>>,
}

impl DependencyGraph {
    pub(crate) fn build(script_text: &str, reactive_vars: &[ReactiveVar]) -> Self {
        let reactive: HashMap<String, u32> = reactive_vars
            .iter()
            .map(|v| (v.name.clone(), v.index))
            .collect();
        let reads_raw: HashMap<String, Vec<String>> = function_dependencies(script_text)
            .unwrap_or_default()
            .into_iter()
            .collect();
        let writes_raw: HashMap<String, Vec<String>> = function_mutations(script_text)
            .unwrap_or_default()
            .into_iter()
            .collect();
        let func_names: HashSet<String> = reads_raw.keys().cloned().collect();

        let mut func_reads = HashMap::new();
        let mut func_writes = HashMap::new();
        for name in &func_names {
            func_reads.insert(
                name.clone(),
                transitive_reads(name, &reads_raw, &reactive, &func_names),
            );
            func_writes.insert(
                name.clone(),
                transitive_writes(name, &reads_raw, &writes_raw, &reactive, &func_names),
            );
        }
        DependencyGraph {
            reactive,
            func_reads,
            func_writes,
        }
    }

    /// Reactive variables read by an expression, transitively through any
    /// top-level functions it calls.
    fn reads_of_expr(&self, expr: &str) -> Deps {
        let mut acc = Vec::new();
        if let Ok(free) = free_identifiers(expr) {
            for name in free {
                if let Some(&i) = self.reactive.get(&name) {
                    acc.push(i);
                }
                if let Some(deps) = self.func_reads.get(&name) {
                    acc.extend(deps.iter().copied());
                }
            }
        }
        Deps::from_indices(acc)
    }

    /// Reactive variables written by a handler, transitively through any
    /// top-level functions it calls.
    fn writes_of_handler(&self, handler: &str) -> Deps {
        let mut acc = Vec::new();
        if let Ok(assigned) = assigned_identifiers(handler) {
            for name in assigned {
                if let Some(&i) = self.reactive.get(&name) {
                    acc.push(i);
                }
            }
        }
        if let Ok(free) = free_identifiers(handler) {
            for name in free {
                if let Some(deps) = self.func_writes.get(&name) {
                    acc.extend(deps.iter().copied());
                }
            }
        }
        Deps::from_indices(acc)
    }
}

fn transitive_reads(
    start: &str,
    reads_raw: &HashMap<String, Vec<String>>,
    reactive: &HashMap<String, u32>,
    func_names: &HashSet<String>,
) -> Vec<u32> {
    let mut acc = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(f) = stack.pop() {
        if !visited.insert(f.clone()) {
            continue;
        }
        if let Some(names) = reads_raw.get(&f) {
            for name in names {
                if let Some(&i) = reactive.get(name) {
                    acc.insert(i);
                }
                if func_names.contains(name) {
                    stack.push(name.clone());
                }
            }
        }
    }
    acc.into_iter().collect()
}

fn transitive_writes(
    start: &str,
    reads_raw: &HashMap<String, Vec<String>>,
    writes_raw: &HashMap<String, Vec<String>>,
    reactive: &HashMap<String, u32>,
    func_names: &HashSet<String>,
) -> Vec<u32> {
    let mut acc = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(f) = stack.pop() {
        if !visited.insert(f.clone()) {
            continue;
        }
        if let Some(names) = writes_raw.get(&f) {
            for name in names {
                if let Some(&i) = reactive.get(name) {
                    acc.insert(i);
                }
            }
        }
        // Call edges come from the read graph (a called function is read).
        if let Some(names) = reads_raw.get(&f) {
            for name in names {
                if func_names.contains(name) {
                    stack.push(name.clone());
                }
            }
        }
    }
    acc.into_iter().collect()
}

/// Walks the template, producing every dynamic part and resolved handler.
pub(crate) fn collect(
    template: &Template,
    graph: &DependencyGraph,
) -> (Vec<DynamicPart>, Vec<ResolvedHandler>) {
    let mut dynamics = Vec::new();
    let mut handlers = Vec::new();
    for node in &template.nodes {
        walk(node, graph, &mut dynamics, &mut handlers);
    }
    (dynamics, handlers)
}

fn walk(
    node: &TemplateNode,
    graph: &DependencyGraph,
    dynamics: &mut Vec<DynamicPart>,
    handlers: &mut Vec<ResolvedHandler>,
) {
    match node {
        TemplateNode::Text(t) => collect_text(t, graph, dynamics),
        TemplateNode::Element(e) => {
            collect_attrs(&e.attrs, graph, dynamics, handlers);
            for c in &e.children {
                walk(c, graph, dynamics, handlers);
            }
        }
        TemplateNode::Component(c) => {
            collect_attrs(&c.props, graph, dynamics, handlers);
            for ch in &c.children {
                walk(ch, graph, dynamics, handlers);
            }
        }
        TemplateNode::If(chain) => collect_if(chain, graph, dynamics, handlers),
        TemplateNode::For(block) => collect_for(block, graph, dynamics, handlers),
        TemplateNode::Comment(_) => {}
    }
}

fn collect_text(t: &TemplateText, graph: &DependencyGraph, dynamics: &mut Vec<DynamicPart>) {
    for seg in &t.segments {
        if let TextSegment::Interpolation(i) = seg {
            dynamics.push(DynamicPart {
                kind: DynamicKind::Text,
                expr: i.expr.clone(),
                range: i.expr_range,
                deps: graph.reads_of_expr(&i.expr),
            });
        }
    }
}

fn collect_attrs(
    attrs: &[TemplateAttr],
    graph: &DependencyGraph,
    dynamics: &mut Vec<DynamicPart>,
    handlers: &mut Vec<ResolvedHandler>,
) {
    for attr in attrs {
        match attr {
            TemplateAttr::Bound { name, expr, .. } => {
                push_expr(dynamics, DynamicKind::Attribute(name.clone()), expr, graph)
            }
            TemplateAttr::TwoWay { name, lvalue, .. } => {
                push_expr(dynamics, DynamicKind::TwoWay(name.clone()), lvalue, graph)
            }
            TemplateAttr::Event { event, handler, .. } => handlers.push(ResolvedHandler {
                event: event.clone(),
                handler: handler.text.clone(),
                range: handler.range,
                writes: graph.writes_of_handler(&handler.text),
            }),
            TemplateAttr::Static {
                name,
                value: Some(v),
                ..
            } => collect_static_value(name, v, graph, dynamics),
            TemplateAttr::Static { value: None, .. } => {}
        }
    }
}

fn collect_static_value(
    name: &str,
    value: &StaticValue,
    graph: &DependencyGraph,
    dynamics: &mut Vec<DynamicPart>,
) {
    for seg in &value.segments {
        if let TextSegment::Interpolation(i) = seg {
            dynamics.push(DynamicPart {
                kind: DynamicKind::AttributeText(name.to_string()),
                expr: i.expr.clone(),
                range: i.expr_range,
                deps: graph.reads_of_expr(&i.expr),
            });
        }
    }
}

fn collect_if(
    chain: &IfChain,
    graph: &DependencyGraph,
    dynamics: &mut Vec<DynamicPart>,
    handlers: &mut Vec<ResolvedHandler>,
) {
    for branch in &chain.branches {
        if let Some(cond) = &branch.condition {
            push_expr(dynamics, DynamicKind::IfCondition, cond, graph);
        }
        walk(&branch.body, graph, dynamics, handlers);
    }
}

fn collect_for(
    block: &ForBlock,
    graph: &DependencyGraph,
    dynamics: &mut Vec<DynamicPart>,
    handlers: &mut Vec<ResolvedHandler>,
) {
    if let Some(parsed) = parse_for(&block.header.text) {
        // Prefer the iterable's precise span; fall back to the whole header.
        let range = match block.header.text.find(&parsed.iterable) {
            Some(off) => {
                let start = block.header.range.start().raw() + off as u32;
                lunas_parser::TextRange::at(start, start + parsed.iterable.len() as u32)
            }
            None => block.header.range,
        };
        dynamics.push(DynamicPart {
            kind: DynamicKind::ForIterable,
            deps: graph.reads_of_expr(&parsed.iterable),
            expr: parsed.iterable,
            range,
        });
    }
    walk(&block.body, graph, dynamics, handlers);
}

fn push_expr(
    dynamics: &mut Vec<DynamicPart>,
    kind: DynamicKind,
    expr: &Expr,
    graph: &DependencyGraph,
) {
    dynamics.push(DynamicPart {
        kind,
        expr: expr.text.clone(),
        range: expr.range,
        deps: graph.reads_of_expr(&expr.text),
    });
}
