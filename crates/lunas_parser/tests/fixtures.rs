//! End-to-end parsing of real `.lunas`/`.lun` files (vendored from the original
//! formatter test suite on `main`). These exercise the whole stack — Pest
//! grammar, HTML parser, and template layer — against actual Lunas source.

use lunas_parser::{parse, Severity, TemplateNode};

fn parse_fixture(name: &str) -> lunas_parser::ParsedFile {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    let src = std::fs::read_to_string(&path).expect("read fixture");
    let (file, diags) = parse(&src);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{name} produced errors: {:?}", errors);
    file
}

fn flatten<'a>(nodes: &'a [TemplateNode], out: &mut Vec<&'a TemplateNode>) {
    for n in nodes {
        out.push(n);
        match n {
            TemplateNode::Element(e) => flatten(&e.children, out),
            TemplateNode::Component(c) => flatten(&c.children, out),
            TemplateNode::If(c) => {
                for b in &c.branches {
                    flatten(std::slice::from_ref(&b.body), out);
                }
            }
            TemplateNode::For(f) => flatten(std::slice::from_ref(&f.body), out),
            _ => {}
        }
    }
}

#[test]
fn counter_game_parses() {
    let file = parse_fixture("counter-game.lun");
    assert!(file.html.is_some());
    assert!(file.script.is_some());
    assert!(file.style.is_some());

    let html = file.html.as_ref().unwrap();
    let mut all = Vec::new();
    flatten(&html.template.nodes, &mut all);

    // Two independent `:if` blocks (not a cascade): success / failure.
    let if_blocks = all
        .iter()
        .filter(|n| matches!(n, TemplateNode::If(_)))
        .count();
    assert_eq!(if_blocks, 2, "expected two standalone :if blocks");

    // The `@click="toggle"` handler is present.
    let has_click = all.iter().any(|n| match n {
        TemplateNode::Element(e) => e.attrs.iter().any(
            |a| matches!(a, lunas_parser::TemplateAttr::Event { event, .. } if event == "click"),
        ),
        _ => false,
    });
    assert!(has_click, "expected an @click handler");

    // `${count}` and the ternary interpolation both parsed.
    let interpolations = all
        .iter()
        .filter_map(|n| match n {
            TemplateNode::Text(t) => Some(t),
            _ => None,
        })
        .flat_map(|t| &t.segments)
        .filter(|s| matches!(s, lunas_parser::TextSegment::Interpolation(_)))
        .count();
    assert!(
        interpolations >= 2,
        "expected interpolations, got {interpolations}"
    );
}

#[test]
fn pass_value_parses() {
    let file = parse_fixture("pass-value.lun");
    // Two inline `@input name:type` prop declarations.
    let inputs: Vec<_> = file
        .directives
        .iter()
        .filter_map(|d| match d {
            lunas_parser::Directive::Input(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(inputs.len(), 2, "expected two @input directives");
    assert_eq!(inputs[0].name, "message1");
    assert_eq!(inputs[0].type_annotation.as_deref(), Some("string"));
    assert_eq!(inputs[1].name, "message2");
}
