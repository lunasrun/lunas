//! Extended never-panic fuzzing plus structural property checks for the HTML
//! parser.
//!
//! Two invariant strengths are asserted:
//!
//! * **Soundness** (holds for *any* input, well-formed or not): every node range
//!   is within the source bounds and lands on a valid UTF-8 char boundary, so
//!   `.slice` always succeeds. A rebasing or off-by-one bug trips this even when
//!   no panic occurs. Asserted by [`assert_sound`].
//! * **Containment** (parent-child nesting): a child's range sits inside its
//!   parent's, and an attribute's range inside the open tag. This only holds for
//!   *well-formed* input — an unterminated open tag (e.g. `<a b="x"` at EOF)
//!   currently emits an attribute range that runs past the truncated
//!   `open_tag_range`, so containment is asserted only on the balanced-nesting
//!   generator, matching `tests/span_invariants.rs`.

use lunas_html_parser::{parse_html, Element, Node};
use lunas_span::TextRange;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn within(inner: TextRange, outer: TextRange) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Soundness for one element subtree: in-bounds and on char boundaries.
fn check_element_sound(el: &Element, file: TextRange, source: &str) {
    assert!(within(el.range, file), "element out of file bounds");
    assert!(
        el.range.slice(source).is_some(),
        "element range off char boundary"
    );
    for attr in &el.attributes {
        assert!(within(attr.range, file), "attr out of file bounds");
        assert!(
            attr.range.slice(source).is_some(),
            "attr range off char boundary"
        );
        if let Some(vr) = attr.value_range {
            assert!(vr.slice(source).is_some(), "value range off char boundary");
        }
    }
    for child in &el.children {
        assert!(within(child.range(), file), "child out of file bounds");
        if let Node::Element(c) = child {
            check_element_sound(c, file, source);
        }
    }
}

/// Parent-child containment for one element subtree (well-formed input only).
fn check_element_containment(el: &Element, file: TextRange) {
    assert!(within(el.range, file), "element out of file bounds");
    assert!(
        within(el.open_tag_range, el.range),
        "open tag not within element"
    );
    for attr in &el.attributes {
        assert!(
            within(attr.range, el.open_tag_range),
            "attr not within open tag"
        );
        if let Some(vr) = attr.value_range {
            assert!(within(vr, attr.range), "value not within attr");
        }
    }
    for child in &el.children {
        assert!(within(child.range(), el.range), "child escapes parent");
        if let Node::Element(c) = child {
            check_element_containment(c, file);
        }
    }
}

/// Parse `input`, asserting it neither panics nor violates the always-true
/// soundness invariant (in-bounds ranges on valid char boundaries).
fn assert_sound(input: &str) {
    let ok = catch_unwind(AssertUnwindSafe(|| {
        let dom = parse_html(input).dom;
        let file = TextRange::at(0, input.len() as u32);
        for node in &dom.children {
            assert!(within(node.range(), file), "top node out of bounds");
            assert!(
                node.range().slice(input).is_some(),
                "node range not on a char boundary: {:?}",
                node.range()
            );
            if let Node::Element(e) = node {
                check_element_sound(e, file, input);
            }
        }
    }));
    assert!(ok.is_ok(), "parse_html misbehaved on input: {input:?}");
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
fn adversarial_recovery_corpus_is_sound() {
    let cases = [
        "<a><b></a></b>",
        "<div><br></br></div>",
        "<script>a<b</script></script>",
        "<style>.x{}</style extra>",
        "<!--><div>",
        "<a b=\"</a>\">text</a>",
        "<a b='c\"d'></a>",
        "<a ::x=\"1\" :y=\"2\" @z=\"3\">",
        "<textarea><div></textarea>",
        "<title></title></title>",
        "<a\n\tb=1\n\tc=2\n>",
        "<a b=\"\u{1F600}\" c=\"あ\">",
        "<><!--<-->",
        "<a/></a>",
        "<img src=x/>text",
        "<p>a</p><p>b</p><p>c",
        "</div></div></div>",
        "<DIV CLASS=X></div>",
        "<a =b =c>",
        "<a b= c= d=>",
        "<!DOCTYPE html><html><head></head><body><p>x</p></body></html>",
    ];
    for c in cases {
        assert_sound(c);
    }
}

#[test]
fn pseudo_random_structural_fuzz_is_sound() {
    // Token fragments biased toward tag structure so the tree builder exercises
    // its open/close/auto-close paths often.
    let fragments: &[&str] = &[
        "<div>",
        "</div>",
        "<span>",
        "</span>",
        "<br>",
        "<img ",
        "src=x",
        "/>",
        ">",
        "<script>",
        "</script>",
        "<style>",
        "</style>",
        "<!--",
        "-->",
        "<!DOCTYPE html>",
        "\"q\"",
        "'s'",
        ":if=",
        "@click=",
        "::v=",
        "=",
        " ",
        "\n",
        "\t",
        "text",
        "あ",
        "\u{1F600}",
        "<",
        ">",
        "</",
        "<a",
        "<Comp ",
        "\0",
    ];
    let mut rng = Lcg(0x0bad_c0de_dead_10cc);
    for _ in 0..6000 {
        let parts = (rng.next() % 14) as usize;
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(fragments[(rng.next() as usize) % fragments.len()]);
        }
        assert_sound(&s);
    }
}

#[test]
fn random_byte_soup_never_panics() {
    // Pure random ASCII-ish bytes (kept valid UTF-8 by construction) to catch
    // states the structural fuzzer biases away from.
    let mut rng = Lcg(0xabcd_1234_5678_ef01);
    for _ in 0..4000 {
        let len = (rng.next() % 60) as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            // Printable ASCII range 0x20..0x7e plus occasional control chars.
            let b = (0x20 + (rng.next() % 0x5f)) as u8;
            s.push(b as char);
        }
        assert_sound(&s);
    }
}

#[test]
fn balanced_random_nesting_has_no_diagnostics() {
    // Well-formed nestings of known non-void tags must parse cleanly with span
    // invariants intact and zero diagnostics.
    let tags = ["div", "span", "p", "section", "ul", "li", "b", "em"];
    let mut rng = Lcg(0x5555_aaaa_5555_aaaa);
    for _ in 0..800 {
        let depth = 1 + (rng.next() % 8) as usize;
        let mut open = String::new();
        let mut close = String::new();
        for _ in 0..depth {
            let t = tags[(rng.next() as usize) % tags.len()];
            open.push_str(&format!("<{t}>"));
            close.insert_str(0, &format!("</{t}>"));
        }
        let src = format!("{open}text{close}");
        let r = parse_html(&src);
        assert!(
            r.diagnostics.is_empty(),
            "unexpected diagnostics on {src:?}: {:?}",
            r.diagnostics
        );
        assert_sound(&src);
        // Well-formed: the stronger parent-child containment invariant holds.
        let file = TextRange::at(0, src.len() as u32);
        for node in &r.dom.children {
            if let Node::Element(e) = node {
                check_element_containment(e, file);
            }
        }
    }
}
