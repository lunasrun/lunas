//! Extended `parse_for` coverage: `of`/`in` variants, destructuring bindings,
//! index patterns, complex iterables, and the special `.entries()` unwrapping.

use lunas_script::{parse_for, ForKind};

fn ok(input: &str) -> (ForKind, String, String) {
    let p = parse_for(input).unwrap_or_else(|| panic!("expected Some for {input:?}"));
    (p.kind, p.binding, p.iterable)
}

// --- of / in kinds ---

#[test]
fn fe_plain_of() {
    assert_eq!(
        ok("item of items"),
        (ForKind::Of, "item".into(), "items".into())
    );
}

#[test]
fn fe_plain_in() {
    assert_eq!(ok("key in obj"), (ForKind::In, "key".into(), "obj".into()));
}

#[test]
fn fe_of_with_let_keyword() {
    assert_eq!(ok("let v of xs"), (ForKind::Of, "v".into(), "xs".into()));
}

#[test]
fn fe_of_with_const_keyword() {
    assert_eq!(ok("const v of xs"), (ForKind::Of, "v".into(), "xs".into()));
}

#[test]
fn fe_of_with_var_keyword() {
    assert_eq!(ok("var v of xs"), (ForKind::Of, "v".into(), "xs".into()));
}

// --- destructuring bindings ---

#[test]
fn fe_array_destructure_binding() {
    assert_eq!(
        ok("[i, v] of pairs"),
        (ForKind::Of, "[i, v]".into(), "pairs".into())
    );
}

#[test]
fn fe_object_destructure_binding() {
    assert_eq!(
        ok("{ id, name } of rows"),
        (ForKind::Of, "{ id, name }".into(), "rows".into())
    );
}

#[test]
fn fe_nested_destructure_binding() {
    assert_eq!(
        ok("[i, { done }] of tasks"),
        (ForKind::Of, "[i, { done }]".into(), "tasks".into())
    );
}

#[test]
fn fe_destructure_with_default() {
    assert_eq!(
        ok("[a = 0, b] of xs"),
        (ForKind::Of, "[a = 0, b]".into(), "xs".into())
    );
}

// --- complex iterables ---

#[test]
fn fe_iterable_call_expression() {
    assert_eq!(
        ok("item of getItems()"),
        (ForKind::Of, "item".into(), "getItems()".into())
    );
}

#[test]
fn fe_iterable_member_access() {
    assert_eq!(
        ok("v of store.items"),
        (ForKind::Of, "v".into(), "store.items".into())
    );
}

#[test]
fn fe_iterable_spread_array() {
    assert_eq!(
        ok("i of [...Array(n).keys()]"),
        (ForKind::Of, "i".into(), "[...Array(n).keys()]".into())
    );
}

#[test]
fn fe_iterable_await_chain() {
    assert_eq!(
        ok("[i, v] of Object.entries(await load().then(r => r.json()))"),
        (
            ForKind::Of,
            "[i, v]".into(),
            "await load().then(r => r.json())".into()
        )
    );
}

// --- .entries() unwrapping behavior ---

#[test]
fn fe_object_entries_non_ident_arg_unwraps() {
    // Object.entries(<non-ident>) unwraps to the argument.
    assert_eq!(
        ok("[k, v] of Object.entries(data.map)"),
        (ForKind::Of, "[k, v]".into(), "data.map".into())
    );
}

#[test]
fn fe_object_entries_ident_arg_kept() {
    // A bare identifier arg is NOT unwrapped — the whole call is kept.
    assert_eq!(
        ok("[k, v] of Object.entries(data)"),
        (ForKind::Of, "[k, v]".into(), "Object.entries(data)".into())
    );
}

#[test]
fn fe_ident_entries_receiver_kept() {
    // `arr.entries()` where the receiver is a bare identifier is NOT unwrapped
    // (only member/paren-member receivers or Object.entries(non-ident) unwrap).
    assert_eq!(
        ok("[i, v] of arr.entries()"),
        (ForKind::Of, "[i, v]".into(), "arr.entries()".into())
    );
}

#[test]
fn fe_chained_member_entries_unwraps() {
    assert_eq!(
        ok("[i, v] of obj.get().items.entries()"),
        (ForKind::Of, "[i, v]".into(), "obj.get().items".into())
    );
}

#[test]
fn fe_paren_member_entries_unwraps() {
    assert_eq!(
        ok("[k, v] of (getObj()).items.entries()"),
        (ForKind::Of, "[k, v]".into(), "(getObj()).items".into())
    );
}

#[test]
fn fe_for_in_entries_not_unwrapped() {
    // The `.entries()` unwrap only applies to for-of, not for-in.
    assert_eq!(
        ok("[i, v] in data.entries()"),
        (ForKind::In, "[i, v]".into(), "data.entries()".into())
    );
}

// --- whitespace tolerance ---

#[test]
fn fe_leading_trailing_whitespace() {
    assert_eq!(
        ok("  item \t of \n items  "),
        (ForKind::Of, "item".into(), "items".into())
    );
}

#[test]
fn fe_whitespace_around_destructure() {
    assert_eq!(
        ok(" [ i , v ] of  arr "),
        (ForKind::Of, "[ i , v ]".into(), "arr".into())
    );
}

// --- literal iterables ---

#[test]
fn fe_array_literal_iterable() {
    assert_eq!(
        ok("x of [1, 2, 3]"),
        (ForKind::Of, "x".into(), "[1, 2, 3]".into())
    );
}

// --- rejection cases ---

#[test]
fn fe_rejects_plain_c_style_for() {
    assert!(parse_for("let i = 0; i < 10; i++").is_none());
}

#[test]
fn fe_rejects_missing_iterable() {
    assert!(parse_for("item of").is_none());
}

#[test]
fn fe_rejects_missing_binding() {
    assert!(parse_for("of items").is_none());
}

#[test]
fn fe_rejects_trailing_tokens() {
    assert!(parse_for("a of b c").is_none());
}

#[test]
fn fe_rejects_empty() {
    assert!(parse_for("").is_none());
}

#[test]
fn fe_rejects_incomplete_member() {
    assert!(parse_for("v of obj.").is_none());
}

#[test]
fn fe_rejects_garbage() {
    assert!(parse_for("not a loop header at all").is_none());
}
