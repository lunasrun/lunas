//! `parse` must never panic, for any input — the crate-wide never-panic
//! invariant. A large deterministic corpus of adversarial and pseudo-random
//! `.lunas`-shaped strings is thrown at the full pipeline (Pest grammar, HTML
//! parser, template pass).

use lunas_parser::parse;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn assert_no_panic(input: &str) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse(input);
    }));
    assert!(result.is_ok(), "parse panicked on input: {:?}", input);
}

#[test]
fn adversarial_lunas_files_do_not_panic() {
    let cases = [
        "",
        "\n",
        "html:",
        "html:\n",
        "html:\n    ",
        "html:\n    <",
        "html:\n    ${",
        "html:\n    ${}",
        "html:\n    ${ {a:1} ",
        "html:\n    <div :if=>x</div>",
        "html:\n    <div :for=\"\">x</div>",
        "html:\n    <div :else></div>",
        "html:\n    <div ::=\"x\"></div>",
        "html:\n    <div @=\"x\"></div>",
        "html:\n    <div :=\"x\"></div>",
        "@input",
        "@input ",
        "@input :",
        "@input x",
        "@use",
        "@use x",
        "@useAutoRouting extra",
        "@@@@",
        "@\n@\n@",
        "script:\n    let x: = =",
        "style:\n    }}}{{{",
        "html:\nhtml:\nhtml:",
        "html:\n    <div>${`a${`b${c}`}`}</div>",
        "html:\n    <Comp :if=\"a\" :for=\"b of c\" :else />",
        "\u{1F600}:\n    x",
        "html:\r\n    <div>\r\n    </div>\r\n",
        "html:\n\t\t\t<a><b><c>",
    ];
    for c in cases {
        assert_no_panic(c);
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

#[test]
fn pseudo_random_fuzz_does_not_panic() {
    // Tokens biased toward `.lunas` structural pieces.
    let fragments: &[&str] = &[
        "html:",
        "style:",
        "script:",
        "@input",
        "@use",
        "@useRouting",
        "\n",
        "    ",
        "\t",
        "<div>",
        "</div>",
        "<a ",
        "/>",
        ":if=",
        ":for=",
        ":else",
        "::v=",
        "@click=",
        "\"x\"",
        "'y'",
        "${",
        "}",
        "${count}",
        "<Comp",
        "from",
        "\"./p\"",
        " of ",
        " in ",
        "name:string",
        "=",
        "あ",
        "\u{1F600}",
        "\0",
        "\r",
    ];
    let mut rng = Lcg(0xdead_beef_cafe_babe);
    for _ in 0..4000 {
        let parts = (rng.next() % 12) as usize;
        let mut s = String::new();
        for _ in 0..parts {
            let idx = (rng.next() as usize) % fragments.len();
            s.push_str(fragments[idx]);
        }
        assert_no_panic(&s);
    }
}

#[test]
fn large_input_parses_without_blowup() {
    // Guard against accidental super-linear behavior: a big template must parse
    // and produce the expected structure. (A quadratic regression would make
    // the test harness time out rather than pass.)
    let n = 5000;
    let mut html = String::from("html:\n    <div>");
    for i in 0..n {
        html.push_str(&format!("<span class=\"c\">${{v{i}}}</span>"));
    }
    html.push_str("</div>\n");

    let (file, diags) = parse(&html);
    assert!(!diags
        .iter()
        .any(|d| d.severity == lunas_parser::Severity::Error));
    let block = file.html.expect("html");
    // The outer <div> holds n <span> children.
    let div = block
        .template
        .nodes
        .iter()
        .find_map(|node| match node {
            lunas_parser::TemplateNode::Element(e) if e.name == "div" => Some(e),
            _ => None,
        })
        .expect("div");
    let spans = div
        .children
        .iter()
        .filter(|c| matches!(c, lunas_parser::TemplateNode::Element(e) if e.name == "span"))
        .count();
    assert_eq!(spans, n);
}

#[test]
fn deeply_nested_input_parses() {
    // Deep nesting must not overflow the stack at a realistic depth.
    let depth = 300;
    let mut html = String::from("html:\n    ");
    for _ in 0..depth {
        html.push_str("<div>");
    }
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html.push('\n');
    let (file, diags) = parse(&html);
    assert!(!diags
        .iter()
        .any(|d| d.severity == lunas_parser::Severity::Error));
    assert!(file.html.is_some());
}
