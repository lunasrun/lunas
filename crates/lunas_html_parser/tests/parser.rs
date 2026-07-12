//! Tree-builder behavior tests, driven through the public `parse_html` API.

use lunas_html_parser::{parse_html, Dom, DomKind, Element, ElementKind, Node, ParseResult};
use lunas_span::Severity;

fn dom(source: &str) -> Dom {
    parse_html(source).dom
}

fn parse_result(source: &str) -> ParseResult {
    parse_html(source)
}

fn first_element(dom: &Dom) -> &Element {
    match &dom.children[0] {
        Node::Element(e) => e,
        other => panic!("expected element, got {:?}", other),
    }
}

// --- DomKind detection ---

#[test]
fn empty_string_is_empty() {
    assert_eq!(dom("").kind, DomKind::Empty);
}

#[test]
fn whitespace_only_is_empty() {
    assert_eq!(dom("   \n\t ").kind, DomKind::Empty);
}

#[test]
fn single_element_is_fragment() {
    assert_eq!(dom("<div></div>").kind, DomKind::Fragment);
}

#[test]
fn doctype_is_document() {
    assert_eq!(dom("<!DOCTYPE html><html></html>").kind, DomKind::Document);
}

#[test]
fn text_only_is_fragment() {
    assert_eq!(dom("hello").kind, DomKind::Fragment);
}

// --- Structure ---

#[test]
fn single_element() {
    let d = dom("<div></div>");
    assert_eq!(d.children.len(), 1);
    let e = first_element(&d);
    assert_eq!(e.name, "div");
    assert_eq!(e.kind, ElementKind::Normal);
    assert!(e.children.is_empty());
}

#[test]
fn nested_elements() {
    let d = dom("<div><span></span></div>");
    let div = first_element(&d);
    assert_eq!(div.children.len(), 1);
    match &div.children[0] {
        Node::Element(e) => assert_eq!(e.name, "span"),
        _ => panic!(),
    }
}

#[test]
fn siblings() {
    let d = dom("<a></a><b></b>");
    assert_eq!(d.children.len(), 2);
}

#[test]
fn deeply_nested_no_overflow() {
    let depth = 50;
    let mut src = String::new();
    for _ in 0..depth {
        src.push_str("<div>");
    }
    for _ in 0..depth {
        src.push_str("</div>");
    }
    let d = dom(&src);
    let mut cur = first_element(&d);
    let mut count = 1;
    while let Some(Node::Element(child)) = cur.children.first() {
        cur = child;
        count += 1;
    }
    assert_eq!(count, depth);
    assert!(parse_result(&src).diagnostics.is_empty());
}

#[test]
fn mixed_text_and_elements() {
    let d = dom("<p>hello <b>world</b>!</p>");
    let p = first_element(&d);
    assert_eq!(p.children.len(), 3);
    match &p.children[0] {
        Node::Text(t) => assert_eq!(t.value, "hello "),
        _ => panic!(),
    }
    match &p.children[1] {
        Node::Element(e) => assert_eq!(e.name, "b"),
        _ => panic!(),
    }
    match &p.children[2] {
        Node::Text(t) => assert_eq!(t.value, "!"),
        _ => panic!(),
    }
}

// --- Void elements ---

#[test]
fn all_void_elements() {
    for name in [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ] {
        let src = format!("<{}>", name);
        let d = dom(&src);
        let e = first_element(&d);
        assert_eq!(e.kind, ElementKind::Void, "{}", name);
        assert!(e.children.is_empty());
    }
}

#[test]
fn void_with_trailing_slash() {
    let d = dom("<br/>");
    assert_eq!(first_element(&d).kind, ElementKind::Void);
}

#[test]
fn void_does_not_capture_following() {
    let d = dom("<br>after");
    assert_eq!(d.children.len(), 2);
    assert_eq!(first_element(&d).kind, ElementKind::Void);
}

// --- Self-closing custom tag ---

#[test]
fn self_closing_component() {
    let d = dom("<Foo />");
    let e = first_element(&d);
    assert_eq!(e.name, "foo");
    assert_eq!(e.kind, ElementKind::Normal);
    assert!(e.children.is_empty());
}

// --- Raw text elements ---

#[test]
fn all_raw_text_elements() {
    for name in ["script", "style", "title", "textarea"] {
        let src = format!("<{0}>x<y></{0}>", name);
        let d = dom(&src);
        let e = first_element(&d);
        assert_eq!(e.children.len(), 1, "{}", name);
        match &e.children[0] {
            Node::Text(t) => assert_eq!(t.value, "x<y>"),
            _ => panic!("{}", name),
        }
    }
}

#[test]
fn script_with_markup_chars() {
    let d = dom("<script>if (a < b) { x(); } </div></script>");
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => assert_eq!(t.value, "if (a < b) { x(); } </div>"),
        _ => panic!(),
    }
}

#[test]
fn style_with_braces_and_gt() {
    let d = dom("<style>.a > .b { color: red; }</style>");
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => assert_eq!(t.value, ".a > .b { color: red; }"),
        _ => panic!(),
    }
}

#[test]
fn empty_script() {
    let d = dom("<script></script>");
    let e = first_element(&d);
    assert!(e.children.is_empty());
}

// --- Attributes ---

#[test]
fn boolean_attribute() {
    let d = dom("<input disabled>");
    let e = first_element(&d);
    assert_eq!(e.attributes[0].name, "disabled");
    assert!(e.attributes[0].value.is_none());
}

#[test]
fn double_quoted_attribute() {
    let d = dom("<a href=\"/x\"></a>");
    assert_eq!(first_element(&d).attributes[0].value.as_deref(), Some("/x"));
}

#[test]
fn single_quoted_attribute() {
    let d = dom("<a href='/x'></a>");
    assert_eq!(first_element(&d).attributes[0].value.as_deref(), Some("/x"));
}

#[test]
fn unquoted_attribute() {
    let d = dom("<a href=/x></a>");
    assert_eq!(first_element(&d).attributes[0].value.as_deref(), Some("/x"));
}

#[test]
fn empty_attribute_value() {
    let d = dom("<a x=\"\"></a>");
    assert_eq!(first_element(&d).attributes[0].value.as_deref(), Some(""));
}

#[test]
fn whitespace_around_eq() {
    let d = dom("<a x = \"v\"></a>");
    assert_eq!(first_element(&d).attributes[0].value.as_deref(), Some("v"));
}

#[test]
fn multiple_attributes() {
    let d = dom("<a id=\"x\" class='y' hidden></a>");
    let e = first_element(&d);
    assert_eq!(e.attributes.len(), 3);
    assert_eq!(e.attributes[0].name, "id");
    assert_eq!(e.attributes[1].name, "class");
    assert_eq!(e.attributes[2].name, "hidden");
    assert!(e.attributes[2].value.is_none());
}

#[test]
fn attribute_value_with_gt() {
    let d = dom("<a t=\"a>b\"></a>");
    assert_eq!(
        first_element(&d).attributes[0].value.as_deref(),
        Some("a>b")
    );
}

#[test]
fn duplicate_attribute_keeps_first_and_warns() {
    let r = parse_result("<a x=\"1\" x=\"2\"></a>");
    let e = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(e.attributes.len(), 1);
    assert_eq!(e.attributes[0].value.as_deref(), Some("1"));
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].severity, Severity::Warning);
}

// --- Comments ---

#[test]
fn normal_comment() {
    let d = dom("<!-- hi -->");
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.value, " hi "),
        _ => panic!(),
    }
}

#[test]
fn empty_comment() {
    let d = dom("<!---->");
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.value, ""),
        _ => panic!(),
    }
}

#[test]
fn unterminated_comment() {
    let d = dom("<!-- oops");
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.value, " oops"),
        _ => panic!(),
    }
}

#[test]
fn comment_between_elements() {
    let d = dom("<a></a><!-- c --><b></b>");
    assert_eq!(d.children.len(), 3);
    assert!(matches!(d.children[1], Node::Comment(_)));
}

// --- Error recovery ---

#[test]
fn mismatched_close_auto_closes_ancestor() {
    let r = parse_result("<div><span></div>");
    let div = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(div.name, "div");
    assert_eq!(div.children.len(), 1);
    match &div.children[0] {
        Node::Element(e) => assert_eq!(e.name, "span"),
        _ => panic!(),
    }
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning));
}

#[test]
fn stray_close_tag_errors() {
    let r = parse_result("<div></div></span>");
    assert_eq!(r.dom.children.len(), 1);
    assert!(r.diagnostics.iter().any(|d| d.is_error()));
}

#[test]
fn unclosed_element_at_eof_warns() {
    let r = parse_result("<div><p>text");
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning));
    let div = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(div.name, "div");
}

// --- Entities & unicode ---

#[test]
fn entities_pass_through() {
    let d = dom("<p>a &amp; b &lt;</p>");
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => assert_eq!(t.value, "a &amp; b &lt;"),
        _ => panic!(),
    }
}

#[test]
fn unicode_text_ranges() {
    let src = "<p>こんにちは</p>";
    let d = dom(src);
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => {
            assert_eq!(t.value, "こんにちは");
            assert_eq!(t.range.slice(src), Some("こんにちは"));
        }
        _ => panic!(),
    }
}

#[test]
fn unicode_in_attribute() {
    let d = dom("<a title=\"日本語\"></a>");
    assert_eq!(
        first_element(&d).attributes[0].value.as_deref(),
        Some("日本語")
    );
}

// --- Ranges round-trip ---

#[test]
fn element_range_round_trips() {
    let src = "<div class=\"x\">hi</div>";
    let d = dom(src);
    let e = first_element(&d);
    assert_eq!(e.range.slice(src), Some("<div class=\"x\">hi</div>"));
    assert_eq!(e.open_tag_range.slice(src), Some("<div class=\"x\">"));
}

#[test]
fn nested_range_round_trips() {
    let src = "<a><b>x</b></a>";
    let d = dom(src);
    let a = first_element(&d);
    assert_eq!(a.range.slice(src), Some("<a><b>x</b></a>"));
    match &a.children[0] {
        Node::Element(b) => assert_eq!(b.range.slice(src), Some("<b>x</b>")),
        _ => panic!(),
    }
}

#[test]
fn void_range_is_open_tag() {
    let src = "<img src=\"a\">";
    let d = dom(src);
    let e = first_element(&d);
    assert_eq!(e.range.slice(src), Some("<img src=\"a\">"));
    assert_eq!(e.range, e.open_tag_range);
}

#[test]
fn comment_range_round_trips() {
    let src = "<!-- c -->";
    let d = dom(src);
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.range.slice(src), Some("<!-- c -->")),
        _ => panic!(),
    }
}

// --- Realistic Lunas fragment ---

#[test]
fn lunas_template_fragment() {
    let src = "<div class=\"counter\">\n  <Button label=\"click\" disabled />\n  <span>{count}</span>\n</div>";
    let r = parse_result(src);
    assert_eq!(r.dom.kind, DomKind::Fragment);
    let div = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(div.name, "div");
    let button = div
        .children
        .iter()
        .find_map(|n| match n {
            Node::Element(e) if e.name == "button" => Some(e),
            _ => None,
        })
        .expect("button element");
    assert_eq!(button.attributes.len(), 2);
    assert_eq!(button.attributes[0].value.as_deref(), Some("click"));
    assert!(r.diagnostics.is_empty());
}

#[test]
fn curly_braces_in_text_pass_through() {
    let d = dom("<span>{count}</span>");
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => assert_eq!(t.value, "{count}"),
        _ => panic!(),
    }
}

// --- serde ---

#[test]
fn dom_serde_round_trip() {
    let d = dom("<div class=\"x\"><b>hi</b><!-- c --></div>");
    let json = serde_json::to_string(&d).expect("serialize");
    let back: Dom = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(d, back);
}

#[test]
fn multiple_roots_with_doctype_and_whitespace() {
    let d = dom("<!DOCTYPE html>\n<html><body></body></html>");
    assert_eq!(d.kind, DomKind::Document);
}

#[test]
fn auto_close_multiple_levels() {
    let r = parse_result("<a><b><c></a>");
    let a = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(a.name, "a");
    assert_eq!(a.children.len(), 1);
    let b = match &a.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(b.name, "b");
    assert!(r.diagnostics.len() >= 2);
}

#[test]
fn case_insensitive_tag_matching() {
    let r = parse_result("<DIV></div>");
    assert!(r.diagnostics.is_empty());
    assert_eq!(first_element(&r.dom).name, "div");
}
