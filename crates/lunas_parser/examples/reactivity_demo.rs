//! Demonstrates the intended end-to-end reactivity flow the future
//! `lunas_compiler` orchestrator will drive, using only the building blocks
//! that already exist:
//!
//!   parse → component bindings (lunas_script::declared_bindings + @input props)
//!         → for each template expression, free_identifiers ∩ bindings
//!
//! Run with `cargo run -p lunas_parser --example reactivity_demo`.

use lunas_parser::{parse, Directive, TemplateNode};
use lunas_script::{declared_bindings, free_identifiers, parse_for};

fn main() {
    let src = "\
@input label:string
html:
    <div :class=\"theme\">${ count + label }</div>
    <button @click=\"count++\">${ label }</button>
    <li :for=\"item of items\">${ item.name }</li>
script:
    let count = 0
    let theme = \"dark\"
    let items = []
";

    let (file, diagnostics) = parse(src);
    for d in &diagnostics {
        println!("{}", d.render(src, &file.line_index));
    }

    // Component bindings = top-level script declarations + @input props.
    let mut bindings = file
        .script
        .as_ref()
        .map(|s| declared_bindings(&s.source.text).unwrap_or_default())
        .unwrap_or_default();
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
    }
}
