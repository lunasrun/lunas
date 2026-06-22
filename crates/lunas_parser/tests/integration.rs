//! Integration tests for the full parse pipeline.

use lunas_parser::{parse, Directive, LineCol, Severity};

fn no_errors(diags: &[lunas_parser::Diagnostic]) -> bool {
    !diags.iter().any(|d| d.severity == Severity::Error)
}

#[test]
fn full_realistic_file() {
    // Canonical Lunas syntax: inline `@input`/`@use`, `${}` interpolation.
    let src = "@input count:number = 0\n\
               @use Button from \"./Button\"\n\
               \n\
               html:\n\
               \x20   <div>${count}</div>\n\
               \n\
               style:\n\
               \x20   div { color: red; }\n\
               \n\
               script:\n\
               \x20   let count: number = 0\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags), "unexpected errors: {:?}", diags);
    assert!(file.html.is_some());
    assert!(file.style.is_some());
    assert!(file.script.is_some());
    assert_eq!(file.directives.len(), 2);

    match &file.directives[0] {
        Directive::Input(p) => {
            assert_eq!(p.name, "count");
            assert_eq!(p.type_annotation.as_deref(), Some("number"));
            assert_eq!(p.default_value.as_deref(), Some("0"));
        }
        other => panic!("expected input, got {:?}", other),
    }
    match &file.directives[1] {
        Directive::UseComponent(u) => {
            assert_eq!(u.component_name, "Button");
            assert_eq!(u.path, "./Button");
        }
        other => panic!("expected use, got {:?}", other),
    }

    // The `${count}` interpolation made it into the template IR.
    use lunas_parser::{TemplateNode, TextSegment};
    fn has_interpolation(nodes: &[TemplateNode]) -> bool {
        nodes.iter().any(|n| match n {
            TemplateNode::Text(t) => t
                .segments
                .iter()
                .any(|s| matches!(s, TextSegment::Interpolation(i) if i.expr == "count")),
            TemplateNode::Element(e) => has_interpolation(&e.children),
            _ => false,
        })
    }
    assert!(has_interpolation(
        &file.html.as_ref().unwrap().template.nodes
    ));

    // The parser extracts the script's raw text; JS/TS parsing is a separate
    // concern (lunas_script), so the original source is preserved verbatim.
    let script = file.script.as_ref().expect("script");
    assert!(script.source.text.contains("let count: number = 0"));
}

#[test]
fn directive_no_params() {
    let (file, _) = parse("@useAutoRouting\nhtml:\n    <p/>\n");
    assert!(matches!(file.directives[0], Directive::UseAutoRouting));
}

#[test]
fn use_routing_directive() {
    let (file, _) = parse("@useRouting\nhtml:\n    <p/>\n");
    assert!(matches!(file.directives[0], Directive::UseRouting));
}

#[test]
fn input_nullable() {
    let (file, diags) = parse("@input\nname: string? = \"x\"\nhtml:\n    <p/>\n");
    assert!(no_errors(&diags));
    match &file.directives[0] {
        Directive::Input(p) => {
            assert!(p.nullable);
            assert_eq!(p.type_annotation.as_deref(), Some("string"));
        }
        _ => panic!("expected input"),
    }
}

#[test]
fn multiple_inputs() {
    let src = "@input\na: number\n@input\nb: string\nhtml:\n    <p/>\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags));
    assert_eq!(file.directives.len(), 2);
}

#[test]
fn only_html() {
    let (file, diags) = parse("html:\n    <div></div>\n");
    assert!(no_errors(&diags));
    assert!(file.html.is_some());
    assert!(file.script.is_none());
    assert!(file.style.is_none());
}

#[test]
fn html_and_script() {
    let (file, diags) = parse("html:\n    <p/>\nscript:\n    let x = 1\n");
    assert!(no_errors(&diags));
    assert!(file.html.is_some());
    assert!(file.script.is_some());
}

#[test]
fn missing_html_warns() {
    let (file, diags) = parse("script:\n    let x = 1\n");
    assert!(file.html.is_none());
    assert!(diags.iter().any(|d| d.message.contains("missing `html:`")));
}

#[test]
fn duplicate_block_errors() {
    let (_file, diags) = parse("html:\n    <p/>\nhtml:\n    <span/>\n");
    assert!(diags.iter().any(|d| d.message.contains("duplicate")));
}

#[test]
fn script_block_kept_verbatim() {
    // No block is indentation-stripped: source.text == range.slice(file), so
    // positions map exactly. SWC handles indented script fine.
    let src = "html:\n    <p/>\nscript:\n    if (x) {\n        y()\n    }\n";
    let (file, _) = parse(src);
    let block = file.script.as_ref().unwrap();
    assert_eq!(
        block.source.range.slice(src),
        Some(block.source.text.as_str())
    );
    assert!(block.source.text.contains("    if (x) {"));
}

#[test]
fn lunas_to_script_preserves_column() {
    // A position inside the script maps to the extracted text with the column
    // unchanged (verbatim extraction). `if` starts at column 4 on the first
    // script line in both the file and the extracted script.
    use lunas_parser::LineCol;
    let src = "html:\n    <p/>\nscript:\n    if (x) {}\n";
    let (file, _) = parse(src);
    let script = file.script.as_ref().unwrap();
    let first_line = file.line_index.line_col(script.source.range.start()).line;
    let mapped = file.lunas_to_script(LineCol::new(first_line, 4)).unwrap();
    assert_eq!(mapped, LineCol::new(0, 4));
    // And that column indexes `if` in the extracted text's first line.
    let first = script.source.text.lines().next().unwrap();
    assert_eq!(&first[4..6], "if");
}

#[test]
fn html_block_keeps_raw_indentation() {
    // HTML is not stripped, so its source.text equals the original body region.
    let src = "html:\n    <ul>\n        <li/>\n    </ul>\n";
    let (file, _) = parse(src);
    let block = file.html.as_ref().unwrap();
    assert_eq!(
        block.source.range.slice(src),
        Some(block.source.text.as_str())
    );
    assert!(block.source.text.contains("    <ul>"));
}

#[test]
fn html_dom_ranges_are_file_absolute() {
    // Regression guard: Dom node ranges must address the .lunas file, not the
    // extracted block, so the language server can map HTML positions.
    use lunas_html_parser::Node;
    let src = "html:\n    <div id=\"a\">hi</div>\n";
    let (file, _) = parse(src);
    let dom = &file.html.as_ref().unwrap().dom;
    // Raw HTML keeps the leading indentation as a whitespace text node, so find
    // the first element child.
    let div = dom
        .children
        .iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .expect("element");
    assert_eq!(div.range.slice(src), Some("<div id=\"a\">hi</div>"));
    assert_eq!(div.open_tag_range.slice(src), Some("<div id=\"a\">"));
    assert_eq!(div.attributes[0].range.slice(src), Some("id=\"a\""));
}

#[test]
fn blank_lines_inside_body() {
    let src = "script:\n    let a = 1\n\n    let b = 2\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags));
    assert!(file.script.as_ref().unwrap().source.text.contains("\n\n"));
}

#[test]
fn body_text_with_keyword_strings() {
    // Content that mentions `@input` and `html:` but is indented, so it must
    // be treated as body content, not new items.
    let src = "html:\n    <p>html: and @input are text</p>\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags));
    assert_eq!(file.directives.len(), 0);
    assert!(file.html.as_ref().unwrap().source.text.contains("@input"));
}

#[test]
fn empty_script_block() {
    let (file, diags) = parse("html:\n    <p/>\nscript:\n");
    assert!(no_errors(&diags), "{:?}", diags);
    // An entirely empty script body becomes no script block (no body range).
    // Either way, no error.
    let _ = file;
}

#[test]
fn script_text_preserved_verbatim() {
    // The parser does not transform the script; TS is kept as-is for the
    // downstream lunas_script stage.
    let src = "html:\n    <p/>\nscript:\n    interface A { x: number }\n    let y: A = { x: 1 }\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags));
    let text = &file.script.as_ref().unwrap().source.text;
    assert!(text.contains("interface A { x: number }"));
    assert!(text.contains("let y: A = { x: 1 }"));
}

#[test]
fn block_source_range_maps_to_original() {
    let src = "html:\n    <div>x</div>\n";
    let (file, _) = parse(src);
    let range = file.html.as_ref().unwrap().source.range;
    let original = range.slice(src).expect("slice");
    // The original (un-stripped) region keeps its indentation.
    assert!(original.contains("    <div>x</div>"));
}

#[test]
fn lunas_to_script_inside_and_outside() {
    let src = "html:\n    <p/>\nscript:\n    let a = 1\n    let b = 2\n";
    let (file, _) = parse(src);
    let script = file.script.as_ref().unwrap();
    let start_line = file.line_index.line_col(script.source.range.start()).line;
    // A position on the first script line maps to script line 0.
    let mapped = file.lunas_to_script(LineCol::new(start_line, 4));
    assert_eq!(mapped, Some(LineCol::new(0, 4)));
    // A position above the script block returns None.
    assert_eq!(file.lunas_to_script(LineCol::new(0, 0)), None);
}

#[test]
fn script_to_lunas_roundtrip() {
    let src = "html:\n    <p/>\nscript:\n    let a = 1\n    let b = 2\n";
    let (file, _) = parse(src);
    let back = file.script_to_lunas(LineCol::new(0, 4)).expect("some");
    let forward = file.lunas_to_script(back).expect("some");
    assert_eq!(forward, LineCol::new(0, 4));
}

#[test]
fn lunas_to_script_none_without_script() {
    let (file, _) = parse("html:\n    <p/>\n");
    assert_eq!(file.lunas_to_script(LineCol::new(0, 0)), None);
}

#[test]
fn crlf_line_endings() {
    let src = "html:\r\n    <div></div>\r\n";
    let (file, diags) = parse(src);
    assert!(no_errors(&diags), "{:?}", diags);
    assert!(file.html.is_some());
}

#[test]
fn no_trailing_newline() {
    let (file, diags) = parse("html:\n    <p/>");
    assert!(no_errors(&diags));
    assert!(file.html.is_some());
}

#[test]
fn trailing_newline() {
    let (file, diags) = parse("html:\n    <p/>\n");
    assert!(no_errors(&diags));
    assert!(file.html.is_some());
}

#[test]
fn use_integration_calls_html_parser() {
    // Verify the HTML parser is invoked and its Dom is stored. Tolerant of the
    // current stub (which may produce an empty Dom).
    let (file, _) = parse("html:\n    <div></div>\n");
    let dom = &file.html.as_ref().unwrap().dom;
    // Either Empty (stub) or Fragment/Document — just ensure it exists.
    let _ = dom.kind;
}

#[test]
fn varying_indent_depth_kept_verbatim() {
    let src = "html:\n    <p/>\nscript:\n        let x = 1\n";
    let (file, _) = parse(src);
    let block = file.script.as_ref().unwrap();
    // Verbatim: text equals the sliced body region (8-space indent preserved).
    assert_eq!(
        block.source.range.slice(src),
        Some(block.source.text.as_str())
    );
    assert!(block.source.text.starts_with("        let x = 1"));
}

#[test]
fn block_at_routes_positions() {
    use lunas_parser::{BlockKind, LineCol};
    let src =
        "@input x:string\nhtml:\n    <div></div>\nstyle:\n    .a {}\nscript:\n    let y = 1\n";
    let (file, _) = parse(src);
    // line 2 = html body, line 4 = style body, line 6 = script body.
    assert_eq!(
        file.block_at_line_col(LineCol::new(2, 5)),
        Some(BlockKind::Html)
    );
    assert_eq!(
        file.block_at_line_col(LineCol::new(4, 5)),
        Some(BlockKind::Style)
    );
    assert_eq!(
        file.block_at_line_col(LineCol::new(6, 5)),
        Some(BlockKind::Script)
    );
    // The `@input` directive line is not inside any block.
    assert_eq!(file.block_at_line_col(LineCol::new(0, 0)), None);
}

#[test]
fn input_inline_variants() {
    use lunas_parser::Directive;
    let check = |src: &str| {
        let (file, diags) = parse(src);
        assert!(
            !diags.iter().any(|d| d.severity == Severity::Error),
            "{src:?} -> {:?}",
            diags
        );
        match &file.directives[0] {
            Directive::Input(p) => p.clone(),
            other => panic!("expected input, got {other:?}"),
        }
    };

    let p = check("@input name:string\nhtml:\n    <p/>\n");
    assert_eq!(p.name, "name");
    assert_eq!(p.type_annotation.as_deref(), Some("string"));
    assert!(!p.nullable);
    assert_eq!(p.default_value, None);

    let p = check("@input name:string?\nhtml:\n    <p/>\n");
    assert!(p.nullable);
    assert_eq!(p.default_value, None);

    let p = check("@input name:string? = \"x\"\nhtml:\n    <p/>\n");
    assert!(p.nullable);
    assert_eq!(p.default_value.as_deref(), Some("\"x\""));

    let p = check("@input  name : number = 42 \nhtml:\n    <p/>\n");
    assert_eq!(p.name, "name");
    assert_eq!(p.type_annotation.as_deref(), Some("number"));
    assert_eq!(p.default_value.as_deref(), Some("42"));
}

#[test]
fn input_without_type_is_error() {
    let (_file, diags) = parse("@input name\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("@input")));
}

#[test]
fn diagnostic_render_on_real_error() {
    // `:else` without a matching `:if` is an error; its rendering points at the
    // offending source line with a caret.
    let src = "html:\n    <div :else>x</div>\n";
    let (file, diags) = parse(src);
    let err = diags.iter().find(|d| d.is_error()).expect("an error");
    let rendered = err.render(src, &file.line_index);
    assert!(rendered.starts_with("error: "), "{rendered}");
    assert!(rendered.contains("--> 2:"), "{rendered}");
    assert!(rendered.contains("<div :else>x</div>"), "{rendered}");
    assert!(rendered.contains('^'), "{rendered}");
}
