//! Edge cases around attribute lexing that are easy to get subtly wrong.

use lunas_html_parser::{parse_html, Element, Node};

fn first_element(source: &str) -> Element {
    let dom = parse_html(source).dom;
    dom.children
        .into_iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .expect("an element")
}

fn attrs(source: &str) -> Vec<(String, Option<String>)> {
    first_element(source)
        .attributes
        .into_iter()
        .map(|a| (a.name, a.value))
        .collect()
}

#[test]
fn value_then_self_close_no_space() {
    // `"text"/>` — the quote ends the value and `/>` self-closes.
    let a = attrs("<input type=\"text\"/>");
    assert_eq!(a, [("type".into(), Some("text".into()))]);
}

#[test]
fn quoted_attrs_no_separating_space() {
    // `b="c"d="e"` — a quote-terminated value is immediately followed by the
    // next attribute with no whitespace.
    let a = attrs("<a b=\"c\"d=\"e\">");
    assert_eq!(
        a,
        [
            ("b".into(), Some("c".into())),
            ("d".into(), Some("e".into())),
        ]
    );
}

#[test]
fn unquoted_attrs_separated_by_space() {
    let a = attrs("<a b=c d=e>");
    assert_eq!(
        a,
        [
            ("b".into(), Some("c".into())),
            ("d".into(), Some("e".into())),
        ]
    );
}

#[test]
fn attribute_name_case_preserved() {
    // Attribute names keep their source casing (Lunas needs `:if`, `@Click`, …
    // verbatim); only tag names are lowercased.
    let a = attrs("<div DataX=\"1\" camelCase=\"2\">");
    assert_eq!(a[0].0, "DataX");
    assert_eq!(a[1].0, "camelCase");
}

#[test]
fn duplicate_attribute_case_insensitive() {
    // `data-x` and `DATA-X` are the same attribute; first wins.
    let r = parse_html("<div data-x=\"1\" DATA-X=\"2\">");
    let e = match &r.dom.children[0] {
        Node::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(e.attributes.len(), 1);
    assert_eq!(e.attributes[0].value.as_deref(), Some("1"));
    assert!(!r.diagnostics.is_empty());
}

#[test]
fn multiline_attribute_value() {
    let a = attrs("<a title=\"line1\nline2\">");
    assert_eq!(a, [("title".into(), Some("line1\nline2".into()))]);
}

#[test]
fn boolean_attrs_between_valued() {
    let a = attrs("<input checked type=\"x\" disabled>");
    assert_eq!(
        a,
        [
            ("checked".into(), None),
            ("type".into(), Some("x".into())),
            ("disabled".into(), None),
        ]
    );
}

#[test]
fn empty_unquoted_value_after_eq_is_recovered() {
    // `b=` at end of tag — degenerate but must not panic; b becomes valueless
    // or empty, and the element still parses.
    let _ = attrs("<a b=>");
    let _ = attrs("<a b= >");
}
