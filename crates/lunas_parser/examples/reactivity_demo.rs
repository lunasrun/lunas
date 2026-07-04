//! Demonstrates the intended end-to-end reactivity flow the future
//! `lunas_compiler` orchestrator will drive, using only the building blocks
//! that already exist:
//!
//!   parse → component bindings (lunas_script::declared_bindings + @input props)
//!         → for each template expression, free_identifiers ∩ bindings
//!
//! Run with `cargo run -p lunas_parser --example reactivity_demo`.

use lunas_parser::{parse, Directive, TemplateAttr, TemplateNode};
use lunas_script::{
    analyze_script, assigned_identifiers, free_identifiers, parse_for, referenced_identifiers,
    ScriptAnalysis,
};

fn empty_analysis() -> ScriptAnalysis {
    ScriptAnalysis {
        bindings: Vec::new(),
        function_mutations: Vec::new(),
    }
}

fn main() {
    let src = "\
@input label:string
html:
    <div :class=\"theme\">${ count + label }</div>
    <button @click=\"add()\">${ label }</button>
    <li :for=\"item of items\">${ item.name }</li>
script:
    let count = 0
    let theme = \"dark\"
    let items = []
    function add(){ items = items.concat(label); count++ }
";

    let (file, diagnostics) = parse(src);
    for d in &diagnostics {
        println!("{}", d.render(src, &file.line_index));
    }

    // One analysis pass over the script: declared bindings + per-function
    // mutation sets. Component bindings also include the @input props.
    let analysis = file
        .script
        .as_ref()
        .map(|s| analyze_script(&s.source.text).unwrap_or_else(|_| empty_analysis()))
        .unwrap_or_else(empty_analysis);
    let mut bindings = analysis.bindings.clone();
    for d in &file.directives {
        if let Directive::Input(p) = d {
            bindings.push(p.name.clone());
        }
    }
    println!("component bindings: {:?}\n", bindings);

    let deps_of = |text: &str| -> Vec<String> {
        free_identifiers(text)
            .unwrap_or_default()
            .into_iter()
            .filter(|id| bindings.contains(id))
            .collect()
    };

    if let Some(html) = &file.html {
        // Directly-analyzable expressions (interpolations, bound/event attrs,
        // if conditions).
        println!("reactive dependencies per expression:");
        html.template.for_each_expression(|text, range| {
            let lc = file.line_index.line_col(range.start());
            println!(
                "  {}:{}  {:?}  ->  {:?}",
                lc.line + 1,
                lc.col + 1,
                text,
                deps_of(text)
            );
        });

        // `:for` headers need parse_for first: the iterable is the reactive part.
        println!("\n:for loops (iterable analyzed via parse_for):");
        html.template.visit(&mut |n| {
            if let TemplateNode::For(block) = n {
                if let Some(parsed) = parse_for(&block.header.text) {
                    println!(
                        "  {:?}  binding={:?}  iterable={:?}  ->  {:?}",
                        block.header.text,
                        parsed.binding,
                        parsed.iterable,
                        deps_of(&parsed.iterable)
                    );
                }
            }
        });

        // Event handler *effects*: what state re-renders when it fires. A handler
        // that calls a function inherits that function's mutation set, which is
        // why `add()` (no direct assignment) still re-renders items + count.
        let fn_muts = &analysis.function_mutations;
        println!("\nevent handler effects (state to re-render):");
        html.template.visit(&mut |n| {
            let attrs = match n {
                TemplateNode::Element(e) => &e.attrs,
                TemplateNode::Component(c) => &c.props,
                _ => return,
            };
            for a in attrs {
                if let TemplateAttr::Event { event, handler, .. } = a {
                    let mut effects = Vec::new();
                    // Direct mutations in the handler text.
                    for m in assigned_identifiers(&handler.text).unwrap_or_default() {
                        if bindings.contains(&m) && !effects.contains(&m) {
                            effects.push(m);
                        }
                    }
                    // Mutations of any function the handler calls.
                    for called in referenced_identifiers(&handler.text).unwrap_or_default() {
                        if let Some((_, muts)) = fn_muts.iter().find(|(name, _)| *name == called) {
                            for m in muts {
                                if bindings.contains(m) && !effects.contains(m) {
                                    effects.push(m.clone());
                                }
                            }
                        }
                    }
                    println!("  @{}={:?}  ->  {:?}", event, handler.text, effects);
                }
            }
        });
    }
}
