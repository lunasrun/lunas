//! Raw-text element edge cases (script/style/title/textarea): content is taken
//! verbatim up to the matching close tag, with no markup or entity processing.

use lunas_html_parser::{parse_html, Node};

fn raw_text(source: &str) -> String {
    let dom = parse_html(source).dom;
    let el = dom
        .children
        .into_iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .expect("element");
    match el.children.into_iter().next() {
        Some(Node::Text(t)) => t.value,
        _ => String::new(),
    }
}

#[test]
fn style_with_nested_at_media_braces() {
    let css = "@media (min-width: 1px) { .a { color: red } }";
    assert_eq!(raw_text(&format!("<style>{css}</style>")), css);
}

#[test]
fn textarea_keeps_entities_and_markup_verbatim() {
    let body = "a &amp; b <c> {x}";
    assert_eq!(raw_text(&format!("<textarea>{body}</textarea>")), body);
}

#[test]
fn title_keeps_markup_verbatim() {
    assert_eq!(raw_text("<title>x < y & z</title>"), "x < y & z");
}

#[test]
fn raw_text_close_tag_is_case_insensitive() {
    assert_eq!(raw_text("<script>x</SCRIPT>"), "x");
}

#[test]
fn raw_text_partial_close_is_content() {
    assert_eq!(raw_text("<style>a </styl b </style>"), "a </styl b ");
}

#[test]
fn empty_raw_text_elements() {
    for tag in ["script", "style", "title", "textarea"] {
        let src = format!("<{tag}></{tag}>");
        assert_eq!(raw_text(&src), "");
    }
}
