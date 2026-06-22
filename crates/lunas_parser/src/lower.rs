//! Stage 2: lower the raw Pest items into the public [`ParsedFile`].
//!
//! Validates block uniqueness, extracts indentation-stripped block bodies,
//! invokes the HTML and JS sub-parsers, and parses directive bodies. Never
//! panics; all problems are accumulated as [`Diagnostic`]s.

use lunas_html_parser::parse_html;
use lunas_span::{Diagnostic, LineIndex, TextRange};

use crate::ir::{
    BlockSource, Directive, HtmlBlock, PropInput, ScriptBlock, StyleBlock, UseComponent,
};
use crate::parser1::{parse1, RawDirective, RawItem, RawLanguageBlock};
use crate::swc_parser::parse_to_ast_json;
use crate::ts_to_js::transform_ts_to_js;
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

    // HTML: pass the indentation-stripped body to the HTML parser. The frozen
    // Dom ranges are therefore relative to the stripped body, not the file; we
    // keep `BlockSource.range` for the block's location in the original source
    // and store the Dom as-is. Rebasing every Dom node would be invasive and is
    // deferred to consumers that need file-absolute HTML positions.
    let html = html_raw.map(|block| {
        let source = extract_block_source(source, block.body_range);
        let result = parse_html(&source.text);
        diagnostics.extend(result.diagnostics);
        HtmlBlock {
            source,
            dom: result.dom,
        }
    });

    let style = style_raw.map(|block| StyleBlock {
        source: extract_block_source(source, block.body_range),
    });

    let script = script_raw.map(|block| {
        let source = extract_block_source(source, block.body_range);
        let js = match transform_ts_to_js(&source.text) {
            Ok(js) => js,
            Err(e) => {
                diagnostics.push(Diagnostic::error(block.body_range, e.to_string()));
                source.text.clone()
            }
        };
        let ast = match parse_to_ast_json(&js) {
            Ok(ast) => ast,
            Err(e) => {
                diagnostics.push(Diagnostic::error(block.body_range, e.to_string()));
                serde_json::Value::Null
            }
        };
        ScriptBlock { source, js, ast }
    });

    let mut directives = Vec::new();
    for raw in raw_directives {
        if let Some(directive) = lower_directive(source, raw, &mut diagnostics) {
            directives.push(directive);
        }
    }

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

/// Extracts a block body: strips the common leading indentation but keeps the
/// `range` pointing at the original (un-stripped) region of the source.
fn extract_block_source(source: &str, range: TextRange) -> BlockSource {
    let raw = range.slice(source).unwrap_or("");
    BlockSource {
        text: strip_common_indent(raw),
        range,
    }
}

/// Removes the longest whitespace prefix common to all non-blank lines.
/// Blank lines are preserved as empty lines. Surrounding blank lines are
/// trimmed.
fn strip_common_indent(text: &str) -> String {
    let common = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(leading_ws_len)
        .min()
        .unwrap_or(0);

    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if !line.trim().is_empty() {
            out.push_str(&line[common..]);
        }
    }
    out.trim_matches('\n').to_string()
}

fn leading_ws_len(line: &str) -> usize {
    line.bytes()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count()
}

fn lower_directive(
    source: &str,
    raw: &RawDirective,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Directive> {
    let body = raw
        .body_range
        .and_then(|r| r.slice(source))
        .map(str::trim)
        .unwrap_or("");
    let body_range = raw.body_range.unwrap_or(TextRange::empty(0u32.into()));

    match raw.keyword.as_str() {
        "input" => match parse_input(body) {
            Some(mut prop) => {
                prop.range = body_range;
                Some(Directive::Input(prop))
            }
            None => {
                diagnostics.push(Diagnostic::error(
                    body_range,
                    "invalid `@input` declaration; expected `name: Type = default`",
                ));
                None
            }
        },
        "use" => match parse_use(body) {
            Some(mut comp) => {
                comp.range = body_range;
                Some(Directive::UseComponent(comp))
            }
            None => {
                diagnostics.push(Diagnostic::error(
                    body_range,
                    "invalid `@use` declaration; expected `Name from \"path\"`",
                ));
                None
            }
        },
        "useAutoRouting" => Some(Directive::UseAutoRouting),
        "useRouting" => Some(Directive::UseRouting),
        other => {
            diagnostics.push(Diagnostic::warning(
                body_range,
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
    fn strip_common_indent_basic() {
        assert_eq!(strip_common_indent("    a\n    b"), "a\nb");
    }

    #[test]
    fn strip_preserves_relative_indent() {
        assert_eq!(strip_common_indent("    a\n        b\n    c"), "a\n    b\nc");
    }

    #[test]
    fn strip_blank_lines() {
        assert_eq!(strip_common_indent("    a\n\n    b"), "a\n\nb");
    }

    #[test]
    fn strip_tabs() {
        assert_eq!(strip_common_indent("\t\ta\n\t\tb"), "a\nb");
    }

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
