//! Tests for dynamic-part dependency resolution and handler write sets.

use lunas_compiler::{resolve, DynamicKind, DynamicPart, ResolvedComponent};

fn ok(src: &str) -> ResolvedComponent {
    let (c, diags) = resolve(src);
    assert!(
        !diags.iter().any(|d| d.is_error()),
        "unexpected errors: {diags:?}"
    );
    c
}

/// The dependency indices of the first dynamic part matching `kind`.
fn deps_of(c: &ResolvedComponent, pred: impl Fn(&DynamicPart) -> bool) -> Vec<u32> {
    let part = c
        .dynamics
        .iter()
        .find(|d| pred(d))
        .expect("a matching part");
    part.deps.indices().to_vec()
}

fn idx(c: &ResolvedComponent, name: &str) -> u32 {
    c.reactive_index(name).expect("reactive var")
}

#[test]
fn interpolation_depends_on_reactive_var() {
    let c = ok("\
html:
    <p>${count}</p>
script:
    let count = 0
    function inc(){ count++ }
");
    let deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    assert_eq!(deps, vec![idx(&c, "count")]);
}

#[test]
fn non_reactive_interpolation_has_no_deps_but_is_recorded() {
    let c = ok("\
html:
    <p>${label}</p>
script:
    const label = \"hi\"
");
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::Text)
        .expect("text part");
    assert_eq!(part.expr, "label");
    assert!(part.deps.is_empty());
}

#[test]
fn handler_write_set_through_function() {
    let c = ok("\
html:
    <button @click=\"inc()\">${count}</button>
script:
    let count = 0
    function inc(){ count++ }
");
    assert_eq!(c.handlers.len(), 1);
    let h = &c.handlers[0];
    assert_eq!(h.event, "click");
    assert_eq!(h.writes.indices(), &[idx(&c, "count")]);
}

#[test]
fn interpolation_through_function_reads_transitively() {
    let c = ok("\
html:
    <p>${total()}</p>
script:
    let price = 1
    let qty = 2
    function total(){ return price * qty }
    function setp(){ price = 3 }
    function setq(){ qty = 4 }
");
    // total() reads price and qty (both reactive: mutated by setp/setq).
    let mut deps = deps_of(&c, |d| d.kind == DynamicKind::Text);
    deps.sort_unstable();
    let mut expected = vec![idx(&c, "price"), idx(&c, "qty")];
    expected.sort_unstable();
    assert_eq!(deps, expected);
}

#[test]
fn bound_attribute_and_two_way() {
    let c = ok("\
html:
    <input :value=\"name\" ::checked=\"flag\" />
script:
    let name = \"\"
    let flag = false
    function f(){ name = \"x\"; flag = true }
");
    let attr = deps_of(&c, |d| d.kind == DynamicKind::Attribute("value".into()));
    assert_eq!(attr, vec![idx(&c, "name")]);
    let tw = deps_of(&c, |d| d.kind == DynamicKind::TwoWay("checked".into()));
    assert_eq!(tw, vec![idx(&c, "flag")]);
}

#[test]
fn if_condition_and_for_iterable() {
    let c = ok("\
html:
    <ul>
        <li :if=\"open\">x</li>
        <li :for=\"item of items\">${item}</li>
    </ul>
script:
    let open = false
    let items = []
    function f(){ open = true; items = [1] }
");
    let cond = deps_of(&c, |d| d.kind == DynamicKind::IfCondition);
    assert_eq!(cond, vec![idx(&c, "open")]);
    let iter = deps_of(&c, |d| d.kind == DynamicKind::ForIterable);
    assert_eq!(iter, vec![idx(&c, "items")]);
    // `item` is the loop binding, not reactive component state.
    assert!(c.reactive_index("item").is_none());
}

#[test]
fn for_iterable_range_points_at_the_iterable() {
    let src = "\
html:
    <li :for=\"item of items\">${item}</li>
script:
    let items = []
    function f(){ items = [1] }
";
    let c = ok(src);
    let part = c
        .dynamics
        .iter()
        .find(|d| d.kind == DynamicKind::ForIterable)
        .expect("for iterable");
    assert_eq!(part.range.slice(src), Some("items"));
}

#[test]
fn deps_mask_is_a_bitset() {
    let c = ok("\
html:
    <p>${a}${b}</p>
script:
    let a = 0
    let b = 0
    function f(){ a = 1; b = 2 }
");
    // Build a part that depends on both by referencing both: use the handler.
    let _ = &c;
    // a -> bit 0, b -> bit 1.
    assert_eq!(idx(&c, "a"), 0);
    assert_eq!(idx(&c, "b"), 1);
    // Each interpolation depends on exactly one.
    let texts: Vec<_> = c
        .dynamics
        .iter()
        .filter(|d| d.kind == DynamicKind::Text)
        .collect();
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].deps.mask_u128(), Some(0b01));
    assert_eq!(texts[1].deps.mask_u128(), Some(0b10));
}
