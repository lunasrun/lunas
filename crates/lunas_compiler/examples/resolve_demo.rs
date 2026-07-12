//! End-to-end demo of the resolution layer: parse a `.lunas` component and
//! print the resolved model a code generator would consume.
//!
//! Run with: `cargo run -p lunas_compiler --example resolve_demo`

use lunas_compiler::{resolve, DynamicKind};

const SOURCE: &str = "\
@input start:number = 0
@use Row from \"./Row.lunas\"
html:
    <div :class=\"theme\">
        <button @click=\"inc()\">+1</button>
        <p>count is ${count}, doubled ${double()}</p>
        <Row :if=\"count > 0\" :value=\"count\" />
        <ul>
            <li :for=\"item of items\">${item}</li>
        </ul>
    </div>
style:
    div { padding: 4px }
script:
    let count = start
    let theme = \"light\"
    let items = []
    const title = \"demo\"
    function inc(){ count++; items = items.concat(count) }
    function double(){ return count * 2 }
";

fn main() {
    let (c, diags) = resolve(SOURCE);

    println!("== diagnostics ==");
    if diags.is_empty() {
        println!("  (none)");
    }
    for d in &diags {
        println!("  {}: {}", d.severity.label(), d.message);
    }

    println!("\n== props ==");
    for p in &c.props {
        println!(
            "  {}{}{}",
            p.name,
            p.type_annotation
                .as_ref()
                .map(|t| format!(": {t}"))
                .unwrap_or_default(),
            p.default_value
                .as_ref()
                .map(|v| format!(" = {v}"))
                .unwrap_or_default(),
        );
    }

    println!("\n== child components (@use) ==");
    for u in &c.imports {
        println!("  {} from {}", u.component_name, u.path);
    }

    println!("\n== reactive variables (numbered) ==");
    for v in &c.reactive_vars {
        println!("  [{}] {}", v.index, v.name);
    }
    println!("  (non-reactive consts like `title` are not numbered)");

    println!("\n== dynamic parts (expr -> reactive deps) ==");
    for d in &c.dynamics {
        println!(
            "  {:<22} {:<14} deps={:?}",
            kind_label(&d.kind),
            d.expr,
            names(&c, d.deps.indices()),
        );
    }

    println!("\n== event handlers (event -> writes) ==");
    for h in &c.handlers {
        println!(
            "  @{} = {:<12} writes={:?}",
            h.event,
            h.handler,
            names(&c, h.writes.indices()),
        );
    }
}

fn kind_label(k: &DynamicKind) -> String {
    match k {
        DynamicKind::Text => "text".into(),
        DynamicKind::Attribute(n) => format!(":{n}"),
        DynamicKind::AttributeText(n) => format!("{n}(text)"),
        DynamicKind::TwoWay(n) => format!("::{n}"),
        DynamicKind::IfCondition => "if".into(),
        DynamicKind::ForIterable => "for".into(),
    }
}

fn names(c: &lunas_compiler::ResolvedComponent, indices: &[u32]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|i| {
            c.reactive_vars
                .iter()
                .find(|v| v.index == *i)
                .map(|v| v.name.clone())
        })
        .collect()
}
