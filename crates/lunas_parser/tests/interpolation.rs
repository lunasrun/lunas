//! Deep edge-case coverage for the `${…}` interpolation scanner
//! (`src/template/scan.rs`), driven through the public `parse` API.
//!
//! The scanner is brace/string/regex/comment aware; these tests pin down the
//! trickier balancing behavior and the never-panic recovery paths.

use lunas_parser::{
    parse, Diagnostic, Severity, TemplateAttr, TemplateElement, TemplateNode, TextSegment,
};

/// Wrap an html body line so it forms a valid `.lunas` file.
fn html(body: &str) -> String {
    format!("html:\n    {}\n", body)
}

fn parse_template(src: &str) -> (Vec<TemplateNode>, Vec<Diagnostic>) {
    let (file, diags) = parse(src);
    (file.html.expect("html block").template.nodes, diags)
}

fn nodes_ok(src: &str) -> Vec<TemplateNode> {
    let (ns, diags) = parse_template(src);
    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "unexpected errors: {:?}",
        diags
    );
    ns
}

fn first_element(nodes: &[TemplateNode]) -> &TemplateElement {
    nodes
        .iter()
        .find_map(|n| match n {
            TemplateNode::Element(e) => Some(e),
            _ => None,
        })
        .expect("an element")
}

/// The first text-child interpolation expression under the first element.
fn first_interp(src: &str) -> String {
    let ns = nodes_ok(src);
    let el = first_element(&ns);
    for child in &el.children {
        if let TemplateNode::Text(t) = child {
            for seg in &t.segments {
                if let TextSegment::Interpolation(i) = seg {
                    return i.expr.clone();
                }
            }
        }
    }
    panic!("no interpolation found in {:?}", el.children);
}

/// Count interpolations across the first element's direct text children.
fn interp_count(src: &str) -> usize {
    let ns = nodes_ok(src);
    let el = first_element(&ns);
    el.children
        .iter()
        .filter_map(|c| match c {
            TemplateNode::Text(t) => Some(t),
            _ => None,
        })
        .flat_map(|t| t.segments.iter())
        .filter(|s| matches!(s, TextSegment::Interpolation(_)))
        .count()
}

// --- Nested braces ---

#[test]
fn deeply_nested_object_braces() {
    assert_eq!(
        first_interp(&html("<div>${ {a:{b:{c:1}}}.a }</div>")),
        " {a:{b:{c:1}}}.a "
    );
}

#[test]
fn arrow_function_with_block_body() {
    assert_eq!(
        first_interp(&html("<div>${ (() => { return 1 })() }</div>")),
        " (() => { return 1 })() "
    );
}

#[test]
fn object_spread_in_call() {
    assert_eq!(
        first_interp(&html("<div>${ f({...a, b:1}) }</div>")),
        " f({...a, b:1}) "
    );
}

// --- Strings containing braces / delimiters ---

#[test]
fn double_quoted_string_with_open_brace() {
    assert_eq!(
        first_interp(&html("<div>${ \"{\" + x }</div>")),
        " \"{\" + x "
    );
}

#[test]
fn single_quoted_string_with_close_brace() {
    assert_eq!(first_interp(&html("<div>${ '}' }</div>")), " '}' ");
}

#[test]
fn string_with_escaped_quote_and_brace() {
    // Escaped quote inside the string must not terminate it early.
    assert_eq!(
        first_interp(&html("<div>${ \"a\\\"}\\\"b\" }</div>")),
        " \"a\\\"}\\\"b\" "
    );
}

#[test]
fn string_with_backslash_before_close_delim() {
    assert_eq!(
        first_interp(&html("<div>${ '\\\\' + y }</div>")),
        " '\\\\' + y "
    );
}

// --- Template literals with ${} substitutions ---

#[test]
fn template_literal_with_brace_in_substitution() {
    assert_eq!(
        first_interp(&html("<div>${ `x${ {a:1}.a }y` }</div>")),
        " `x${ {a:1}.a }y` "
    );
}

#[test]
fn nested_template_literals() {
    // Template inside a template substitution.
    assert_eq!(
        first_interp(&html("<div>${ `a${`b${c}d`}e` }</div>")),
        " `a${`b${c}d`}e` "
    );
}

#[test]
fn template_literal_with_string_containing_backtick_close() {
    assert_eq!(
        first_interp(&html("<div>${ `pre${ q ? '}' : '{' }post` }</div>")),
        " `pre${ q ? '}' : '{' }post` "
    );
}

// --- Regex literals ---

#[test]
fn regex_with_close_brace() {
    assert_eq!(
        first_interp(&html("<div>${ /}/.test(x) }</div>")),
        " /}/.test(x) "
    );
}

#[test]
fn regex_char_class_with_braces() {
    assert_eq!(
        first_interp(&html("<div>${ /[{}]/.exec(s) }</div>")),
        " /[{}]/.exec(s) "
    );
}

#[test]
fn regex_with_escaped_slash() {
    assert_eq!(
        first_interp(&html("<div>${ /a\\/b}/.test(x) }</div>")),
        " /a\\/b}/.test(x) "
    );
}

#[test]
fn regex_with_flags() {
    assert_eq!(
        first_interp(&html("<div>${ s.replace(/}/gi, '') }</div>")),
        " s.replace(/}/gi, '') "
    );
}

#[test]
fn division_after_identifier_is_not_regex() {
    // `a` is a value, so `/` is division; the `}` after still closes.
    assert_eq!(
        first_interp(&html("<div>${ total / count }</div>")),
        " total / count "
    );
}

#[test]
fn division_after_paren_is_not_regex() {
    assert_eq!(
        first_interp(&html("<div>${ (a + b) / 2 }</div>")),
        " (a + b) / 2 "
    );
}

// --- Comments inside interpolations ---

#[test]
fn line_comment_inside_interpolation() {
    assert_eq!(
        first_interp(&html("<div>${ a // }not-this\n }</div>")),
        " a // }not-this\n "
    );
}

#[test]
fn block_comment_with_brace() {
    assert_eq!(
        first_interp(&html("<div>${ a /* } */ + b }</div>")),
        " a /* } */ + b "
    );
}

#[test]
fn block_comment_with_star_slash_sequences() {
    assert_eq!(
        first_interp(&html("<div>${ x /* ** */ }</div>")),
        " x /* ** */ "
    );
}

// --- Empty / whitespace-only ---

#[test]
fn empty_interpolation_warns_not_errors() {
    let (_ns, diags) = parse_template(&html("<div>${}</div>"));
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("empty interpolation")));
    assert!(!diags.iter().any(|d| d.is_error()));
}

#[test]
fn whitespace_only_interpolation_warns() {
    let (_ns, diags) = parse_template(&html("<div>${   }</div>"));
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("empty interpolation")));
}

// --- Unterminated ---

#[test]
fn unterminated_reports_error_and_recovers() {
    let (ns, diags) = parse_template(&html("<div>${ a + b </div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("unterminated")));
    // Tree still built.
    assert!(!ns.is_empty());
}

#[test]
fn unterminated_by_unbalanced_open_brace() {
    // The inner `{` is never closed, so the whole `${` is unterminated.
    let (_ns, diags) = parse_template(&html("<div>${ {a: 1 }</div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("unterminated")));
}

#[test]
fn unterminated_string_inside_interpolation_is_unterminated() {
    let (_ns, diags) = parse_template(&html("<div>${ \"no close }</div>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("unterminated")));
}

#[test]
fn bare_dollar_without_brace_is_literal() {
    // A `$` not followed by `{` is ordinary text — no interpolation, no error.
    let (_ns, diags) = parse_template(&html("<div>price: $5</div>"));
    assert_eq!(interp_count(&html("<div>price: $5</div>")), 0);
    assert!(!diags.iter().any(|d| d.is_error()));
}

// --- Adjacency ---

#[test]
fn three_adjacent_interpolations() {
    assert_eq!(interp_count(&html("<div>${a}${b}${c}</div>")), 3);
}

#[test]
fn interpolations_separated_by_single_char() {
    assert_eq!(interp_count(&html("<div>${a}/${b}</div>")), 2);
}

#[test]
fn interpolation_at_start_and_end_of_run() {
    let ns = nodes_ok(&html("<div>${a} mid ${b}</div>"));
    let el = first_element(&ns);
    let text = match &el.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    // interp, literal, interp
    assert!(matches!(text.segments[0], TextSegment::Interpolation(_)));
    assert!(matches!(text.segments[1], TextSegment::Literal { .. }));
    assert!(matches!(text.segments[2], TextSegment::Interpolation(_)));
}

// --- Interpolation in attributes ---

#[test]
fn interpolation_in_static_attribute_with_braces() {
    let ns = nodes_ok(&html("<div data-x=\"${ {k:1}.k }\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let segs = &value.as_ref().unwrap().segments;
            assert!(segs
                .iter()
                .any(|s| matches!(s, TextSegment::Interpolation(i) if i.expr == " {k:1}.k ")));
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn interpolation_in_attribute_with_regex() {
    let ns = nodes_ok(&html("<div title=\"${ /}/.source }\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let segs = &value.as_ref().unwrap().segments;
            assert!(segs
                .iter()
                .any(|s| matches!(s, TextSegment::Interpolation(i) if i.expr == " /}/.source ")));
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn multiple_interpolations_in_single_attr() {
    let ns = nodes_ok(&html("<div class=\"${a} x ${b} y ${c}\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let n = value
                .as_ref()
                .unwrap()
                .segments
                .iter()
                .filter(|s| matches!(s, TextSegment::Interpolation(_)))
                .count();
            assert_eq!(n, 3);
        }
        other => panic!("got {:?}", other),
    }
}

// --- Span invariants ---

#[test]
fn interp_ranges_slice_back_to_source() {
    let src = html("<div>a ${ x + y } b</div>");
    let (file, _) = parse(&src);
    let ns = file.html.as_ref().unwrap().template.nodes.clone();
    let el = first_element(&ns);
    let text = match &el.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    for seg in &text.segments {
        if let TextSegment::Interpolation(i) = seg {
            // The inner expr_range slices to the expr; the outer range wraps `${…}`.
            assert_eq!(i.expr_range.slice(&src), Some(i.expr.as_str()));
            let whole = i.range.slice(&src).unwrap();
            assert!(whole.starts_with("${") && whole.ends_with('}'));
            // The inner range is strictly contained in the outer range.
            assert!(i.range.start() < i.expr_range.start());
            assert!(i.expr_range.end() < i.range.end());
        }
    }
}

#[test]
fn literal_segment_ranges_slice_back() {
    let src = html("<div>hello ${x} world</div>");
    let (file, _) = parse(&src);
    let ns = file.html.as_ref().unwrap().template.nodes.clone();
    let el = first_element(&ns);
    let text = match &el.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    for seg in &text.segments {
        if let TextSegment::Literal { text, range } = seg {
            assert_eq!(range.slice(&src), Some(text.as_str()));
        }
    }
}

// --- `<` / `>` inside interpolations (must not be read as HTML tags) ---

#[test]
fn less_than_inside_interpolation() {
    assert_eq!(first_interp(&html("<div>${a < b}</div>")), "a < b");
}

#[test]
fn greater_than_inside_interpolation() {
    assert_eq!(first_interp(&html("<div>${a > b}</div>")), "a > b");
}

#[test]
fn less_than_or_equal_with_logical_and() {
    assert_eq!(
        first_interp(&html("<div>${a <= b && c}</div>")),
        "a <= b && c"
    );
}

#[test]
fn left_shift_inside_interpolation() {
    assert_eq!(first_interp(&html("<div>${x << 2}</div>")), "x << 2");
}

#[test]
fn both_angle_brackets_in_call_args() {
    // `f(a<b, c>d)` — two comparisons in one call; neither angle is markup.
    assert_eq!(
        first_interp(&html("<div>${f(a<b, c>d)}</div>")),
        "f(a<b, c>d)"
    );
}

#[test]
fn angle_brackets_inside_string_in_ternary() {
    // The `"<x>"` string literal inside the interpolation must survive verbatim.
    assert_eq!(
        first_interp(&html("<div>${cond ? \"<x>\" : \"\"}</div>")),
        "cond ? \"<x>\" : \"\""
    );
}

#[test]
fn angle_bracket_operators_produce_no_errors() {
    for body in [
        "<div>${a < b}</div>",
        "<div>${a > b}</div>",
        "<div>${a <= b && c}</div>",
        "<div>${x << 2}</div>",
        "<div>${f(a<b, c>d)}</div>",
        "<div>${cond ? \"<x>\" : \"\"}</div>",
    ] {
        let (_ns, diags) = parse_template(&html(body));
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "unexpected errors for {body:?}: {diags:?}"
        );
    }
}

#[test]
fn less_than_inside_attribute_interpolation() {
    // `<` inside an interpolation in a *quoted attribute value* must not close
    // the value or start a tag either.
    let ns = nodes_ok(&html("<div title=\"${a < b ? 'x' : 'y'}\"></div>"));
    let el = first_element(&ns);
    match &el.attrs[0] {
        TemplateAttr::Static { value, .. } => {
            let segs = &value.as_ref().unwrap().segments;
            assert!(segs.iter().any(
                |s| matches!(s, TextSegment::Interpolation(i) if i.expr == "a < b ? 'x' : 'y'")
            ));
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn interp_with_angle_bracket_ranges_slice_back() {
    // Span fidelity is preserved by the length-preserving mask: the recorded
    // expr slices back to the exact original source (with the real `<`).
    let src = html("<div>${ a < b }</div>");
    let (file, diags) = parse(&src);
    assert!(!diags.iter().any(|d| d.is_error()), "{:?}", diags);
    let ns = file.html.as_ref().unwrap().template.nodes.clone();
    let el = first_element(&ns);
    let text = match &el.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    let interp = text
        .segments
        .iter()
        .find_map(|s| match s {
            TextSegment::Interpolation(i) => Some(i),
            _ => None,
        })
        .expect("interpolation");
    assert_eq!(interp.expr_range.slice(&src), Some(interp.expr.as_str()));
    assert_eq!(interp.expr, " a < b ");
}

#[test]
fn static_angle_brackets_outside_interpolation_stay_markup() {
    // Don't over-correct: a real `<x>` in static text is still tokenized as an
    // element, not swallowed as interpolation text.
    let ns = nodes_ok(&html("<div>a <x>b</x> c</div>"));
    let el = first_element(&ns);
    // The div has an inner <x> element child (proving `<x>` was markup).
    assert!(el
        .children
        .iter()
        .any(|c| matches!(c, TemplateNode::Element(e) if e.name == "x")));
}

// --- Multi-byte / unicode inside interpolations ---

#[test]
fn unicode_inside_interpolation_keeps_valid_spans() {
    let src = html("<div>${ 名前 + 'あ' }</div>");
    let (file, diags) = parse(&src);
    assert!(!diags.iter().any(|d| d.is_error()), "{:?}", diags);
    let ns = file.html.as_ref().unwrap().template.nodes.clone();
    let el = first_element(&ns);
    let text = match &el.children[0] {
        TemplateNode::Text(t) => t,
        other => panic!("expected text, got {:?}", other),
    };
    let interp = text
        .segments
        .iter()
        .find_map(|s| match s {
            TextSegment::Interpolation(i) => Some(i),
            _ => None,
        })
        .expect("interpolation");
    assert_eq!(interp.expr_range.slice(&src), Some(interp.expr.as_str()));
}
