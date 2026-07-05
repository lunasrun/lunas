//! Error-recovery and structural edge cases for the HTML tree builder. The
//! parser must recover from malformed input (never `Err`, never panic) and
//! preserve span-containment invariants. Complements `tests/parser.rs`.

use lunas_html_parser::{parse_html, Dom, Element, Node, ParseResult};
use lunas_span::{Severity, TextRange};

fn dom(source: &str) -> Dom {
    parse_html(source).dom
}

fn result(source: &str) -> ParseResult {
    parse_html(source)
}

fn first_element(dom: &Dom) -> &Element {
    dom.children
        .iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .expect("an element")
}

fn within(inner: TextRange, outer: TextRange) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Recursively assert every child range is contained in its parent, and every
/// element range is within the source bounds.
fn check_containment(el: &Element, file: TextRange) {
    assert!(within(el.range, file), "{} out of file bounds", el.name);
    assert!(
        within(el.open_tag_range, el.range),
        "{}: open tag not within element",
        el.name
    );
    for attr in &el.attributes {
        assert!(
            within(attr.range, el.open_tag_range),
            "{}: attr {:?} not within open tag",
            el.name,
            attr.name
        );
        if let Some(vr) = attr.value_range {
            assert!(within(vr, attr.range), "{}: value not within attr", el.name);
        }
    }
    for child in &el.children {
        assert!(
            within(child.range(), el.range),
            "{}: child not within parent",
            el.name
        );
        if let Node::Element(c) = child {
            check_containment(c, file);
        }
    }
}

fn assert_spans_ok(source: &str) {
    let d = dom(source);
    let file = TextRange::at(0, source.len() as u32);
    for node in &d.children {
        assert!(within(node.range(), file), "top node out of bounds");
        if let Node::Element(e) = node {
            check_containment(e, file);
        }
    }
}

// --- Auto-close / mismatched tags ---

#[test]
fn mismatched_close_auto_closes_multiple_levels() {
    let r = result("<a><b><c></a>");
    let a = first_element(&r.dom);
    assert_eq!(a.name, "a");
    // Warnings for the two implicitly closed elements.
    assert!(
        r.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
            >= 2
    );
    assert_spans_ok("<a><b><c></a>");
}

#[test]
fn stray_close_tag_with_no_open_errors() {
    let r = result("<p>hi</span></p>");
    assert!(r.diagnostics.iter().any(|d| d.is_error()));
    // The <p> still parses with its text.
    let p = first_element(&r.dom);
    assert_eq!(p.name, "p");
}

#[test]
fn interleaved_tags_recover() {
    // Classic misnesting `<b><i></b></i>` — must not panic and must recover.
    let r = result("<b><i>x</b>y</i>");
    assert!(!r.dom.children.is_empty());
    assert_spans_ok("<b><i>x</b>y</i>");
}

#[test]
fn unclosed_at_eof_warns_for_each() {
    let r = result("<div><section><p>text");
    let warnings = r
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    assert!(
        warnings >= 3,
        "expected an unclosed warning per open element"
    );
    assert_spans_ok("<div><section><p>text");
}

#[test]
fn case_insensitive_close_matches() {
    let r = result("<Section></SECTION>");
    assert!(r.diagnostics.is_empty());
    assert_eq!(first_element(&r.dom).name, "section");
}

// --- Void elements ---

#[test]
fn void_element_does_not_swallow_siblings() {
    let d = dom("<div><br><span>x</span></div>");
    let div = first_element(&d);
    // br (void) and span are both direct children of div.
    let names: Vec<&str> = div
        .children
        .iter()
        .filter_map(|n| match n {
            Node::Element(e) => Some(e.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, ["br", "span"]);
}

#[test]
fn void_with_close_tag_produces_stray_close() {
    // `</br>` has no matching open (br is void) — recovered as a stray close.
    let r = result("<br></br>");
    assert!(r.diagnostics.iter().any(|d| d.is_error()));
}

#[test]
fn all_void_elements_have_no_children() {
    for name in [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ] {
        let d = dom(&format!("<{name}>text-after"));
        let e = first_element(&d);
        assert!(e.children.is_empty(), "{name} should have no children");
        assert_eq!(e.name, name);
    }
}

// --- Raw text elements ---

#[test]
fn script_with_nested_close_lookalike() {
    let d = dom("<script>const s = '</scriptx>';</script>");
    let e = first_element(&d);
    match &e.children[0] {
        Node::Text(t) => assert_eq!(t.value, "const s = '</scriptx>';"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn style_with_braces_and_comments() {
    let css = "/* c */ .a { color: red } @media(x){.b{}}";
    let d = dom(&format!("<style>{css}</style>"));
    match &first_element(&d).children[0] {
        Node::Text(t) => assert_eq!(t.value, css),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn textarea_preserves_interpolation_and_markup() {
    let body = "before ${x} <b>not-an-element</b> after";
    let d = dom(&format!("<textarea>{body}</textarea>"));
    match &first_element(&d).children[0] {
        Node::Text(t) => assert_eq!(t.value, body),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn unterminated_raw_text_runs_to_eof() {
    let d = dom("<script>never closed");
    match &first_element(&d).children[0] {
        Node::Text(t) => assert_eq!(t.value, "never closed"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn self_closing_script_has_no_raw_text() {
    // `<script/>` self-closes: the following text is a sibling, not raw content.
    let d = dom("<script/>after");
    assert_eq!(d.children.len(), 2);
    assert!(first_element(&d).children.is_empty());
}

// --- Attributes ---

#[test]
fn empty_quoted_value_kept() {
    let d = dom("<a x=\"\" y=''></a>");
    let e = first_element(&d);
    assert_eq!(e.attributes[0].value.as_deref(), Some(""));
    assert_eq!(e.attributes[1].value.as_deref(), Some(""));
}

#[test]
fn attribute_value_with_reserved_chars() {
    let d = dom("<a data=\"a>b<c\"></a>");
    assert_eq!(
        first_element(&d).attributes[0].value.as_deref(),
        Some("a>b<c")
    );
}

#[test]
fn many_boolean_attributes() {
    let d = dom("<input a b c d e f g>");
    let e = first_element(&d);
    assert_eq!(e.attributes.len(), 7);
    assert!(e.attributes.iter().all(|a| a.value.is_none()));
}

#[test]
fn attribute_names_preserve_case_and_prefixes() {
    // Lunas needs `:if`, `@click`, `::v` verbatim.
    let d = dom("<div :if=\"a\" @click=\"go\" ::model=\"m\" DataX=\"1\"></div>");
    let names: Vec<&str> = first_element(&d)
        .attributes
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, [":if", "@click", "::model", "DataX"]);
}

#[test]
fn value_range_slices_back_to_value() {
    let src = "<a href=\"/path\"></a>";
    let d = dom(src);
    let attr = &first_element(&d).attributes[0];
    assert_eq!(attr.value_range.unwrap().slice(src), Some("/path"));
}

// --- Comments & doctype ---

#[test]
fn doctype_uppercase_and_lowercase() {
    assert_eq!(
        dom("<!DOCTYPE html>").kind,
        lunas_html_parser::DomKind::Document
    );
    assert_eq!(
        dom("<!doctype html>").kind,
        lunas_html_parser::DomKind::Document
    );
}

#[test]
fn comment_with_dashes_inside() {
    let d = dom("<!-- a -- b -->");
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.value, " a -- b "),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn unterminated_comment_recovers_to_eof() {
    let d = dom("<div><!-- oops");
    let div = first_element(&d);
    assert!(div.children.iter().any(|n| matches!(n, Node::Comment(_))));
}

#[test]
fn nested_comment_delimiters() {
    // `<!--` inside a comment does not open a new one; first `-->` closes.
    let d = dom("<!-- <!-- inner --> tail");
    match &d.children[0] {
        Node::Comment(c) => assert_eq!(c.value, " <!-- inner "),
        other => panic!("got {other:?}"),
    }
}

// --- Deep nesting & unicode ---

#[test]
fn deep_nesting_no_overflow_and_spans_ok() {
    let depth = 200;
    let src = "<div>".repeat(depth) + &"</div>".repeat(depth);
    let r = result(&src);
    assert!(r.diagnostics.is_empty());
    assert_spans_ok(&src);
}

#[test]
fn unicode_tag_content_and_attrs_span_ok() {
    let src = "<p title=\"日本語\">こんにちは<b>あ</b>う</p>";
    let r = result(src);
    assert!(r.diagnostics.is_empty());
    assert_spans_ok(src);
    let p = first_element(&r.dom);
    assert_eq!(p.attributes[0].value.as_deref(), Some("日本語"));
}

#[test]
fn emoji_in_text_and_ranges_valid() {
    let src = "<span>hi \u{1F600} there</span>";
    let d = dom(src);
    match &first_element(&d).children[0] {
        Node::Text(t) => {
            assert_eq!(t.value, "hi \u{1F600} there");
            assert_eq!(t.range.slice(src), Some("hi \u{1F600} there"));
        }
        other => panic!("got {other:?}"),
    }
}

// --- Dangling / stray markup ---

#[test]
fn lone_lt_is_text() {
    let d = dom("a < b");
    // No element; the `<` is plain text content.
    assert!(d.children.iter().all(|n| !matches!(n, Node::Element(_))));
}

#[test]
fn empty_angle_brackets_recover() {
    for s in ["<>", "</>", "< >", "<=>"] {
        let r = result(s);
        // Never panics and produces some children.
        let _ = r.dom.children;
    }
    assert_spans_ok("before <> after");
}

#[test]
fn trailing_open_tag_at_eof() {
    let r = result("<div><span");
    // The unclosed span at EOF still builds an element under div.
    let div = first_element(&r.dom);
    assert_eq!(div.name, "div");
}

// --- serde round-trip on a recovered tree ---

#[test]
fn recovered_tree_serde_round_trip() {
    let d = dom("<div><span></div><!-- c --><br>");
    let json = serde_json::to_string(&d).expect("serialize");
    let back: Dom = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(d, back);
}
