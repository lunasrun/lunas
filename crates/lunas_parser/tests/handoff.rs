//! Cross-crate handoff: `lunas_parser` stores embedded JS (the `:for` header,
//! interpolation expressions) as raw text + spans; the downstream consumer
//! (here standing in for the future orchestrator) parses it with `lunas_script`.
//! This validates that the raw text the parser captures is exactly what the JS
//! tooling needs.

use lunas_parser::{parse, TemplateNode};
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
