//! Demonstrates the LSP navigation foundation: given a `.lunas` source and a
//! binding name, print where it is declared in the `script:` block and every
//! place it is referenced across the template — all in file `line:col`.
//!
//! This is the data a Lunas language server needs for go-to-definition and
//! find-references / highlight / rename of a component binding.
//!
//! Run: `cargo run -p lunas_parser --example lsp_demo`

use lunas_parser::{parse, LineCol, TextRange};
use lunas_script::{declared_bindings_with_spans, referenced_identifiers_with_spans};

fn main() {
    let target = "count";
    let src = "\
html:
    <div :class=\"count > 0 ? 'on' : 'off'\">${ count }</div>
    <button @click=\"count = count + 1\">+</button>
script:
    let count = 0
    function inc(){ count++ }
";

    let (file, _) = parse(src);
    let show = |label: &str, r: TextRange| {
        let LineCol { line, col } = file.line_index.line_col(r.start());
        println!(
            "  {label}  {}:{}  {:?}",
            line + 1,
            col + 1,
            r.slice(src).unwrap_or("")
        );
    };

    println!("binding: {target:?}\n");

    println!("declaration:");
    if let Some(script) = &file.script {
        for (name, local) in declared_bindings_with_spans(&script.source.text).unwrap_or_default() {
            if name == target {
                show("def", local.shifted(script.source.range.start()));
            }
        }
    }

    println!("\ntemplate references:");
    if let Some(html) = &file.html {
        html.template.for_each_expression(|text, expr_range| {
            for (name, local) in referenced_identifiers_with_spans(text).unwrap_or_default() {
                if name == target {
                    show("ref", local.shifted(expr_range.start()));
                }
            }
        });
    }
}
