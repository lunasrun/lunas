//! Recursive-descent tree builder. Implemented by the html-parser agent.

use crate::dom::{Attribute, Comment, Dom, DomKind, Element, ElementKind, Node, Text};
use crate::lexer::{tokenize, Token, TokenKind};
use crate::ParseResult;
use lunas_span::{Diagnostic, TextRange, TextSize};

pub(crate) fn parse(source: &str) -> ParseResult {
    let tokens = tokenize(source);
    Builder::new(source, tokens).run()
}

/// An element that is currently open on the stack while its children are being
/// collected.
struct OpenElement {
    name: String,
    /// Original-cased name for round-tripping diagnostics; lowercased name above
    /// is used for matching.
    raw_name: String,
    attributes: Vec<Attribute>,
    children: Vec<Node>,
    start: TextSize,
    open_tag_range: TextRange,
}

struct Builder<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    stack: Vec<OpenElement>,
    roots: Vec<Node>,
    diagnostics: Vec<Diagnostic>,
    has_doctype: bool,
}

fn slice_owned(source: &str, range: TextRange) -> String {
    range.slice(source).unwrap_or("").to_string()
}

impl<'a> Builder<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Builder {
            source,
            tokens,
            pos: 0,
            stack: Vec::new(),
            roots: Vec::new(),
            diagnostics: Vec::new(),
            has_doctype: false,
        }
    }

    /// Appends a finished node to whatever container is currently on top.
    fn push_node(&mut self, node: Node) {
        match self.stack.last_mut() {
            Some(open) => open.children.push(node),
            None => self.roots.push(node),
        }
    }

    fn run(mut self) -> ParseResult {
        while self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            match token.kind {
                TokenKind::Doctype => self.has_doctype = true,
                TokenKind::Text => {
                    self.push_node(Node::Text(Text {
                        value: slice_owned(self.source, token.range),
                        range: token.range,
                    }));
                }
                TokenKind::Comment { content } => {
                    self.push_node(Node::Comment(Comment {
                        value: slice_owned(self.source, content),
                        range: token.range,
                    }));
                }
                TokenKind::RawText => {
                    self.push_node(Node::Text(Text {
                        value: slice_owned(self.source, token.range),
                        range: token.range,
                    }));
                }
                TokenKind::OpenTagStart { name } => self.open_element(name, token.range),
                TokenKind::CloseTag { name } => self.close_element(name, token.range),
                // The delimiters and stray errors are consumed alongside their
                // open tag; encountering them standalone means nothing to do.
                TokenKind::OpenTagEnd
                | TokenKind::SelfCloseTagEnd
                | TokenKind::Attribute { .. }
                | TokenKind::Error => {}
            }
        }

        self.close_unclosed_at_eof();

        let kind = if self.has_doctype {
            DomKind::Document
        } else if self.roots.iter().all(is_whitespace_node) {
            DomKind::Empty
        } else {
            DomKind::Fragment
        };

        ParseResult {
            dom: Dom {
                kind,
                children: self.roots,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn open_element(&mut self, name_range: TextRange, start_range: TextRange) {
        let raw_name = slice_owned(self.source, name_range);
        let name = raw_name.to_ascii_lowercase();

        let (attributes, end_range, self_closed) = self.collect_attributes();
        let open_tag_range = TextRange::new(start_range.start(), end_range.end());

        if crate::is_void_element(&name) {
            let element = Element {
                name,
                kind: ElementKind::Void,
                attributes,
                children: Vec::new(),
                range: open_tag_range,
                open_tag_range,
            };
            self.push_node(Node::Element(element));
            return;
        }

        if self_closed {
            let element = Element {
                name,
                kind: ElementKind::Normal,
                attributes,
                children: Vec::new(),
                range: open_tag_range,
                open_tag_range,
            };
            self.push_node(Node::Element(element));
            return;
        }

        self.stack.push(OpenElement {
            name,
            raw_name,
            attributes,
            children: Vec::new(),
            start: start_range.start(),
            open_tag_range,
        });
    }

    /// Consumes attribute tokens following an `OpenTagStart` plus the closing
    /// delimiter. Returns the attributes, the delimiter token's range, and
    /// whether the tag was self-closing.
    fn collect_attributes(&mut self) -> (Vec<Attribute>, TextRange, bool) {
        let mut attributes: Vec<Attribute> = Vec::new();
        // Fall back to the previous token's end if the stream ends abruptly.
        let mut end_range = self.tokens[self.pos.saturating_sub(1)].range;

        while self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            match token.kind {
                TokenKind::Attribute { name, value } => {
                    self.pos += 1;
                    let attr_name = slice_owned(self.source, name);
                    let lowered = attr_name.to_ascii_lowercase();
                    if attributes
                        .iter()
                        .any(|a| a.name.eq_ignore_ascii_case(&lowered))
                    {
                        self.diagnostics.push(Diagnostic::warning(
                            token.range,
                            format!("duplicate attribute `{}`", attr_name),
                        ));
                        continue;
                    }
                    attributes.push(Attribute {
                        name: attr_name,
                        value: value.map(|v| slice_owned(self.source, v)),
                        range: token.range,
                    });
                }
                TokenKind::OpenTagEnd => {
                    end_range = token.range;
                    self.pos += 1;
                    return (attributes, end_range, false);
                }
                TokenKind::SelfCloseTagEnd => {
                    end_range = token.range;
                    self.pos += 1;
                    return (attributes, end_range, true);
                }
                TokenKind::Error => {
                    self.pos += 1;
                }
                // Any other token means the open tag was never properly closed
                // (e.g. EOF mid-tag); stop without consuming it.
                _ => break,
            }
        }
        (attributes, end_range, false)
    }

    fn close_element(&mut self, name_range: TextRange, close_range: TextRange) {
        let name = slice_owned(self.source, name_range).to_ascii_lowercase();

        let matching = self.stack.iter().rposition(|e| e.name == name);
        match matching {
            None => {
                self.diagnostics.push(Diagnostic::error(
                    close_range,
                    format!("stray closing tag `</{}>`", name),
                ));
            }
            Some(index) => {
                // Auto-close any elements above the match.
                while self.stack.len() > index + 1 {
                    let unclosed = self.finalize_top(None);
                    if let Some(name) = unclosed {
                        self.diagnostics.push(Diagnostic::warning(
                            close_range,
                            format!("`<{}>` implicitly closed by `</{}>`", name, slice_owned(self.source, name_range)),
                        ));
                    }
                }
                self.finalize_top(Some(close_range.end()));
            }
        }
    }

    /// Pops the top open element, builds an `Element` node, and appends it to
    /// its parent. `close_end` is the end offset of the matching close tag (or
    /// `None` when auto-closed, in which case the element ends where its last
    /// child does). Returns the auto-closed element's raw name when `close_end`
    /// is `None`.
    fn finalize_top(&mut self, close_end: Option<TextSize>) -> Option<String> {
        let Some(open) = self.stack.pop() else {
            return None;
        };
        let auto_closed = close_end.is_none();
        let end = close_end.unwrap_or_else(|| {
            open.children
                .last()
                .map(|n| n.range().end())
                .unwrap_or(open.open_tag_range.end())
        });
        let range = TextRange::new(open.start, end);
        let element = Element {
            name: open.name,
            kind: ElementKind::Normal,
            attributes: open.attributes,
            children: open.children,
            range,
            open_tag_range: open.open_tag_range,
        };
        self.push_node(Node::Element(element));
        if auto_closed {
            Some(open.raw_name)
        } else {
            None
        }
    }

    fn close_unclosed_at_eof(&mut self) {
        while !self.stack.is_empty() {
            let tag_range = self
                .stack
                .last()
                .map(|e| e.open_tag_range)
                .unwrap_or_else(|| TextRange::empty(TextSize::new(0)));
            if let Some(name) = self.finalize_top(None) {
                self.diagnostics.push(Diagnostic::warning(
                    tag_range,
                    format!("unclosed element `<{}>`", name),
                ));
            }
        }
    }
}

fn is_whitespace_node(node: &Node) -> bool {
    match node {
        Node::Text(t) => t.value.trim().is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dom(source: &str) -> Dom {
        parse(source).dom
    }

    fn parse_result(source: &str) -> ParseResult {
        parse(source)
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
        assert_eq!(
            first_element(&d).attributes[0].value.as_deref(),
            Some("/x")
        );
    }

    #[test]
    fn single_quoted_attribute() {
        let d = dom("<a href='/x'></a>");
        assert_eq!(
            first_element(&d).attributes[0].value.as_deref(),
            Some("/x")
        );
    }

    #[test]
    fn unquoted_attribute() {
        let d = dom("<a href=/x></a>");
        assert_eq!(
            first_element(&d).attributes[0].value.as_deref(),
            Some("/x")
        );
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
        assert_eq!(r.diagnostics[0].severity, lunas_span::Severity::Warning);
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
        // span auto-closed by </div>
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
        assert!(r.diagnostics.iter().any(|d| d.severity == lunas_span::Severity::Warning));
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
            .any(|d| d.severity == lunas_span::Severity::Warning));
        // Tree still built: div > p > text
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
            .unwrap();
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
        let json = serde_json::to_string(&d).unwrap();
        let back: Dom = serde_json::from_str(&json).unwrap();
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
        // b and c auto-closed inside a
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
}
