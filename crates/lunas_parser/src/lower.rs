//! Stage 2: lower the raw Pest items into the public [`ParsedFile`].
//!
//! Validates block uniqueness, extracts indentation-stripped block bodies,
//! invokes the HTML sub-parser, and parses directive bodies. The script block
//! is only extracted as raw text; JS/TS parsing lives in `lunas_script`. Never
//! panics; all problems are accumulated as [`Diagnostic`]s.

use lunas_html_parser::parse_html;
use lunas_span::{Diagnostic, LineIndex, TextRange};

use crate::ir::{
    BlockSource, Directive, HtmlBlock, PropInput, ScriptBlock, StyleBlock, UseComponent,
};
use crate::parser1::{parse1, RawDirective, RawItem, RawLanguageBlock};
use crate::ParsedFile;

pub(crate) fn lower(source: &str) -> (ParsedFile, Vec<Diagnostic>) {
    let line_index = LineIndex::new(source);
    let mut diagnostics = Vec::new();

    let items = match parse1(source) {
        Ok(items) => items,
        Err(diag) => {
            diagnostics.push(diag);
            return (
                ParsedFile {
                    html: None,
                    style: None,
                    script: None,
                    directives: Vec::new(),
                    line_index,
                },
                diagnostics,
            );
        }
    };

    let mut html_raw: Option<&RawLanguageBlock> = None;
    let mut style_raw: Option<&RawLanguageBlock> = None;
    let mut script_raw: Option<&RawLanguageBlock> = None;
    let mut raw_directives: Vec<&RawDirective> = Vec::new();

    for item in &items {
        match item {
            RawItem::LanguageBlock(block) => {
                let slot = match block.name.as_str() {
                    "html" => &mut html_raw,
                    "style" => &mut style_raw,
                    "script" => &mut script_raw,
                    _ => continue,
                };
                if slot.is_some() {
                    diagnostics.push(Diagnostic::error(
                        block.body_range,
                        format!("duplicate `{}:` block", block.name),
                    ));
                } else {
                    *slot = Some(block);
                }
            }
            RawItem::Directive(d) => raw_directives.push(d),
        }
    }

    if html_raw.is_none() {
        diagnostics.push(Diagnostic::warning(
            TextRange::empty(0u32.into()),
            "missing `html:` block",
        ));
    }

    // Lower directives first: the template pass needs the `@use` component
    // name set to tell components apart from HTML elements.
    let mut directives = Vec::new();
    for raw in raw_directives {
        if let Some(directive) = lower_directive(source, raw, &mut diagnostics) {
            directives.push(directive);
        }
    }
    let component_names: std::collections::HashSet<String> = directives
        .iter()
        .filter_map(|d| match d {
            Directive::UseComponent(u) => Some(u.component_name.clone()),
            _ => None,
        })
        .collect();

    // Every block keeps its body verbatim (no indentation stripping). This is
    // what makes position mapping exact: each block's text equals
    // `range.slice(file)`, so an offset within a block maps to the extracted
    // text by a constant byte/line shift with the column unchanged — the
    // property the language server relies on (and the HTML Dom relies on for a
    // single-offset rebase). SWC is happy to parse indented script, so there is
    // no reason to strip.
    let html = html_raw.map(|block| {
        let source_block = extract_block_source(source, block.body_range);
        let mut result = parse_html(&source_block.text);

        let offset = block.body_range.start();
        result.dom.shift_ranges(offset);
        diagnostics.extend(result.diagnostics.into_iter().map(|mut d| {
            d.range = d.range.shifted(offset);
            d
        }));

        let template =
            crate::template::build(source, &result.dom, &component_names, &mut diagnostics);

        HtmlBlock {
            source: source_block,
            dom: result.dom,
            template,
        }
    });

    let style = style_raw.map(|block| StyleBlock {
        source: extract_block_source(source, block.body_range),
    });

    let script = script_raw.map(|block| ScriptBlock {
        source: extract_block_source(source, block.body_range),
    });

    (
        ParsedFile {
            html,
            style,
            script,
            directives,
            line_index,
        },
        diagnostics,
    )
}

/// Extracts a block body verbatim: `text` is exactly `range.slice(source)`.
fn extract_block_source(source: &str, range: TextRange) -> BlockSource {
    BlockSource {
        text: range.slice(source).unwrap_or("").to_string(),
        range,
    }
}

fn lower_directive(
    source: &str,
    raw: &RawDirective,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Directive> {
    let content = raw
        .content_range
        .and_then(|r| r.slice(source))
        .map(str::trim)
        .unwrap_or("");
    let content_range = raw.content_range.unwrap_or(TextRange::empty(0u32.into()));

    match raw.keyword.as_str() {
        "input" => match parse_input(content) {
            Some(mut prop) => {
                prop.range = content_range;
                Some(Directive::Input(prop))
            }
            None => {
                diagnostics.push(Diagnostic::error(
                    content_range,
                    "invalid `@input` declaration; expected `name: Type = default`",
                ));
                None
            }
        },
        "use" => match parse_use(content) {
            Some(mut comp) => {
                comp.range = content_range;
                Some(Directive::UseComponent(comp))
            }
            None => {
                diagnostics.push(Diagnostic::error(
                    content_range,
                    "invalid `@use` declaration; expected `Name from \"path\"`",
                ));
                None
            }
        },
        "useAutoRouting" => Some(Directive::UseAutoRouting),
        "useRouting" => Some(Directive::UseRouting),
        other => {
            diagnostics.push(Diagnostic::warning(
                content_range,
                format!("unknown directive `@{}`", other),
            ));
            None
        }
    }
}

/// Parses an `@input` body: `name: Type? = default`, where the type and the
/// default are both optional and a trailing `?` on the type marks it nullable.
fn parse_input(body: &str) -> Option<PropInput> {
    let (name_part, rest) = body.split_once(':')?;
    let name = name_part.trim();
    if name.is_empty() || !is_ident(name) {
        return None;
    }

    let (type_part, default_value) = match rest.split_once('=') {
        Some((t, d)) => (t.trim(), Some(d.trim().to_string())),
        None => (rest.trim(), None),
    };

    let (type_str, nullable) = match type_part.strip_suffix('?') {
        Some(stripped) => (stripped.trim(), true),
        None => (type_part, false),
    };

    let type_annotation = if type_str.is_empty() {
        None
    } else {
        Some(type_str.to_string())
    };

    Some(PropInput {
        name: name.to_string(),
        type_annotation,
        default_value: default_value.filter(|d| !d.is_empty()),
        nullable,
        range: TextRange::empty(0u32.into()),
    })
}

/// Parses a `@use` body: `Name from "path"` (single or double quotes).
fn parse_use(body: &str) -> Option<UseComponent> {
    let (name_part, after) = body.split_once("from")?;
    let component_name = name_part.trim();
    if component_name.is_empty() || !is_ident(component_name) {
        return None;
    }
    let path = unquote(after.trim())?;
    Some(UseComponent {
        component_name: component_name.to_string(),
        path,
        range: TextRange::empty(0u32.into()),
    })
}

fn unquote(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return Some(s[1..s.len() - 1].to_string());
        }
    }
    None
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_full() {
        let p = parse_input("count: number = 0").expect("ok");
        assert_eq!(p.name, "count");
        assert_eq!(p.type_annotation.as_deref(), Some("number"));
        assert_eq!(p.default_value.as_deref(), Some("0"));
        assert!(!p.nullable);
    }

    #[test]
    fn input_nullable() {
        let p = parse_input("name: string? = \"x\"").expect("ok");
        assert!(p.nullable);
        assert_eq!(p.type_annotation.as_deref(), Some("string"));
        assert_eq!(p.default_value.as_deref(), Some("\"x\""));
    }

    #[test]
    fn input_type_only() {
        let p = parse_input("flag: boolean").expect("ok");
        assert_eq!(p.type_annotation.as_deref(), Some("boolean"));
        assert_eq!(p.default_value, None);
        assert!(!p.nullable);
    }

    #[test]
    fn input_no_type() {
        let p = parse_input("x:").expect("ok");
        assert_eq!(p.type_annotation, None);
        assert_eq!(p.default_value, None);
    }

    #[test]
    fn input_nullable_no_default() {
        let p = parse_input("x: T?").expect("ok");
        assert!(p.nullable);
        assert_eq!(p.type_annotation.as_deref(), Some("T"));
    }

    #[test]
    fn input_invalid_no_colon() {
        assert!(parse_input("noColon").is_none());
    }

    #[test]
    fn use_double_quotes() {
        let u = parse_use("Button from \"./Button\"").expect("ok");
        assert_eq!(u.component_name, "Button");
        assert_eq!(u.path, "./Button");
    }

    #[test]
    fn use_single_quotes() {
        let u = parse_use("Card from './Card.lunas'").expect("ok");
        assert_eq!(u.path, "./Card.lunas");
    }

    #[test]
    fn use_invalid_no_from() {
        assert!(parse_use("Button \"./x\"").is_none());
    }

    #[test]
    fn use_invalid_unquoted() {
        assert!(parse_use("Button from ./x").is_none());
    }
}
