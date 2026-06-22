//! Cross-crate handoff: `lunas_parser` stores embedded JS (the `:for` header,
//! interpolation expressions) as raw text + spans; the downstream consumer
//! (here standing in for the future orchestrator) parses it with `lunas_script`.
//! This validates that the raw text the parser captures is exactly what the JS
//! tooling needs.

use lunas_parser::{parse, TemplateAttr, TemplateNode};
use lunas_script::{parse_for, ForKind};

fn for_headers(nodes: &[TemplateNode], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            TemplateNode::For(f) => {
                out.push(f.header.text.clone());
                for_headers(std::slice::from_ref(&f.body), out);
            }
            TemplateNode::If(c) => {
                for b in &c.branches {
                    for_headers(std::slice::from_ref(&b.body), out);
                }
            }
            TemplateNode::Element(e) => for_headers(&e.children, out),
            TemplateNode::Component(c) => for_headers(&c.children, out),
            _ => {}
        }
    }
}

#[test]
fn for_header_handoff_to_lunas_script() {
    let src =
        "html:\n    <ul>\n        <li :for=\"[i, v] of items.entries()\">${v}</li>\n    </ul>\n";
    let (file, diags) = parse(src);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");

    let mut headers = Vec::new();
    for_headers(&file.html.unwrap().template.nodes, &mut headers);
    assert_eq!(headers.len(), 1);

    // The raw header the parser captured parses cleanly with lunas_script.
    let parsed = parse_for(&headers[0]).expect("valid for header");
    assert_eq!(parsed.kind, ForKind::Of);
    assert_eq!(parsed.binding, "[i, v]");
    assert_eq!(parsed.iterable, "items.entries()");
}

#[test]
fn plain_for_header_handoff() {
    let src = "html:\n    <li :for=\"item of list\">${item}</li>\n";
    let (file, _) = parse(src);
    let mut headers = Vec::new();
    for_headers(&file.html.unwrap().template.nodes, &mut headers);
    let parsed = parse_for(&headers[0]).expect("valid");
    assert_eq!(parsed.binding, "item");
    assert_eq!(parsed.iterable, "list");
}

#[test]
fn reactivity_dependency_flow_end_to_end() {
    // The full intended flow across all three crates (what the orchestrator will
    // do): parse -> component bindings from the script -> for each template
    // expression, its free identifiers intersected with the bindings are the
    // reactive dependencies.
    use lunas_parser::TextSegment;
    use lunas_script::{declared_bindings, free_identifiers};

    let src = "\
@input label:string
html:
    <div>${ count + label }</div>
    <button @click=\"count++\">+</button>
script:
    let count = 0
    let unused = 0
";
    let (file, diags) = parse(src);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");

    // Component bindings = script declarations + @input props.
    let script = file.script.as_ref().unwrap();
    let mut bindings = declared_bindings(&script.source.text).unwrap();
    bindings.push("label".to_string()); // the @input prop
    let is_binding = |name: &str| bindings.iter().any(|b| b == name);

    // Collect the reactive dependency set of the `${ count + label }` interpolation.
    let html = file.html.unwrap();
    let mut interp_deps = Vec::new();
    html.template.visit(&mut |n| {
        if let TemplateNode::Text(t) = n {
            for seg in &t.segments {
                if let TextSegment::Interpolation(i) = seg {
                    for id in free_identifiers(&i.expr).unwrap() {
                        if is_binding(&id) && !interp_deps.contains(&id) {
                            interp_deps.push(id);
                        }
                    }
                }
            }
        }
    });
    interp_deps.sort();
    assert_eq!(interp_deps, ["count", "label"]);

    // The @click handler mutates `count` (what to re-render on click).
    let mut handler_mutations = Vec::new();
    html.template.visit(&mut |n| {
        let attrs = match n {
            TemplateNode::Element(e) => &e.attrs,
            _ => return,
        };
        for a in attrs {
            if let TemplateAttr::Event { handler, .. } = a {
                for id in lunas_script::assigned_identifiers(&handler.text).unwrap() {
                    if is_binding(&id) {
                        handler_mutations.push(id);
                    }
                }
            }
        }
    });
    assert_eq!(handler_mutations, ["count"]);
}
