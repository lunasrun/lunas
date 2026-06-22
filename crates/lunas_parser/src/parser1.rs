//! Stage 1: split a `.lunas` file into raw blocks and directives using the
//! Pest grammar. Produces [`RawItem`]s with byte-offset ranges; semantic
//! validation and sub-parsing happen later in `lower`.

use lunas_span::{Diagnostic, TextRange};
use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "grammar/lunas.pest"]
struct LunasParser;

/// A raw, un-analyzed top-level item produced by the Pest grammar.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawItem {
    LanguageBlock(RawLanguageBlock),
    Directive(RawDirective),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawLanguageBlock {
    pub name: String,
    /// Range of the block body (excluding the `name:` keyword line).
    pub body_range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawDirective {
    pub keyword: String,
    pub params: Option<String>,
    /// Range of the directive's content: the inline text after the keyword
    /// (e.g. `message1:string` in `@input message1:string`) when present,
    /// otherwise the indented body line(s). `None` for bare directives.
    pub content_range: Option<TextRange>,
}

fn span_range(span: pest::Span) -> TextRange {
    TextRange::at(span.start() as u32, span.end() as u32)
}

/// Runs the Pest grammar over `source`. On a grammar error returns a single
/// diagnostic; the lowering stage treats that as "no items".
pub(crate) fn parse1(source: &str) -> Result<Vec<RawItem>, Diagnostic> {
    let file = LunasParser::parse(Rule::file, source).map_err(|e| {
        let range = match e.location {
            pest::error::InputLocation::Pos(p) => TextRange::at(p as u32, p as u32),
            pest::error::InputLocation::Span((s, end)) => TextRange::at(s as u32, end as u32),
        };
        Diagnostic::error(range, format!("failed to parse .lunas file: {}", e.variant))
    })?;

    let mut items = Vec::new();

    // `file` is the single top rule; iterate its inner items.
    for pair in file.into_iter().flatten() {
        match pair.as_rule() {
            Rule::directive => {
                let mut keyword = String::new();
                let mut params = None;
                let mut inline_range = None;
                let mut body_range = None;
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::ident => keyword = inner.as_str().to_string(),
                        Rule::params => params = Some(inner.as_str().to_string()),
                        Rule::inline => {
                            let span = inner.as_span();
                            if !inner.as_str().trim().is_empty() {
                                inline_range = Some(span_range(span));
                            }
                        }
                        Rule::body => {
                            let span = inner.as_span();
                            if span.start() != span.end() {
                                body_range = Some(span_range(span));
                            }
                        }
                        _ => {}
                    }
                }
                // Inline content (same line) takes precedence over an indented
                // body, matching the canonical `@input name:type` form.
                items.push(RawItem::Directive(RawDirective {
                    keyword,
                    params,
                    content_range: inline_range.or(body_range),
                }));
            }
            Rule::language_block => {
                let mut name = String::new();
                let mut body_range = TextRange::empty(0u32.into());
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::block_name => name = inner.as_str().to_string(),
                        Rule::body => body_range = span_range(inner.as_span()),
                        _ => {}
                    }
                }
                items.push(RawItem::LanguageBlock(RawLanguageBlock { name, body_range }));
            }
            _ => {}
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directives(items: &[RawItem]) -> Vec<&RawDirective> {
        items
            .iter()
            .filter_map(|i| match i {
                RawItem::Directive(d) => Some(d),
                _ => None,
            })
            .collect()
    }

    fn blocks(items: &[RawItem]) -> Vec<&RawLanguageBlock> {
        items
            .iter()
            .filter_map(|i| match i {
                RawItem::LanguageBlock(b) => Some(b),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn parses_single_html_block() {
        let src = "html:\n    <div></div>\n";
        let items = parse1(src).expect("parse ok");
        let b = blocks(&items);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].name, "html");
        let body = b[0].body_range.slice(src).expect("slice");
        assert!(body.contains("<div></div>"));
    }

    #[test]
    fn parses_directive_with_inline_content() {
        let src = "@input message1:string\n";
        let items = parse1(src).expect("parse ok");
        let d = directives(&items);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].keyword, "input");
        let content = d[0].content_range.expect("content").slice(src).expect("slice");
        assert_eq!(content.trim(), "message1:string");
    }

    #[test]
    fn parses_directive_with_indented_body() {
        let src = "@input\nname: string = \"a\"\n";
        let items = parse1(src).expect("parse ok");
        let d = directives(&items);
        let content = d[0].content_range.expect("content").slice(src).expect("slice");
        assert!(content.contains("name: string = \"a\""));
    }

    #[test]
    fn directive_body_with_colon_not_a_block() {
        let src = "@input\nname: Type\n";
        let items = parse1(src).expect("parse ok");
        assert!(blocks(&items).is_empty());
        assert_eq!(directives(&items).len(), 1);
    }

    #[test]
    fn no_param_directive() {
        let src = "@useAutoRouting\n";
        let items = parse1(src).expect("parse ok");
        let d = directives(&items);
        assert_eq!(d[0].keyword, "useAutoRouting");
        assert_eq!(d[0].params, None);
        assert_eq!(d[0].content_range, None);
    }

    #[test]
    fn multiple_blocks() {
        let src = "html:\n    <p/>\nstyle:\n    p{}\nscript:\n    let x=0\n";
        let items = parse1(src).expect("parse ok");
        assert_eq!(blocks(&items).len(), 3);
    }
}
