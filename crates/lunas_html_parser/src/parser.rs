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

