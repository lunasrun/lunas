//! Tests for the static-skeleton pass: static HTML emission, dynamic slot
//! positions in skeleton coordinates, and dynamic-element ref paths.

use lunas_compiler::codegen::{build_skeleton, InsertPos, Skeleton, SlotContent};
use lunas_parser::parse;

fn skel(body: &str) -> Skeleton {
    let src = format!("html:\n    {body}\n");
    let (file, diags) = parse(&src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected errors: {diags:?}"
    );
    build_skeleton(&file.html.expect("html").template)
}

#[test]
fn static_only_passthrough() {
    let s = skel("<div class=\"box\"><p>hi</p></div>");
    assert_eq!(s.html, "<div class=\"box\"><p>hi</p></div>");
    assert!(s.slots.is_empty());
    assert!(s.dynamic_elements.is_empty());
}

#[test]
fn interelement_whitespace_dropped() {
    let s = skel("<div>\n        <p>a</p>\n        <p>b</p>\n    </div>");
    assert_eq!(s.html, "<div><p>a</p><p>b</p></div>");
}

#[test]
fn comments_dropped() {
    let s = skel("<div><!-- note --><p>x</p></div>");
    assert_eq!(s.html, "<div><p>x</p></div>");
    assert!(s.slots.is_empty());
}

#[test]
fn dynamic_text_appended_when_last() {
    let s = skel("<p>${count}</p>");
    assert_eq!(s.html, "<p></p>");
    assert_eq!(s.slots.len(), 1);
    assert!(matches!(s.slots[0].content, SlotContent::Text(_)));
    assert_eq!(s.slots[0].pos, InsertPos::Append(vec![0]));
}

#[test]
fn whole_run_with_interpolation_is_one_slot() {
    // Literals in the same run are part of the dynamic text node, not the
    // skeleton.
    let s = skel("<p>count: ${count}!</p>");
    assert_eq!(s.html, "<p></p>");
    assert_eq!(s.slots.len(), 1);
    match &s.slots[0].content {
        SlotContent::Text(t) => assert_eq!(t.segments.len(), 3),
        other => panic!("expected text slot, got {other:?}"),
    }
}

#[test]
fn dynamic_text_before_static_element() {
    let s = skel("<p>${x}<span>s</span></p>");
    assert_eq!(s.html, "<p><span>s</span></p>");
    assert_eq!(s.slots[0].pos, InsertPos::Before(vec![0, 0]));
}

#[test]
fn bound_attr_makes_element_dynamic_and_attr_omitted() {
    let s = skel("<input :value=\"v\">");
    assert_eq!(s.html, "<input>");
    assert_eq!(s.dynamic_elements.len(), 1);
    assert_eq!(s.dynamic_elements[0].path, vec![0]);
    assert_eq!(s.dynamic_elements[0].name, "input");
}

#[test]
fn event_attr_kept_out_of_html() {
    let s = skel("<button @click=\"f()\">x</button>");
    assert_eq!(s.html, "<button>x</button>");
    assert_eq!(s.dynamic_elements.len(), 1);
}

#[test]
fn interpolated_static_attr_is_dynamic() {
    let s = skel("<div class=\"a ${x}\"></div>");
    assert_eq!(s.html, "<div></div>");
    assert_eq!(s.dynamic_elements.len(), 1);
}

#[test]
fn fully_static_attrs_stay() {
    let s = skel("<a href=\"/x\" download>go</a>");
    assert_eq!(s.html, "<a href=\"/x\" download>go</a>");
    assert!(s.dynamic_elements.is_empty());
}

#[test]
fn attr_value_quote_escaped() {
    // A double quote inside a single-quoted source attribute must be escaped
    // when re-emitted double-quoted.
    let s = skel("<div title='say \"hi\"'></div>");
    assert_eq!(s.html, "<div title=\"say &quot;hi&quot;\"></div>");
}

#[test]
fn if_slot_before_following_element() {
    let s = skel("<div><span :if=\"c\">y</span><b>z</b></div>");
    assert_eq!(s.html, "<div><b>z</b></div>");
    assert_eq!(s.slots.len(), 1);
    assert!(matches!(s.slots[0].content, SlotContent::If(_)));
    assert_eq!(s.slots[0].pos, InsertPos::Before(vec![0, 0]));
}

#[test]
fn adjacent_if_slots_share_position_in_order() {
    let s = skel("<div><i :if=\"a\">x</i><i :if=\"b\">y</i><u>t</u></div>");
    assert_eq!(s.html, "<div><u>t</u></div>");
    assert_eq!(s.slots.len(), 2);
    assert_eq!(s.slots[0].pos, InsertPos::Before(vec![0, 0]));
    assert_eq!(s.slots[1].pos, InsertPos::Before(vec![0, 0]));
}

#[test]
fn for_slot_appended_inside_parent() {
    let s = skel("<ul><li :for=\"n of items\">${n}</li></ul>");
    assert_eq!(s.html, "<ul></ul>");
    assert_eq!(s.slots.len(), 1);
    assert!(matches!(s.slots[0].content, SlotContent::For(_)));
    assert_eq!(s.slots[0].pos, InsertPos::Append(vec![0]));
}

#[test]
fn text_if_text_needs_split() {
    // "before " and " after" merge into one text node in the parsed skeleton;
    // the :if block must be inserted at a split point between them.
    let s = skel("<div>before <span :if=\"c\">y</span> after</div>");
    assert_eq!(s.html, "<div>before  after</div>");
    assert_eq!(s.slots.len(), 1);
    assert_eq!(
        s.slots[0].pos,
        InsertPos::BeforeSplit {
            path: vec![0, 0],
            utf16_offset: 7 // "before ".len() in UTF-16
        }
    );
}

#[test]
fn split_offset_counts_utf16() {
    // "あ𝄞 " is 1 + 2 + 1 = 4 UTF-16 units.
    let s = skel("<div>あ𝄞 <span :if=\"c\">y</span>z</div>");
    assert_eq!(
        s.slots[0].pos,
        InsertPos::BeforeSplit {
            path: vec![0, 0],
            utf16_offset: 4
        }
    );
}

#[test]
fn void_element_not_closed() {
    let s = skel("<div><img src=\"a.png\"><br></div>");
    assert_eq!(s.html, "<div><img src=\"a.png\"><br></div>");
}

#[test]
fn component_slot() {
    let src = "@use Card from \"./Card.lunas\"\nhtml:\n    <div><Card></Card><p>x</p></div>\n";
    let (file, diags) = parse(src);
    assert!(!diags.iter().any(|d| d.is_error()), "{diags:?}");
    let s = build_skeleton(&file.html.expect("html").template);
    assert_eq!(s.html, "<div><p>x</p></div>");
    assert_eq!(s.slots.len(), 1);
    assert!(matches!(s.slots[0].content, SlotContent::Component(_)));
    assert_eq!(s.slots[0].pos, InsertPos::Before(vec![0, 0]));
}

#[test]
fn nested_paths_are_correct() {
    let s = skel("<div><section><p>${x}</p></section><footer @click=\"f\">t</footer></div>");
    // <p> is [0,0,0]; slot appends into it.
    assert_eq!(s.slots[0].pos, InsertPos::Append(vec![0, 0, 0]));
    // footer is the second child of div.
    assert_eq!(s.dynamic_elements[0].path, vec![0, 1]);
    assert_eq!(
        s.html,
        "<div><section><p></p></section><footer>t</footer></div>"
    );
}

#[test]
fn slots_are_in_template_preorder() {
    let s = skel(
        "<div><p>${a}</p><span :if=\"c\">x</span><ul><li :for=\"n of ns\">${n}</li></ul></div>",
    );
    let kinds: Vec<&str> = s
        .slots
        .iter()
        .map(|sl| match sl.content {
            SlotContent::Text(_) => "text",
            SlotContent::If(_) => "if",
            SlotContent::For(_) => "for",
            SlotContent::Component(_) => "component",
        })
        .collect();
    assert_eq!(kinds, vec!["text", "if", "for"]);
}
