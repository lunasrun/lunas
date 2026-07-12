//! Edge cases for directive parsing (`@use`, `@input`, `@useRouting`,
//! `@useAutoRouting`) and attribute classification (`:` / `::` / `@` / static),
//! driven through the public `parse` API. Malformed directives must produce
//! diagnostics, never panic.

use lunas_parser::{parse, Directive, Severity, TemplateAttr, TemplateElement, TemplateNode};

fn parse_ok(src: &str) -> lunas_parser::ParsedFile {
    let (file, diags) = parse(src);
    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "unexpected errors for {src:?}: {:?}",
        diags
    );
    file
}

fn directives(src: &str) -> Vec<Directive> {
    parse(src).0.directives
}

fn html(body: &str) -> String {
    format!("html:\n    {}\n", body)
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

fn attrs_of(src: &str) -> Vec<TemplateAttr> {
    let file = parse_ok(src);
    first_element(&file.html.unwrap().template.nodes)
        .attrs
        .clone()
}

// --- @use forms ---

#[test]
fn use_inline_double_quotes() {
    let ds = directives("@use Button from \"./Button\"\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::UseComponent(u) => {
            assert_eq!(u.component_name, "Button");
            assert_eq!(u.path, "./Button");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn use_inline_single_quotes() {
    let ds = directives("@use Card from './widgets/Card.lunas'\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::UseComponent(u) => assert_eq!(u.path, "./widgets/Card.lunas"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn use_block_form_next_line() {
    // `@use()` header with the declaration on the following line.
    let ds = directives("@use()\nButton from \"./Button\"\n\nhtml:\n    <p/>\n");
    assert!(matches!(&ds[0], Directive::UseComponent(u) if u.component_name == "Button"));
}

#[test]
fn use_extra_whitespace_tolerated() {
    let ds = directives("@use   Widget   from   \"./W\"  \nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::UseComponent(u) => {
            assert_eq!(u.component_name, "Widget");
            assert_eq!(u.path, "./W");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn use_missing_from_errors() {
    let (_f, diags) = parse("@use Button \"./x\"\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("@use")));
}

#[test]
fn use_unquoted_path_errors() {
    let (_f, diags) = parse("@use Button from ./x\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("@use")));
}

#[test]
fn use_mismatched_quotes_errors() {
    let (_f, diags) = parse("@use Button from \"./x'\nhtml:\n    <p/>\n");
    assert!(diags.iter().any(|d| d.is_error()));
}

#[test]
fn use_empty_name_errors() {
    let (_f, diags) = parse("@use from \"./x\"\nhtml:\n    <p/>\n");
    assert!(diags.iter().any(|d| d.is_error()));
}

// --- @input forms ---

#[test]
fn input_name_only_is_error() {
    // No `:` type separator — invalid.
    let (_f, diags) = parse("@input count\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("@input")));
}

#[test]
fn input_type_and_default() {
    let ds = directives("@input count: number = 0\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::Input(p) => {
            assert_eq!(p.name, "count");
            assert_eq!(p.type_annotation.as_deref(), Some("number"));
            assert_eq!(p.default_value.as_deref(), Some("0"));
            assert!(!p.nullable);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn input_nullable_marker() {
    let ds = directives("@input name: string?\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::Input(p) => {
            assert!(p.nullable);
            assert_eq!(p.type_annotation.as_deref(), Some("string"));
            assert_eq!(p.default_value, None);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn input_nullable_with_default() {
    let ds = directives("@input name: string? = \"x\"\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::Input(p) => {
            assert!(p.nullable);
            assert_eq!(p.default_value.as_deref(), Some("\"x\""));
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn input_no_type_after_colon() {
    // `name:` with nothing after — type is None, still valid.
    let ds = directives("@input flag:\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::Input(p) => {
            assert_eq!(p.name, "flag");
            assert_eq!(p.type_annotation, None);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn input_default_with_equals_in_value() {
    // The default value itself contains `=` (comparison / arrow).
    let ds = directives("@input cmp: string = a === b\nhtml:\n    <p/>\n");
    match &ds[0] {
        Directive::Input(p) => {
            // split_once('=') keeps everything after the first `=` as the default.
            assert_eq!(p.type_annotation.as_deref(), Some("string"));
            assert!(p.default_value.as_deref().unwrap().contains("=="));
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn input_invalid_ident_name_errors() {
    let (_f, diags) = parse("@input 1bad: number\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("@input")));
}

#[test]
fn input_empty_name_errors() {
    let (_f, diags) = parse("@input : number\nhtml:\n    <p/>\n");
    assert!(diags.iter().any(|d| d.is_error()));
}

#[test]
fn multiple_inputs_all_captured() {
    let src = "@input a: number\n@input b: string?\n@input c: boolean = true\nhtml:\n    <p/>\n";
    let ds = directives(src);
    let inputs = ds
        .iter()
        .filter(|d| matches!(d, Directive::Input(_)))
        .count();
    assert_eq!(inputs, 3);
}

// --- Routing directives ---

#[test]
fn use_routing_directive() {
    let ds = directives("@useRouting\nhtml:\n    <p/>\n");
    assert!(matches!(&ds[0], Directive::UseRouting));
}

#[test]
fn use_auto_routing_directive() {
    let ds = directives("@useAutoRouting\nhtml:\n    <p/>\n");
    assert!(matches!(&ds[0], Directive::UseAutoRouting));
}

#[test]
fn unknown_directive_warns_and_is_dropped() {
    let (file, diags) = parse("@somethingElse foo\nhtml:\n    <p/>\n");
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("unknown directive")));
    // Unknown directives are not surfaced in the directive list.
    assert!(file.directives.is_empty());
}

// --- Attribute classification ---

#[test]
fn bound_attribute_strips_single_colon() {
    let a = attrs_of(&html("<input :value=\"title\" />"));
    assert!(
        matches!(&a[0], TemplateAttr::Bound { name, expr, .. } if name == "value" && expr.text == "title")
    );
}

#[test]
fn two_way_attribute_strips_double_colon() {
    let a = attrs_of(&html("<input ::value=\"title\" />"));
    assert!(matches!(&a[0], TemplateAttr::TwoWay { name, .. } if name == "value"));
}

#[test]
fn event_attribute_strips_at() {
    let a = attrs_of(&html("<button @click=\"go()\">x</button>"));
    assert!(
        matches!(&a[0], TemplateAttr::Event { event, handler, .. } if event == "click" && handler.text == "go()")
    );
}

#[test]
fn double_colon_takes_precedence_over_single() {
    // `::` must classify as TwoWay, not Bound with a leading `:` name.
    let a = attrs_of(&html("<input ::checked=\"on\" />"));
    assert!(matches!(&a[0], TemplateAttr::TwoWay { name, .. } if name == "checked"));
}

#[test]
fn event_with_dotted_name_preserved() {
    let a = attrs_of(&html("<div @keydown.enter=\"go\"></div>"));
    assert!(matches!(&a[0], TemplateAttr::Event { event, .. } if event == "keydown.enter"));
}

#[test]
fn bound_attribute_with_expression_value() {
    let a = attrs_of(&html("<div :class=\"on ? 'a' : 'b'\"></div>"));
    assert!(matches!(&a[0], TemplateAttr::Bound { expr, .. } if expr.text == "on ? 'a' : 'b'"));
}

#[test]
fn static_boolean_attr_no_value() {
    let a = attrs_of(&html("<input disabled />"));
    assert!(
        matches!(&a[0], TemplateAttr::Static { name, value, .. } if name == "disabled" && value.is_none())
    );
}

#[test]
fn mixed_static_and_bound_and_event() {
    let a = attrs_of(&html(
        "<button id=\"b\" :class=\"c\" @click=\"go\" disabled>x</button>",
    ));
    assert!(a
        .iter()
        .any(|x| matches!(x, TemplateAttr::Static { name, .. } if name == "id")));
    assert!(a
        .iter()
        .any(|x| matches!(x, TemplateAttr::Bound { name, .. } if name == "class")));
    assert!(a
        .iter()
        .any(|x| matches!(x, TemplateAttr::Event { event, .. } if event == "click")));
    assert!(a.iter().any(|x| matches!(x, TemplateAttr::Static { name, value, .. } if name == "disabled" && value.is_none())));
}

#[test]
fn reserved_bound_names_error() {
    for name in ["innerHtml", "textContent", "INNERHTML"] {
        let (_f, diags) = parse(&html(&format!("<div :{name}=\"x\"></div>")));
        assert!(
            diags
                .iter()
                .any(|d| d.is_error() && d.message.contains("not supported")),
            "expected error for :{name}"
        );
    }
}

#[test]
fn bound_without_value_errors() {
    let (_f, diags) = parse(&html("<input :value />"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("expects an expression")));
}

#[test]
fn event_without_value_errors() {
    let (_f, diags) = parse(&html("<button @click>x</button>"));
    assert!(diags
        .iter()
        .any(|d| d.is_error() && d.message.contains("expects an expression")));
}

#[test]
fn duplicate_static_attribute_warns() {
    // The HTML parser deduplicates; the diagnostic bubbles up through `parse`.
    let (_f, diags) = parse(&html("<div id=\"a\" id=\"b\"></div>"));
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("duplicate attribute")));
}

#[test]
fn directive_ranges_are_file_absolute() {
    let src = "@input count: number = 0\nhtml:\n    <p/>\n";
    let (file, _) = parse(src);
    match &file.directives[0] {
        Directive::Input(p) => {
            // The stored range slices back to the directive body text.
            let sliced = p.range.slice(src).expect("in-bounds range");
            assert!(sliced.contains("count"));
        }
        other => panic!("got {other:?}"),
    }
}
