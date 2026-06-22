//! Tests for the binding-aware template layer, driven through `parse`.

use lunas_parser::{
    parse, BranchKind, Diagnostic, Severity, TemplateAttr, TemplateElement, TemplateNode,
    TextSegment,
};

fn nodes(src: &str) -> Vec<TemplateNode> {
    let (file, diags) = parse(src);
    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "unexpected errors: {:?}",
        diags
    );
    file.html.expect("html block").template.nodes
}

fn parse_template(src: &str) -> (Vec<TemplateNode>, Vec<Diagnostic>) {
    let (file, diags) = parse(src);
    (file.html.expect("html block").template.nodes, diags)
}

/// All element nodes among `nodes`, skipping whitespace text etc.
fn elements(nodes: &[TemplateNode]) -> Vec<&TemplateElement> {
    nodes
        .iter()
        .filter_map(|n| match n {
            TemplateNode::Element(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn first_element(nodes: &[TemplateNode]) -> &TemplateElement {
    elements(nodes).into_iter().next().expect("an element")
}

/// Wrap an html body line so it forms a valid `.lunas` file.
fn html(body: &str) -> String {
    format!("html:\n    {}\n", body)
}

// --- Interpolation in text ---

#[test]
fn single_interpolation() {
    let ns = nodes(&html("<div>${count}</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, "count"),
        other => panic!("expected interpolation, got {:?}", other),
    }
}

#[test]
fn interpolation_between_literals() {
    let ns = nodes(&html("<div>a ${x} b</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    assert_eq!(text.segments.len(), 3);
    match &text.segments[0] {
        TextSegment::Literal { text, .. } => assert_eq!(text, "a "),
        _ => panic!(),
    }
    match &text.segments[1] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, "x"),
        _ => panic!(),
    }
    match &text.segments[2] {
        TextSegment::Literal { text, .. } => assert_eq!(text, " b"),
        _ => panic!(),
    }
}

#[test]
fn multiple_interpolations() {
    let ns = nodes(&html("<div>${a}${b}</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    let exprs: Vec<&str> = text
        .segments
        .iter()
        .filter_map(|s| match s {
            TextSegment::Interpolation(i) => Some(i.expr.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(exprs, vec!["a", "b"]);
}

#[test]
fn interpolation_with_ternary() {
    let ns = nodes(&html("<div>${ on ? 'a' : 'b' }</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, " on ? 'a' : 'b' "),
        _ => panic!(),
    }
}

#[test]
fn interpolation_balances_nested_braces() {
    let ns = nodes(&html("<div>${ {a:1}.a }</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, " {a:1}.a "),
        other => panic!("got {:?}", other),
    }
}

#[test]
fn interpolation_brace_inside_string_literal() {
    let ns = nodes(&html("<div>${ \"}\" }</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, " \"}\" "),
        other => panic!("got {:?}", other),
    }
}

#[test]
fn interpolation_template_literal_substitution() {
    let ns = nodes(&html("<div>${ `a${b}c` }</div>"));
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => assert_eq!(i.expr, " `a${b}c` "),
        other => panic!("got {:?}", other),
    }
}

#[test]
fn interpolation_expr_range_is_absolute() {
    let src = html("<div>${count}</div>");
    let (file, _) = parse(&src);
    let ns = file.html.as_ref().unwrap().template.nodes.clone();
    let div = first_element(&ns);
    let text = match &div.children[0] {
        TemplateNode::Text(t) => t,
        _ => panic!(),
    };
    match &text.segments[0] {
        TextSegment::Interpolation(i) => {
            assert_eq!(i.expr_range.slice(&src), Some("count"));
            assert_eq!(i.range.slice(&src), Some("${count}"));
        }
        _ => panic!(),
    }
}

#[test]
fn unterminated_interpolation_errors_and_recovers() {
    let (ns, diags) = parse_template(&html("<div>${count</div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("unterminated")));
    // Tree still built.
    assert!(!elements(&ns).is_empty());
}

#[test]
fn empty_interpolation_warns() {
    let (_ns, diags) = parse_template(&html("<div>${}</div>"));
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("empty interpolation")));
}

// --- Attribute classification ---

#[test]
fn bound_attribute() {
    let ns = nodes(&html("<input :value=\"title\" />"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Bound { name, expr, .. } => {
            assert_eq!(name, "value");
            assert_eq!(expr.text, "title");
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn two_way_attribute() {
    let ns = nodes(&html("<input ::value=\"title\" />"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::TwoWay { name, lvalue, .. } => {
            assert_eq!(name, "value");
            assert_eq!(lvalue.text, "title");
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn event_attribute() {
    let ns = nodes(&html("<button @click=\"toggle\">x</button>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Event { event, handler, .. } => {
            assert_eq!(event, "click");
            assert_eq!(handler.text, "toggle");
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn static_attribute() {
    let ns = nodes(&html("<div class=\"box\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { name, value, .. } => {
            assert_eq!(name, "class");
            let v = value.as_ref().expect("value");
            match &v.segments[0] {
                TextSegment::Literal { text, .. } => assert_eq!(text, "box"),
                _ => panic!(),
            }
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn static_attribute_with_interpolation() {
    let ns = nodes(&html("<div class=\"a ${cls} b\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let v = value.as_ref().unwrap();
            assert_eq!(v.segments.len(), 3);
        }
        _ => panic!(),
    }
}

#[test]
fn boolean_attribute_has_no_value() {
    let ns = nodes(&html("<input disabled />"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { name, value, .. } => {
            assert_eq!(name, "disabled");
            assert!(value.is_none());
        }
        _ => panic!(),
    }
}

#[test]
fn reserved_bound_attribute_errors() {
    let (_ns, diags) = parse_template(&html("<div :innerHtml=\"x\"></div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("not supported")));
}

// --- Control flow: if cascade ---

#[test]
fn if_only() {
    let ns = nodes(&html("<div :if=\"a\">x</div>"));
    let chain = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("if chain");
    assert_eq!(chain.branches.len(), 1);
    assert_eq!(chain.branches[0].kind, BranchKind::If);
    assert_eq!(chain.branches[0].condition.as_ref().unwrap().text, "a");
}

#[test]
fn if_elseif_else_grouped() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :elseif=\"b\">2</div>\n    <div :else>3</div>\n";
    let ns = nodes(src);
    let chain = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("if chain");
    assert_eq!(chain.branches.len(), 3);
    assert_eq!(chain.branches[0].kind, BranchKind::If);
    assert_eq!(chain.branches[1].kind, BranchKind::ElseIf);
    assert_eq!(chain.branches[2].kind, BranchKind::Else);
    assert!(chain.branches[2].condition.is_none());
    // Exactly one top-level If node — the cascade is grouped, not three nodes.
    assert_eq!(
        ns.iter()
            .filter(|n| matches!(n, TemplateNode::If(_)))
            .count(),
        1
    );
}

#[test]
fn else_without_if_errors() {
    let (_ns, diags) = parse_template(&html("<div :else>x</div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("without a matching")));
}

#[test]
fn elseif_breaks_on_unrelated_element() {
    // An if followed by a plain element (not elseif/else) is a 1-branch chain,
    // and the plain element stays a sibling.
    let src = "html:\n    <div :if=\"a\">1</div>\n    <p>plain</p>\n";
    let ns = nodes(src);
    let if_count = ns
        .iter()
        .filter(|n| matches!(n, TemplateNode::If(_)))
        .count();
    let p = elements(&ns).into_iter().find(|e| e.name == "p");
    assert_eq!(if_count, 1);
    assert!(p.is_some());
}

// --- Control flow: for ---

#[test]
fn for_block() {
    let ns = nodes(&html("<li :for=\"item of items\">${item}</li>"));
    let for_block = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::For(f) => Some(f),
            _ => None,
        })
        .expect("for block");
    assert_eq!(for_block.header.text, "item of items");
    assert!(matches!(*for_block.body, TemplateNode::Element(_)));
}

#[test]
fn for_with_if_nests_for_outside() {
    let ns = nodes(&html("<li :for=\"x of xs\" :if=\"x\">y</li>"));
    let for_block = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::For(f) => Some(f),
            _ => None,
        })
        .expect("for block");
    // :for outer, :if inner.
    match &*for_block.body {
        TemplateNode::If(chain) => {
            assert_eq!(chain.branches.len(), 1);
            assert_eq!(chain.branches[0].condition.as_ref().unwrap().text, "x");
        }
        other => panic!("expected inner if, got {:?}", other),
    }
}

#[test]
fn for_empty_header_errors() {
    let (_ns, diags) = parse_template(&html("<li :for=\"\">x</li>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("`:for`")));
}

// --- Components ---

#[test]
fn component_resolved_from_use_table() {
    let src =
        "@use()\nButton from \"./Button\"\n\nhtml:\n    <Button label=\"go\" :count=\"n\" />\n";
    let ns = nodes(src);
    let comp = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::Component(c) => Some(c),
            _ => None,
        })
        .expect("component");
    assert_eq!(comp.name, "Button");
    // static prop + bound prop
    assert!(comp
        .props
        .iter()
        .any(|p| matches!(p, TemplateAttr::Static { name, .. } if name == "label")));
    assert!(comp
        .props
        .iter()
        .any(|p| matches!(p, TemplateAttr::Bound { name, .. } if name == "count")));
}

#[test]
fn pascalcase_without_use_is_element() {
    // Not in @use → treated as a plain element (lowercased name), not component.
    let ns = nodes(&html("<Widget />"));
    assert!(ns.iter().all(|n| !matches!(n, TemplateNode::Component(_))));
    assert_eq!(first_element(&ns).name, "widget");
}

#[test]
fn lowercase_tag_is_never_component() {
    let src = "@use()\nButton from \"./Button\"\n\nhtml:\n    <button>x</button>\n";
    let ns = nodes(src);
    assert!(ns.iter().all(|n| !matches!(n, TemplateNode::Component(_))));
    assert_eq!(first_element(&ns).name, "button");
}

// --- serde ---

#[test]
fn template_serde_round_trip() {
    let ns = nodes(&html("<div :if=\"a\">${x}</div>"));
    let json = serde_json::to_string(&ns).expect("serialize");
    let back: Vec<TemplateNode> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(ns, back);
}

// --- Nesting & control flow on components (hardening) ---

#[test]
fn if_on_component() {
    let src = "@use()\nCard from \"./Card\"\n\nhtml:\n    <Card :if=\"show\" />\n";
    let ns = nodes(src);
    let chain = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("if chain");
    assert!(matches!(
        *chain.branches[0].body,
        TemplateNode::Component(_)
    ));
}

#[test]
fn for_on_component() {
    let src = "@use()\nRow from \"./Row\"\n\nhtml:\n    <Row :for=\"r of rows\" :data=\"r\" />\n";
    let ns = nodes(src);
    let f = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::For(f) => Some(f),
            _ => None,
        })
        .expect("for block");
    assert_eq!(f.header.text, "r of rows");
    assert!(matches!(*f.body, TemplateNode::Component(_)));
}

#[test]
fn nested_for_then_if_child() {
    let src = "html:\n    <ul :for=\"x of xs\"><li :if=\"x\">${x}</li></ul>\n";
    let ns = nodes(src);
    let f = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::For(f) => Some(f),
            _ => None,
        })
        .expect("for block");
    let ul = match &*f.body {
        TemplateNode::Element(e) => e,
        other => panic!("expected ul element, got {:?}", other),
    };
    assert_eq!(ul.name, "ul");
    let inner_if = ul.children.iter().any(|n| matches!(n, TemplateNode::If(_)));
    assert!(inner_if, "expected nested if inside the for body");
}

#[test]
fn multiple_interpolations_in_static_attr() {
    let ns = nodes(&html("<div class=\"${a} ${b}\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let segs = &value.as_ref().unwrap().segments;
            let interps = segs
                .iter()
                .filter(|s| matches!(s, TextSegment::Interpolation(_)))
                .count();
            assert_eq!(interps, 2);
        }
        _ => panic!(),
    }
}

#[test]
fn cascade_of_components() {
    let src = "@use()\nA from \"./A\"\n\nhtml:\n    <A :if=\"p\" />\n    <A :else />\n";
    let ns = nodes(src);
    let chain = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("if chain");
    assert_eq!(chain.branches.len(), 2);
    assert!(matches!(
        *chain.branches[0].body,
        TemplateNode::Component(_)
    ));
    assert!(matches!(
        *chain.branches[1].body,
        TemplateNode::Component(_)
    ));
}

#[test]
fn deeply_nested_elements_preserve_interpolation() {
    let ns = nodes(&html("<div><section><p>${deep}</p></section></div>"));
    let div = first_element(&ns);
    fn find_interp(nodes: &[TemplateNode]) -> bool {
        nodes.iter().any(|n| match n {
            TemplateNode::Text(t) => t
                .segments
                .iter()
                .any(|s| matches!(s, TextSegment::Interpolation(_))),
            TemplateNode::Element(e) => find_interp(&e.children),
            _ => false,
        })
    }
    assert!(find_interp(&div.children));
}

// --- Components: children, multiple @use, quoting (hardening) ---

#[test]
fn component_with_children() {
    let src = "@use()\nCard from \"./Card\"\n\nhtml:\n    <Card><p>slot ${x}</p></Card>\n";
    let ns = nodes(src);
    let comp = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::Component(c) => Some(c),
            _ => None,
        })
        .expect("component");
    // The <p> child (and its interpolation) are preserved under the component.
    let has_p = comp
        .children
        .iter()
        .any(|n| matches!(n, TemplateNode::Element(e) if e.name == "p"));
    assert!(has_p, "expected <p> child under the component");
}

#[test]
fn multiple_use_components_both_resolve() {
    let src = "@use A from \"./A\"\n@use B from './B'\n\nhtml:\n    <A /><B />\n";
    let (file, diags) = parse(src);
    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "{:?}",
        diags
    );
    let html = file.html.unwrap();
    let comps: Vec<&str> = html
        .template
        .nodes
        .iter()
        .filter_map(|n| match n {
            TemplateNode::Component(c) => Some(c.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(comps.contains(&"A"));
    assert!(comps.contains(&"B"));
}

// --- Cascade boundary behavior (documented decisions) ---

#[test]
fn non_whitespace_text_breaks_cascade() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    sep\n    <div :else>2</div>\n";
    let (ns, diags) = parse_template(src);
    let chains = ns
        .iter()
        .filter(|n| matches!(n, TemplateNode::If(_)))
        .count();
    assert_eq!(chains, 1);
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("without a matching")));
}

#[test]
fn comment_breaks_cascade() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <!-- c -->\n    <div :else>2</div>\n";
    let (_ns, diags) = parse_template(src);
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("without a matching")));
}

#[test]
fn whitespace_only_text_keeps_cascade() {
    let src = "html:\n    <div :if=\"a\">1</div>\n\n\n    <div :else>2</div>\n";
    let ns = nodes(src);
    let chain = ns
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("if chain");
    assert_eq!(chain.branches.len(), 2);
}

// --- Template::visit traversal helper ---

#[test]
fn visit_reaches_all_nested_nodes() {
    use lunas_parser::Template;
    let src = "html:\n    <ul :for=\"x of xs\"><li :if=\"x\">${a}</li><li :else>${b}</li></ul>\n";
    let (file, _) = parse(src);
    let template: Template = file.html.unwrap().template;

    let mut interpolations = 0;
    let mut elements = 0;
    template.visit(&mut |n| match n {
        TemplateNode::Element(_) => elements += 1,
        TemplateNode::Text(t) => {
            interpolations += t
                .segments
                .iter()
                .filter(|s| matches!(s, TextSegment::Interpolation(_)))
                .count();
        }
        _ => {}
    });

    // ul + two li elements are all reached through For -> If -> branch bodies.
    assert_eq!(elements, 3);
    // ${a} and ${b} inside the two branches.
    assert_eq!(interpolations, 2);
}
