//! Extended never-panic fuzzing plus span-soundness property checks for the
//! full `.lunas` parse pipeline (Pest grammar, HTML sub-parser, template pass,
//! directive lowering). Complements `tests/robustness.rs`.
//!
//! Beyond "does not panic", every produced template node, interpolation, and
//! diagnostic range must slice on a valid char boundary of the source — a
//! rebasing bug in the block-offset shift would surface here.

use lunas_parser::{parse, TemplateNode, TextSegment};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Assert every interpolation and diagnostic range is in-bounds and lands on a
/// UTF-8 char boundary of `src` (so `.slice` succeeds).
fn assert_ranges_sound(src: &str) {
    let (file, diags) = parse(src);

    for d in &diags {
        assert!(
            d.range.slice(src).is_some(),
            "diagnostic range {:?} not on a char boundary of {src:?}",
            d.range
        );
    }

    if let Some(html) = &file.html {
        fn walk(nodes: &[TemplateNode], src: &str) {
            for n in nodes {
                match n {
                    TemplateNode::Text(t) => {
                        for seg in &t.segments {
                            if let TextSegment::Interpolation(i) = seg {
                                assert_eq!(
                                    i.expr_range.slice(src),
                                    Some(i.expr.as_str()),
                                    "interpolation expr_range mismatch"
                                );
                                assert!(i.range.slice(src).is_some());
                            }
                        }
                    }
                    TemplateNode::Element(e) => walk(&e.children, src),
                    TemplateNode::Component(c) => walk(&c.children, src),
                    TemplateNode::If(chain) => {
                        for b in &chain.branches {
                            walk(std::slice::from_ref(&b.body), src);
                        }
                    }
                    TemplateNode::For(fb) => walk(std::slice::from_ref(&fb.body), src),
                    TemplateNode::Comment(_) => {}
                }
            }
        }
        walk(&html.template.nodes, src);
    }
}

fn assert_no_panic(input: &str) {
    let ok = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse(input);
    }));
    assert!(ok.is_ok(), "parse panicked on: {input:?}");
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
fn adversarial_template_corpus_is_sound() {
    let cases = [
        "html:\n    <div>${ {a:{b:1}} }</div>",
        "html:\n    <div>${ `t${x}` }${y}</div>",
        "html:\n    <div>${ /}/.test(x) }</div>",
        "html:\n    <div>${ a // }\n }</div>",
        "html:\n    <div>${ /* } */ z }</div>",
        "html:\n    <li :for=\"const [i,v] of xs\">${v}</li>",
        "html:\n    <div :if=\"a\">1</div>\n    <div :elseif=\"b\">2</div>\n    <div :else>3</div>",
        "@use A from \"./A\"\nhtml:\n    <A :for=\"x of xs\" :data=\"x\" />",
        "@input n: number = 0\n@input s: string?\nhtml:\n    <p>${n}${s}</p>",
        "html:\n    <div title=\"${ {k:1}.k }\" :class=\"c\">あ${x}</div>",
        "html:\n    <textarea>${not-parsed}</textarea>",
        "html:\n    <script>if (a<b) { ${nope} }</script>",
        // `<`/`>` operators inside interpolations must not be read as tags.
        "html:\n    <div>${a < b}</div>",
        "html:\n    <div>${a > b}</div>",
        "html:\n    <div>${a <= b && c >= d}</div>",
        "html:\n    <div>${x << 2 >> 1}</div>",
        "html:\n    <div>${f(a<b, c>d)}</div>",
        "html:\n    <div>${cond ? \"<x>\" : \"</y>\"}</div>",
        "html:\n    <p title=\"${a < b}\">${c > d}</p>",
        "html:\n    <div>${ `t${a<b}` }</div>",
        // Unterminated interpolation containing `<` must still recover.
        "html:\n    <div>${a < b",
        // Adversarial / malformed — must recover, not panic.
        "html:\n    <div>${",
        "html:\n    <div>${ \"unterminated",
        "html:\n    <div :for=\"\" :if=\"\" :else>x</div>",
        "@use\n@input\n@useRouting\nhtml:\n    <>",
        "html:\n    <あ ::=\"x\" @=\"y\" :=\"z\">",
    ];
    for c in cases {
        assert_no_panic(c);
        assert_ranges_sound(c);
    }
}

#[test]
fn pseudo_random_pipeline_fuzz_is_sound() {
    let fragments: &[&str] = &[
        "html:",
        "style:",
        "script:",
        "@input",
        "@use",
        "@useRouting",
        "@useAutoRouting",
        "\n",
        "    ",
        "\t",
        "<div>",
        "</div>",
        "<li ",
        "<Comp ",
        "/>",
        ">",
        ":if=",
        ":elseif=",
        ":else",
        ":for=",
        "::v=",
        "@click=",
        "\"x of xs\"",
        "\"cond\"",
        "'y'",
        "${",
        "}",
        "${count}",
        "${ {a:1} }",
        "${ `t${x}` }",
        "${a < b}",
        "${a > b}",
        "${a<b, c>d}",
        "<",
        ">",
        "from",
        "\"./p\"",
        " of ",
        " in ",
        "name:string",
        "name:string?",
        "= 0",
        "=",
        "あ",
        "\u{1F600}",
        "\0",
        "\r",
    ];
    let mut rng = Lcg(0xfeed_0000_beef_1111);
    for _ in 0..5000 {
        let parts = (rng.next() % 14) as usize;
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(fragments[(rng.next() as usize) % fragments.len()]);
        }
        assert_no_panic(&s);
        // Only run the (heavier) soundness walk when an html block plausibly
        // exists, but it's safe either way.
        assert_ranges_sound(&s);
    }
}

#[test]
fn multibyte_offsets_stay_sound() {
    // Multi-byte leading content shifts every downstream span; verify the block
    // rebasing keeps interpolation ranges on char boundaries.
    let bodies = [
        "html:\n    <div>あ${x}い</div>",
        "html:\n    <div title=\"日本${n}語\">🎉${m}</div>",
        "@input 名前: string\nhtml:\n    <p>${名前}</p>",
    ];
    for b in bodies {
        assert_no_panic(b);
        assert_ranges_sound(b);
    }
}

#[test]
fn nested_interpolation_stress_is_sound() {
    // Many interpolations with awkward inner content, all in one run.
    let mut body = String::from("html:\n    <div>");
    for i in 0..200 {
        body.push_str(&format!("${{ {{k:{i}}}.k /* }} */ }}"));
    }
    body.push_str("</div>\n");
    assert_no_panic(&body);
    assert_ranges_sound(&body);
}
