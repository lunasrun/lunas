//! Lexer unit tests, exercising the tokenizer through the hidden
//! `internals` facade.

use lunas_html_parser::internals::{tokenize, TokenKind};
use lunas_span::TextRange;

fn slice(source: &str, r: TextRange) -> &str {
    r.slice(source).expect("range on char boundary")
}

#[test]
fn empty_input() {
    assert!(tokenize("").is_empty());
}

#[test]
fn plain_text() {
    let toks = tokenize("hello");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::Text);
    assert_eq!(slice("hello", toks[0].range), "hello");
}

#[test]
fn simple_open_close() {
    let toks = tokenize("<div></div>");
    assert!(matches!(toks[0].kind, TokenKind::OpenTagStart { .. }));
    assert_eq!(toks[1].kind, TokenKind::OpenTagEnd);
    assert!(matches!(toks[2].kind, TokenKind::CloseTag { .. }));
}

#[test]
fn open_tag_name_range() {
    let src = "<div>";
    let toks = tokenize(src);
    if let TokenKind::OpenTagStart { name } = toks[0].kind {
        assert_eq!(slice(src, name), "div");
    } else {
        panic!("expected open tag start");
    }
}

#[test]
fn self_closing() {
    let toks = tokenize("<br/>");
    assert!(matches!(toks[0].kind, TokenKind::OpenTagStart { .. }));
    assert_eq!(toks[1].kind, TokenKind::SelfCloseTagEnd);
}

#[test]
fn self_closing_with_space() {
    let toks = tokenize("<Foo />");
    assert_eq!(toks.last().expect("token").kind, TokenKind::SelfCloseTagEnd);
}

#[test]
fn attribute_boolean() {
    let src = "<input disabled>";
    let toks = tokenize(src);
    if let TokenKind::Attribute { name, value } = toks[1].kind {
        assert_eq!(slice(src, name), "disabled");
        assert!(value.is_none());
    } else {
        panic!("expected attribute");
    }
}

#[test]
fn attribute_double_quoted() {
    let src = "<a href=\"x\">";
    let toks = tokenize(src);
    if let TokenKind::Attribute { name, value } = toks[1].kind {
        assert_eq!(slice(src, name), "href");
        assert_eq!(slice(src, value.expect("value")), "x");
    } else {
        panic!();
    }
}

#[test]
fn attribute_single_quoted() {
    let src = "<a href='x'>";
    let toks = tokenize(src);
    if let TokenKind::Attribute { value, .. } = toks[1].kind {
        assert_eq!(slice(src, value.expect("value")), "x");
    } else {
        panic!();
    }
}

#[test]
fn attribute_unquoted() {
    let src = "<a href=x>";
    let toks = tokenize(src);
    if let TokenKind::Attribute { value, .. } = toks[1].kind {
        assert_eq!(slice(src, value.expect("value")), "x");
    } else {
        panic!();
    }
}

#[test]
fn attribute_whitespace_around_eq() {
    let src = "<a href = \"x\">";
    let toks = tokenize(src);
    if let TokenKind::Attribute { value, .. } = toks[1].kind {
        assert_eq!(slice(src, value.expect("value")), "x");
    } else {
        panic!();
    }
}

#[test]
fn attribute_value_with_gt() {
    let src = "<a t=\"a>b\">";
    let toks = tokenize(src);
    if let TokenKind::Attribute { value, .. } = toks[1].kind {
        assert_eq!(slice(src, value.expect("value")), "a>b");
    } else {
        panic!();
    }
}

#[test]
fn empty_attribute_value() {
    let src = "<a x=\"\">";
    let toks = tokenize(src);
    if let TokenKind::Attribute { value, .. } = toks[1].kind {
        assert_eq!(slice(src, value.expect("value")), "");
    } else {
        panic!();
    }
}

#[test]
fn close_tag_with_whitespace() {
    let src = "</ div >";
    let toks = tokenize(src);
    if let TokenKind::CloseTag { name } = toks[0].kind {
        assert_eq!(slice(src, name), "div");
    } else {
        panic!();
    }
}

#[test]
fn comment_content() {
    let src = "<!-- hi -->";
    let toks = tokenize(src);
    if let TokenKind::Comment { content } = toks[0].kind {
        assert_eq!(slice(src, content), " hi ");
    } else {
        panic!();
    }
}

#[test]
fn empty_comment() {
    let src = "<!---->";
    let toks = tokenize(src);
    if let TokenKind::Comment { content } = toks[0].kind {
        assert_eq!(slice(src, content), "");
    } else {
        panic!();
    }
}

#[test]
fn unterminated_comment() {
    let src = "<!-- oops";
    let toks = tokenize(src);
    if let TokenKind::Comment { content } = toks[0].kind {
        assert_eq!(slice(src, content), " oops");
    } else {
        panic!();
    }
}

#[test]
fn doctype() {
    let toks = tokenize("<!DOCTYPE html>");
    assert_eq!(toks[0].kind, TokenKind::Doctype);
}

#[test]
fn doctype_lowercase() {
    let toks = tokenize("<!doctype html>");
    assert_eq!(toks[0].kind, TokenKind::Doctype);
}

#[test]
fn raw_text_script() {
    let src = "<script>if (a < b) {}</script>";
    let toks = tokenize(src);
    let raw = toks
        .iter()
        .find(|t| t.kind == TokenKind::RawText)
        .expect("raw text token");
    assert_eq!(slice(src, raw.range), "if (a < b) {}");
}

#[test]
fn raw_text_with_fake_close() {
    let src = "<script></div></script>";
    let toks = tokenize(src);
    let raw = toks
        .iter()
        .find(|t| t.kind == TokenKind::RawText)
        .expect("raw text token");
    assert_eq!(slice(src, raw.range), "</div>");
}

#[test]
fn raw_text_no_premature_match() {
    let src = "<script>x</scriptable>y</script>";
    let toks = tokenize(src);
    let raw = toks
        .iter()
        .find(|t| t.kind == TokenKind::RawText)
        .expect("raw text token");
    assert_eq!(slice(src, raw.range), "x</scriptable>y");
}

#[test]
fn stray_lt_is_text() {
    let toks = tokenize("a < b");
    assert!(toks.iter().all(|t| t.kind == TokenKind::Text));
}

#[test]
fn unicode_text_boundaries() {
    let src = "<p>あいう</p>";
    let toks = tokenize(src);
    let text = toks
        .iter()
        .find(|t| t.kind == TokenKind::Text)
        .expect("text token");
    assert_eq!(slice(src, text.range), "あいう");
}
