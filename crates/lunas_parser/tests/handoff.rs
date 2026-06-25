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

#[test]
fn analysis_on_real_fixture_script() {
    use lunas_script::{assigned_identifiers, declared_bindings};
    let path = format!(
        "{}/tests/fixtures/counter-game.lun",
        env!("CARGO_MANIFEST_DIR")
    );
    let src = std::fs::read_to_string(&path).expect("read fixture");
    let (file, _) = parse(&src);
    let script = file.script.expect("script block");

    let declared = declared_bindings(&script.source.text).expect("analyze");
    for expected in ["count", "clear", "increment", "toggle", "interval"] {
        assert!(
            declared.contains(&expected.to_string()),
            "missing {expected} in {declared:?}"
        );
    }

    let assigned = assigned_identifiers(&script.source.text).expect("analyze");
    assert!(assigned.contains(&"count".to_string()));
    assert!(assigned.contains(&"interval".to_string()));
}

#[test]
fn find_all_references_to_a_binding_in_the_template() {
    // The LSP "find references / highlight" flow: locate every use of a binding
    // across all template expressions, in file-absolute positions.
    use lunas_script::referenced_identifiers_with_spans;

    let src = "\
html:
    <div :class=\"count > 0 ? 'on' : 'off'\">${ count }</div>
    <button @click=\"count = count + 1\">+</button>
script:
    let count = 0
";
    let (file, _) = parse(src);
    let html = file.html.unwrap();

    let mut refs = Vec::new();
    html.template.for_each_expression(|text, expr_range| {
        for (name, local) in referenced_identifiers_with_spans(text).unwrap_or_default() {
            if name == "count" {
                refs.push(local.shifted(expr_range.start()));
            }
        }
    });

    // count: once in :class, once in ${ count }, twice in @click = 4 uses.
    assert_eq!(refs.len(), 4, "{refs:?}");
    for r in &refs {
        assert_eq!(r.slice(src), Some("count"), "bad range {r:?}");
    }
}

#[test]
fn go_to_definition_template_binding_to_script_declaration() {
    // LSP go-to-definition: a binding used in the template resolves to its
    // declaration site in the script:, in file-absolute coordinates.
    use lunas_script::declared_bindings_with_spans;

    let src = "\
html:
    <div>${ count }</div>
script:
    let other = 1
    let count = 0
";
    let (file, _) = parse(src);
    let script = file.script.unwrap();

    let decls: Vec<(String, _)> = declared_bindings_with_spans(&script.source.text)
        .unwrap()
        .into_iter()
        .map(|(name, local)| (name, local.shifted(script.source.range.start())))
        .collect();

    let count_decl = decls
        .iter()
        .find(|(n, _)| n == "count")
        .map(|(_, r)| *r)
        .expect("count declared");

    assert_eq!(count_decl.slice(src), Some("count"));
    let lc = file.line_index.line_col(count_decl.start());
    assert_eq!(lc.line, 4); // "    let count = 0"
}

#[test]
fn reactivity_pipeline_on_todo_fixture() {
    use lunas_script::{analyze_script, free_identifiers};

    let path = format!("{}/tests/fixtures/todo.lunas", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("read");
    let (file, _) = parse(&src);

    let script = file.script.as_ref().unwrap();
    let analysis = analyze_script(&script.source.text).unwrap();
    for n in [
        "theme",
        "draft",
        "items",
        "remaining",
        "add",
        "complete",
        "onKey",
    ] {
        assert!(analysis.bindings.contains(&n.to_string()), "missing {n}");
    }
    let add_muts = analysis
        .function_mutations
        .iter()
        .find(|(n, _)| n == "add")
        .map(|(_, m)| m.clone())
        .unwrap();
    assert!(add_muts.contains(&"items".to_string()));
    assert!(add_muts.contains(&"draft".to_string()));

    let mut found_remaining_dep = false;
    file.html
        .as_ref()
        .unwrap()
        .template
        .for_each_expression(|text, _| {
            let deps: Vec<String> = free_identifiers(text)
                .unwrap_or_default()
                .into_iter()
                .filter(|id| analysis.bindings.contains(id))
                .collect();
            if deps.contains(&"remaining".to_string()) {
                found_remaining_dep = true;
            }
        });
    assert!(
        found_remaining_dep,
        "expected an expression depending on `remaining`"
    );
}
