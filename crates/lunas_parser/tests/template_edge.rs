//! Edge cases for control-flow grouping (`:if`/`:elseif`/`:else` cascades and
//! `:for` loops) and structural template shapes, driven through `parse`. These
//! complement `tests/template.rs`; malformed control flow must yield
//! diagnostics, never panic.

use lunas_parser::{parse, BranchKind, Diagnostic, IfChain, Severity, TemplateNode};

fn html(body: &str) -> String {
    format!("html:\n    {}\n", body)
}

fn parse_template(src: &str) -> (Vec<TemplateNode>, Vec<Diagnostic>) {
    let (file, diags) = parse(src);
    (file.html.expect("html block").template.nodes, diags)
}

fn nodes_ok(src: &str) -> Vec<TemplateNode> {
    let (ns, diags) = parse_template(src);
    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "unexpected errors: {:?}",
        diags
    );
    ns
}

fn find_if(nodes: &[TemplateNode]) -> Option<&IfChain> {
    nodes.iter().find_map(|n| match n {
        TemplateNode::If(c) => Some(c),
        _ => None,
    })
}

fn find_for(nodes: &[TemplateNode]) -> Option<&lunas_parser::ForBlock> {
    nodes.iter().find_map(|n| match n {
        TemplateNode::For(f) => Some(f),
        _ => None,
    })
}

fn has_error(diags: &[Diagnostic], needle: &str) -> bool {
    diags
        .iter()
        .any(|d| d.is_error() && d.message.contains(needle))
}

// --- :if cascades ---

#[test]
fn if_elseif_elseif_else_four_branches() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :elseif=\"b\">2</div>\n    <div :elseif=\"c\">3</div>\n    <div :else>4</div>\n";
    let ns = nodes_ok(src);
    let chain = find_if(&ns).expect("chain");
    assert_eq!(chain.branches.len(), 4);
    assert_eq!(chain.branches[0].kind, BranchKind::If);
    assert_eq!(chain.branches[1].kind, BranchKind::ElseIf);
    assert_eq!(chain.branches[2].kind, BranchKind::ElseIf);
    assert_eq!(chain.branches[3].kind, BranchKind::Else);
    assert!(chain.branches[3].condition.is_none());
    // Only one grouped If node at top level.
    assert_eq!(
        ns.iter()
            .filter(|n| matches!(n, TemplateNode::If(_)))
            .count(),
        1
    );
}

#[test]
fn if_then_else_no_elseif() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :else>2</div>\n";
    let ns = nodes_ok(src);
    let chain = find_if(&ns).expect("chain");
    assert_eq!(chain.branches.len(), 2);
    assert_eq!(chain.branches[1].kind, BranchKind::Else);
}

#[test]
fn second_else_after_else_is_orphan() {
    // Cascade ends at the first :else; a trailing :else has no matching :if.
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :else>2</div>\n    <div :else>3</div>\n";
    let (ns, diags) = parse_template(src);
    assert_eq!(find_if(&ns).unwrap().branches.len(), 2);
    assert!(has_error(&diags, "without a matching"));
}

#[test]
fn elseif_after_else_is_orphan() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :else>2</div>\n    <div :elseif=\"c\">3</div>\n";
    let (_ns, diags) = parse_template(src);
    assert!(has_error(&diags, "without a matching"));
}

#[test]
fn orphan_elseif_errors() {
    let (_ns, diags) = parse_template(&html("<div :elseif=\"b\">x</div>"));
    assert!(has_error(&diags, "without a matching"));
}

#[test]
fn orphan_else_errors() {
    let (_ns, diags) = parse_template(&html("<div :else>x</div>"));
    assert!(has_error(&diags, "without a matching"));
}

#[test]
fn if_without_condition_errors() {
    let (_ns, diags) = parse_template(&html("<div :if=\"\">x</div>"));
    assert!(has_error(&diags, "expects a condition"));
}

#[test]
fn elseif_without_condition_errors() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :elseif=\"\">2</div>\n";
    let (_ns, diags) = parse_template(src);
    assert!(has_error(&diags, "expects a condition"));
}

#[test]
fn else_with_value_warns() {
    let (_ns, diags) = parse_template(&{
        let mut s = String::from("html:\n    <div :if=\"a\">1</div>\n");
        s.push_str("    <div :else=\"nope\">2</div>\n");
        s
    });
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("does not take a value")));
}

#[test]
fn two_independent_if_chains() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <p>sep</p>\n    <div :if=\"b\">2</div>\n";
    let ns = nodes_ok(src);
    let chains = ns
        .iter()
        .filter(|n| matches!(n, TemplateNode::If(_)))
        .count();
    assert_eq!(chains, 2);
}

#[test]
fn cascade_ranges_cover_all_branches() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <div :else>2</div>\n";
    let (file, _) = parse(src);
    let ns = file.html.unwrap().template.nodes;
    let chain = find_if(&ns).unwrap();
    // The chain range covers from the first branch start to the last branch end.
    assert!(chain.range.start() <= chain.branches[0].range.start());
    assert!(chain.range.end() >= chain.branches.last().unwrap().range.end());
}

// --- :if + :for on the same element ---

#[test]
fn for_wraps_if_on_same_element() {
    let ns = nodes_ok(&html("<li :for=\"x of xs\" :if=\"x.ok\">${x}</li>"));
    let f = find_for(&ns).expect("for");
    match &*f.body {
        TemplateNode::If(chain) => {
            assert_eq!(chain.branches.len(), 1);
            assert_eq!(chain.branches[0].condition.as_ref().unwrap().text, "x.ok");
        }
        other => panic!("expected inner if, got {other:?}"),
    }
}

#[test]
fn for_with_elseif_on_same_element_errors() {
    // `:for` cannot combine with `:elseif`/`:else`.
    let (_ns, diags) = parse_template(&html("<li :for=\"x of xs\" :else>y</li>"));
    assert!(diags.iter().any(|d| d.is_error()));
}

// --- :for headers ---

#[test]
fn for_of_header() {
    let ns = nodes_ok(&html("<li :for=\"item of items\">${item}</li>"));
    assert_eq!(find_for(&ns).unwrap().header.text, "item of items");
}

#[test]
fn for_in_header() {
    let ns = nodes_ok(&html("<li :for=\"key in obj\">${key}</li>"));
    assert_eq!(find_for(&ns).unwrap().header.text, "key in obj");
}

#[test]
fn for_destructuring_header() {
    let ns = nodes_ok(&html(
        "<li :for=\"const [i, v] of data.entries()\">${v}</li>",
    ));
    assert_eq!(
        find_for(&ns).unwrap().header.text,
        "const [i, v] of data.entries()"
    );
}

#[test]
fn for_object_destructuring_header() {
    let ns = nodes_ok(&html("<li :for=\"const {id, name} of rows\">${name}</li>"));
    assert_eq!(
        find_for(&ns).unwrap().header.text,
        "const {id, name} of rows"
    );
}

#[test]
fn for_header_kept_raw_not_split() {
    // The header text is stored verbatim; splitting is the downstream concern.
    let ns = nodes_ok(&html("<li :for=\"x, i of items\">y</li>"));
    assert_eq!(find_for(&ns).unwrap().header.text, "x, i of items");
}

#[test]
fn for_empty_header_errors() {
    let (_ns, diags) = parse_template(&html("<li :for=\"\">x</li>"));
    assert!(has_error(&diags, "`:for`"));
}

#[test]
fn for_whitespace_only_header_errors() {
    let (_ns, diags) = parse_template(&html("<li :for=\"   \">x</li>"));
    assert!(has_error(&diags, "`:for`"));
}

#[test]
fn for_missing_value_errors() {
    let (_ns, diags) = parse_template(&html("<li :for>x</li>"));
    assert!(has_error(&diags, "`:for`"));
}

#[test]
fn for_header_range_slices_back() {
    let src = html("<li :for=\"item of items\">y</li>");
    let (file, _) = parse(&src);
    let ns = file.html.unwrap().template.nodes;
    let f = find_for(&ns).unwrap();
    assert_eq!(f.header.range.slice(&src), Some("item of items"));
}

// --- Nesting ---

#[test]
fn for_inside_if_body() {
    let src = "html:\n    <div :if=\"show\"><ul :for=\"x of xs\"><li>${x}</li></ul></div>\n";
    let ns = nodes_ok(src);
    let chain = find_if(&ns).expect("if");
    let div = match &*chain.branches[0].body {
        TemplateNode::Element(e) => e,
        other => panic!("expected div, got {other:?}"),
    };
    assert!(div
        .children
        .iter()
        .any(|n| matches!(n, TemplateNode::For(_))));
}

#[test]
fn if_cascade_inside_for_body() {
    let src = "html:\n    <ul :for=\"x of xs\"><li :if=\"x.a\">a</li><li :elseif=\"x.b\">b</li><li :else>c</li></ul>\n";
    let ns = nodes_ok(src);
    let f = find_for(&ns).expect("for");
    let ul = match &*f.body {
        TemplateNode::Element(e) => e,
        other => panic!("expected ul, got {other:?}"),
    };
    let inner = ul
        .children
        .iter()
        .find_map(|n| match n {
            TemplateNode::If(c) => Some(c),
            _ => None,
        })
        .expect("inner if chain");
    assert_eq!(inner.branches.len(), 3);
}

#[test]
fn triple_nested_for() {
    let src =
        "html:\n    <a :for=\"i of xs\"><b :for=\"j of i\"><c :for=\"k of j\">${k}</c></b></a>\n";
    let ns = nodes_ok(src);
    let outer = find_for(&ns).expect("outer for");
    let a = match &*outer.body {
        TemplateNode::Element(e) => e,
        _ => panic!("expected a"),
    };
    let mid = a
        .children
        .iter()
        .find_map(|n| match n {
            TemplateNode::For(f) => Some(f),
            _ => None,
        })
        .expect("middle for");
    let b = match &*mid.body {
        TemplateNode::Element(e) => e,
        _ => panic!("expected b"),
    };
    assert!(b.children.iter().any(|n| matches!(n, TemplateNode::For(_))));
}

// --- Comments & whitespace boundaries ---

#[test]
fn comment_between_if_and_else_breaks_cascade() {
    let src = "html:\n    <div :if=\"a\">1</div>\n    <!-- c -->\n    <div :else>2</div>\n";
    let (_ns, diags) = parse_template(src);
    assert!(has_error(&diags, "without a matching"));
}

#[test]
fn multiple_blank_lines_keep_cascade() {
    let src = "html:\n    <div :if=\"a\">1</div>\n\n\n\n    <div :elseif=\"b\">2</div>\n";
    let ns = nodes_ok(src);
    assert_eq!(find_if(&ns).unwrap().branches.len(), 2);
}

// --- serde round-trip on rich control flow ---

#[test]
fn control_flow_serde_round_trip() {
    let src = "html:\n    <ul :for=\"x of xs\"><li :if=\"x.a\">${x.v}</li><li :else>-</li></ul>\n";
    let (file, _) = parse(src);
    let ns = file.html.unwrap().template.nodes;
    let json = serde_json::to_string(&ns).expect("serialize");
    let back: Vec<TemplateNode> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(ns, back);
}
