//! Runs the html5lib-tests tokenizer suite against our lexer.
//!
//! html5lib-tests (vendored under `tests/html5lib/`, MIT licensed — see the
//! README there) is the standard cross-implementation conformance suite. Our
//! lexer is a pragmatic tokenizer for `.lunas` templates, not a spec-complete
//! HTML5 tokenizer: it intentionally does not implement character-reference
//! decoding, the alternate tokenizer states (RCDATA/RAWTEXT/PLAINTEXT as
//! selectable initial states), DOCTYPE internals, or bogus-comment/markup
//! declaration recovery.
//!
//! This harness therefore runs the subset of the suite that falls inside our
//! scope (see `is_in_scope`) and asserts we match the spec exactly on it.
//! Out-of-scope cases are counted and reported, not silently ignored, so the
//! coverage picture stays honest.

use lunas_html_parser::internals::{tokenize, Token, TokenKind};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A token in the html5lib expected-output model.
#[derive(Debug, Clone, PartialEq, Eq)]
enum H5 {
    Character(String),
    Comment(String),
    StartTag {
        name: String,
        attrs: BTreeMap<String, String>,
        self_closing: bool,
    },
    EndTag(String),
    Doctype,
}

/// Converts our token stream into the html5lib model so the two can be
/// compared. Applies the same normalizations html5lib's tokenizer does for the
/// data state: lowercased tag/attr names, first-wins duplicate attributes, and
/// merged adjacent character runs.
fn map_tokens(source: &str, tokens: &[Token]) -> Vec<H5> {
    let mut out: Vec<H5> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match &tok.kind {
            TokenKind::Text | TokenKind::RawText => {
                push_chars(&mut out, slice(source, tok));
                i += 1;
            }
            TokenKind::Comment { content } => {
                out.push(H5::Comment(
                    content.slice(source).unwrap_or("").to_string(),
                ));
                i += 1;
            }
            TokenKind::Doctype => {
                out.push(H5::Doctype);
                i += 1;
            }
            TokenKind::CloseTag { name } => {
                out.push(H5::EndTag(
                    name.slice(source).unwrap_or("").to_ascii_lowercase(),
                ));
                i += 1;
            }
            TokenKind::OpenTagStart { name } => {
                let tag_name = name.slice(source).unwrap_or("").to_ascii_lowercase();
                let mut attrs: BTreeMap<String, String> = BTreeMap::new();
                let mut self_closing = false;
                i += 1;
                while i < tokens.len() {
                    match &tokens[i].kind {
                        TokenKind::Attribute { name, value } => {
                            let key = name.slice(source).unwrap_or("").to_ascii_lowercase();
                            let val = value
                                .and_then(|v| v.slice(source))
                                .unwrap_or("")
                                .to_string();
                            // html5lib keeps the first occurrence of a duplicate.
                            attrs.entry(key).or_insert(val);
                            i += 1;
                        }
                        TokenKind::OpenTagEnd => {
                            i += 1;
                            break;
                        }
                        TokenKind::SelfCloseTagEnd => {
                            self_closing = true;
                            i += 1;
                            break;
                        }
                        _ => break,
                    }
                }
                out.push(H5::StartTag {
                    name: tag_name,
                    attrs,
                    self_closing,
                });
            }
            // Error / stray delimiter tokens have no html5lib counterpart.
            TokenKind::OpenTagEnd
            | TokenKind::SelfCloseTagEnd
            | TokenKind::Attribute { .. }
            | TokenKind::Error => {
                i += 1;
            }
        }
    }
    out
}

fn push_chars(out: &mut Vec<H5>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(H5::Character(last)) = out.last_mut() {
        last.push_str(text);
    } else {
        out.push(H5::Character(text.to_string()));
    }
}

fn slice<'a>(source: &'a str, tok: &Token) -> &'a str {
    tok.range.slice(source).unwrap_or("")
}

/// Parses the html5lib expected-output JSON array into our model, merging
/// adjacent `Character` tokens (the suite already does this, but be defensive).
fn parse_expected(output: &Value) -> Option<Vec<H5>> {
    let arr = output.as_array()?;
    let mut out: Vec<H5> = Vec::new();
    for tok in arr {
        let parts = tok.as_array()?;
        let kind = parts.first()?.as_str()?;
        match kind {
            "Character" => push_chars(&mut out, parts.get(1)?.as_str()?),
            "Comment" => out.push(H5::Comment(parts.get(1)?.as_str()?.to_string())),
            "StartTag" => {
                let name = parts.get(1)?.as_str()?.to_string();
                let mut attrs = BTreeMap::new();
                if let Some(obj) = parts.get(2).and_then(|v| v.as_object()) {
                    for (k, v) in obj {
                        attrs.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                    }
                }
                let self_closing = parts.get(3).and_then(|v| v.as_bool()).unwrap_or(false);
                out.push(H5::StartTag {
                    name,
                    attrs,
                    self_closing,
                });
            }
            "EndTag" => out.push(H5::EndTag(parts.get(1)?.as_str()?.to_string())),
            "DOCTYPE" => out.push(H5::Doctype),
            _ => return None,
        }
    }
    Some(out)
}

/// Returns `Some(reason)` if a test case falls outside our tokenizer's scope.
/// Out-of-scope cases exercise spec machinery we deliberately do not implement;
/// each reason documents exactly which.
fn out_of_scope_reason(input: &str, expected: &[H5]) -> Option<&'static str> {
    if input.contains('&') {
        return Some("character references not decoded");
    }
    if input.contains('\0') {
        return Some("NUL not replaced with U+FFFD");
    }
    if input.contains('\r') {
        return Some("CR/CRLF not normalized to LF");
    }
    if expected.iter().any(|t| matches!(t, H5::Doctype)) {
        return Some("DOCTYPE internals not modeled");
    }
    if input.contains("<![CDATA[") {
        return Some("CDATA sections");
    }
    // Bogus comment / markup declaration: `<!` not opening a real comment.
    if input.contains("<!") && !input.contains("<!--") {
        return Some("bogus comment / markup declaration recovery");
    }
    // Comment end-state automaton: unterminated or abruptly-closed comments
    // (`<!--`, `<!-->`, `<!--->`, `<!-- --`, nested `<!--`). We only implement
    // "content up to the first `-->`".
    if input.starts_with("<!--") {
        match input.find("-->") {
            None => return Some("unterminated comment recovery"),
            // `-->` overlapping the `<!--` opener is an abrupt close.
            Some(idx) if idx < 4 => return Some("abrupt comment closing"),
            _ => {}
        }
    }
    // Empty/`</>`-style tags and `<?` processing instructions.
    if input.contains("<>") || input.contains("</>") || input.contains("<?") {
        return Some("empty-tag / processing-instruction recovery");
    }
    // Illegal or unfinished end tags: `</` not followed by an ASCII letter
    // (`</`, `</1>`, `</!`) become bogus comments / characters per spec.
    if has_illegal_end_tag(input) {
        return Some("illegal/unfinished end tag recovery");
    }
    // A `<` appearing inside an open tag (`<a<b>`) has bespoke spec handling.
    if has_nested_lt_in_tag(input) {
        return Some("nested `<` inside a tag");
    }
    None
}

/// Detects `</` not immediately followed by an ASCII letter.
fn has_illegal_end_tag(input: &str) -> bool {
    let bytes = input.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'<' && bytes[i + 1] == b'/' {
            match bytes.get(i + 2) {
                Some(c) if c.is_ascii_alphabetic() => {}
                _ => return true,
            }
        }
    }
    false
}

/// Detects a `<` that begins a tag and contains another `<` before its `>`.
fn has_nested_lt_in_tag(input: &str) -> bool {
    let bytes = input.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'<' && bytes.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'>' {
                if bytes[j] == b'<' {
                    return true;
                }
                j += 1;
            }
        }
    }
    false
}

/// Specific, individually-reviewed cases where our tokenizer knowingly diverges
/// from the HTML5 spec on adversarial input, keyed by the html5lib test
/// description. These exercise per-character recovery states (attribute-name /
/// before-value / tag-name automata, illegal end-tag names, astral tag-name
/// starts) that a `.lunas` template tokenizer has no need to reproduce.
///
/// Unlike the broad category predicates above, this list is exact: any *new*
/// divergence that is not enumerated here fails the suite, so the list doubles
/// as a regression guard.
const KNOWN_DIVERGENCES: &[&str] = &[
    // `=` appearing where an attribute name is expected: the spec starts an
    // attribute literally named `=`; we treat `=` as part of the surrounding
    // token. (Descriptions are the raw inputs as named by html5lib.)
    "= attribute",
    "== attribute",
    "=== attribute",
    "==== attribute",
    "<a =>",
    "<a=>",
    "<a/=>",
    "<a a=''=>",
    // `/` inside an unquoted attribute value is value text per spec, not a
    // self-close marker.
    "<a a=/>",
    "<a a=a/>",
    // Astral (non-ASCII) code point as a tag-name start: spec emits it as text.
    // The html5lib description carries the literal `\uXXXX` escape text.
    "<\\uDBC0\\uDC00",
    // EOF reached mid-tag in the various tag/attribute states: the spec discards
    // the incomplete tag, we close it leniently.
    "Slash EOF in tag name",
    "EOF in tag name state ",
    "EOF in before attribute name state",
    "EOF in attribute name state",
    "EOF in after attribute name state",
    "EOF in before attribute value state",
    "EOF in attribute value (double quoted) state",
    "EOF in attribute value (single quoted) state",
    "EOF in attribute value (unquoted) state",
    "EOF in after attribute value state",
];

struct Summary {
    matched: usize,
    out_of_scope: usize,
    known_divergences: usize,
    mismatched: Vec<String>,
}

fn run_file(path: &PathBuf, summary: &mut Summary) {
    let data = std::fs::read_to_string(path).expect("read test file");
    let json: Value = serde_json::from_str(&data).expect("valid test JSON");
    let tests = match json.get("tests").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return,
    };

    for test in tests {
        // Only the default data state; alternate initial states are out of scope.
        if test.get("initialStates").is_some() {
            summary.out_of_scope += 1;
            continue;
        }
        if test.get("doubleEscaped").and_then(|v| v.as_bool()) == Some(true) {
            summary.out_of_scope += 1;
            continue;
        }
        let input = match test.get("input").and_then(|v| v.as_str()) {
            Some(i) => i,
            None => continue,
        };
        let expected = match test.get("output").and_then(parse_expected) {
            Some(e) => e,
            None => {
                summary.out_of_scope += 1;
                continue;
            }
        };

        if out_of_scope_reason(input, &expected).is_some() {
            summary.out_of_scope += 1;
            continue;
        }

        let desc = test
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("<no description>");

        let got = map_tokens(input, &tokenize(input));
        if got == expected {
            summary.matched += 1;
        } else if KNOWN_DIVERGENCES.contains(&desc) {
            summary.known_divergences += 1;
        } else {
            summary.mismatched.push(format!(
                "{}\n  input:    {:?}\n  expected: {:?}\n  got:      {:?}",
                desc, input, expected, got
            ));
        }
    }
}

fn tokenizer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("html5lib")
        .join("tokenizer")
}

#[test]
fn html5lib_tokenizer_suite() {
    let dir = tokenizer_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read tokenizer dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "test").unwrap_or(false))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no vendored tokenizer test files found");

    let mut summary = Summary {
        matched: 0,
        out_of_scope: 0,
        known_divergences: 0,
        mismatched: Vec::new(),
    };
    for f in &files {
        run_file(f, &mut summary);
    }

    eprintln!(
        "html5lib tokenizer: {} matched, {} out-of-scope, {} known divergences, {} unexpected mismatches (across {} files)",
        summary.matched,
        summary.out_of_scope,
        summary.known_divergences,
        summary.mismatched.len(),
        files.len()
    );

    if !summary.mismatched.is_empty() {
        let shown: Vec<_> = summary.mismatched.iter().take(15).collect();
        panic!(
            "{} in-scope tokenizer mismatches:\n\n{}",
            summary.mismatched.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        );
    }

    // Guard against accidental scope collapse: the in-scope subset must remain
    // substantial.
    assert!(
        summary.matched >= 400,
        "expected a large in-scope subset, only {} matched",
        summary.matched
    );
}
