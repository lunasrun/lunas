//! Structural span invariants: in a well-formed parse, every child node's range
//! is contained within its parent element's range, and every range lies within
//! the source bounds. Catches rebasing / span-computation bugs.

use lunas_html_parser::{parse_html, Element, Node};
use lunas_span::{TextRange, TextSize};

fn within(inner: TextRange, outer: TextRange) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

fn check_element(el: &Element, file: TextRange) {
    assert!(
        within(el.range, file),
        "element {:?} out of file bounds",
        el.name
    );
    assert!(
        within(el.open_tag_range, el.range),
        "{}: open tag not within element range",
        el.name
    );
    for attr in &el.attributes {
        assert!(
            within(attr.range, el.open_tag_range),
            "{}: attribute {:?} not within open tag",
            el.name,
            attr.name
        );
    }
    for child in &el.children {
        assert!(
            within(child.range(), el.range),
            "{}: child not contained in parent range",
            el.name
        );
        if let Node::Element(c) = child {
            check_element(c, file);
        }
    }
}

fn check(source: &str) {
    let dom = parse_html(source).dom;
    let file = TextRange::new(TextSize::new(0), TextSize::new(source.len() as u32));
    for node in &dom.children {
        assert!(within(node.range(), file), "top node out of bounds");
        if let Node::Element(e) = node {
            check_element(e, file);
        }
    }
}

#[test]
fn nesting_span_containment() {
    let cases = [
        "<a><b><c>text</c></b></a>",
        "<div class=\"x\" id=\"y\"><p>hi <b>bold</b>!</p></div>",
        "<ul><li>1</li><li>2</li></ul>",
        "<section><!-- c --><span>s</span></section>",
        "<a><img src=\"x\"><br></a>",
        "<script>if (a<b){}</script>",
        "<p>あ<b>い</b>う</p>",
    ];
    for c in cases {
        check(c);
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
fn nesting_span_containment_generated() {
    // Build well-formed random nestings of a small tag set and verify the
    // containment invariant holds for every node.
    let tags = ["div", "span", "p", "b", "section", "ul", "li"];
    let mut rng = Lcg(0xfeed_face_0000_0001);
    for _ in 0..500 {
        let depth = 1 + (rng.next() % 6) as usize;
        let mut open = String::new();
        let mut close = String::new();
        for _ in 0..depth {
            let t = tags[(rng.next() as usize) % tags.len()];
            open.push_str(&format!("<{t}>"));
            close.insert_str(0, &format!("</{t}>"));
        }
        let src = format!("{open}text{close}");
        check(&src);
    }
}
