//! Coverage of the mutation-tracking analyses: `assigned_identifiers` (all
//! assignment / update / in-place-mutator targets in a snippet) and
//! `function_mutations` (per top-level function, the roots it mutates). These
//! drive which component state a handler changes for reactive updates.

use lunas_script::{assigned_identifiers, function_mutations};

fn assigns(code: &str) -> Vec<String> {
    assigned_identifiers(code).expect("parse ok")
}

fn none() -> Vec<String> {
    Vec::<String>::new()
}

// --- plain and compound assignment ---

#[test]
fn assign_all_compound_operators() {
    assert_eq!(
        assigns("a += 1; b -= 1; c *= 2; d /= 2; e %= 2; f **= 2"),
        ["a", "b", "c", "d", "e", "f"]
    );
}

#[test]
fn assign_bitwise_and_shift_compound() {
    assert_eq!(
        assigns("a &= 1; b |= 1; c ^= 1; d <<= 1; e >>= 1; f >>>= 1"),
        ["a", "b", "c", "d", "e", "f"]
    );
}

#[test]
fn assign_logical_compound() {
    assert_eq!(assigns("a ||= 1; b &&= 1; c ??= 1"), ["a", "b", "c"]);
}

// --- update expressions ---

#[test]
fn assign_prefix_and_postfix_update() {
    assert_eq!(assigns("++a; a++; --b; b--"), ["a", "a", "b", "b"]);
}

#[test]
fn assign_update_on_member_reports_root() {
    assert_eq!(assigns("obj.count++; ++state.n"), ["obj", "state"]);
}

// --- member / index targets report the root binding ---

#[test]
fn assign_deep_member_root() {
    assert_eq!(assigns("a.b.c.d = 1"), ["a"]);
}

#[test]
fn assign_computed_index_root() {
    assert_eq!(assigns("arr[i] = 1; grid[x][y] = 2"), ["arr", "grid"]);
}

#[test]
fn assign_mixed_member_and_index_root() {
    assert_eq!(assigns("state.items[k].done = true"), ["state"]);
}

#[test]
fn assign_parenthesized_target_root() {
    assert_eq!(assigns("(obj).x = 1"), ["obj"]);
}

// --- in-place mutating methods report the receiver root ---

#[test]
fn assign_array_mutators() {
    assert_eq!(
        assigns("xs.push(1); ys.pop(); zs.shift(); ws.unshift(0)"),
        ["xs", "ys", "zs", "ws"]
    );
}

#[test]
fn assign_more_array_mutators() {
    assert_eq!(
        assigns("a.splice(0,1); b.sort(); c.reverse(); d.fill(0); e.copyWithin(0,1)"),
        ["a", "b", "c", "d", "e"]
    );
}

#[test]
fn assign_collection_mutators() {
    assert_eq!(
        assigns("s.add(1); s2.delete(2); m.set('k', 1); m2.clear()"),
        ["s", "s2", "m", "m2"]
    );
}

#[test]
fn assign_mutator_on_nested_member_reports_root() {
    assert_eq!(assigns("state.list.push(1)"), ["state"]);
}

#[test]
fn assign_non_mutating_methods_ignored() {
    // filter/map/slice/concat return new values; the receiver is not mutated.
    assert_eq!(
        assigns("const a = xs.filter(f).map(g).slice(0).concat(ys)"),
        none()
    );
}

#[test]
fn assign_mutator_name_as_plain_call_not_counted() {
    // A bare `push(x)` (not a method call) is not a receiver mutation.
    assert_eq!(assigns("push(x)"), none());
}

// --- assignment chains and RHS recursion ---

#[test]
fn assign_chain_reports_all_targets() {
    assert_eq!(assigns("a = b = c = 1"), ["a", "b", "c"]);
}

#[test]
fn assign_rhs_nested_assignment_and_mutator() {
    // The RHS is recursed: inner assignment and a mutator both count.
    assert_eq!(assigns("outer = (inner = 1)"), ["outer", "inner"]);
    assert_eq!(assigns("x = xs.push(1)"), ["x", "xs"]);
}

// --- destructuring assignment targets ---

#[test]
fn assign_array_destructure_targets() {
    assert_eq!(assigns("[a, , b] = pair"), ["a", "b"]);
}

#[test]
fn assign_object_destructure_targets() {
    assert_eq!(assigns("({ x, y: z, ...rest } = obj)"), ["x", "z", "rest"]);
}

#[test]
fn assign_nested_destructure_targets() {
    assert_eq!(assigns("({ a: { b }, c: [d] } = src)"), ["b", "d"]);
}

// --- comparisons are not assignments ---

#[test]
fn assign_equality_not_counted() {
    assert_eq!(assigns("a == b; c === d; e != f; g !== h"), none());
}

// --- mutations occurring inside nested functions still count (flat visitor) ---

#[test]
fn assign_inside_nested_function() {
    assert_eq!(
        assigns("function outer(){ function inner(){ state = 1 } inner() }"),
        ["state"]
    );
}

#[test]
fn assign_inside_arrow_callback() {
    assert_eq!(
        assigns("items.forEach(() => { touched = true })"),
        ["touched"]
    );
}

#[test]
fn assign_via_aliased_member() {
    // Mutating through an aliased reference still roots at the alias binding.
    assert_eq!(assigns("const ref = obj; ref.x = 1"), ["ref"]);
}

// --- function_mutations: per-function grouping ---

#[test]
fn fmut_multiple_functions_grouped() {
    let muts = function_mutations(
        "function a(){ x = 1 }\nfunction b(){ y++ }\nconst c = () => { z.push(1) }",
    )
    .unwrap();
    assert_eq!(
        muts,
        vec![
            ("a".to_string(), vec!["x".to_string()]),
            ("b".to_string(), vec!["y".to_string()]),
            ("c".to_string(), vec!["z".to_string()]),
        ]
    );
}

#[test]
fn fmut_function_expression_const() {
    let muts = function_mutations("const f = function(){ n = 1 }").unwrap();
    assert_eq!(muts, vec![("f".to_string(), vec!["n".to_string()])]);
}

#[test]
fn fmut_dedups_repeated_targets() {
    let muts = function_mutations("function f(){ a = 1; a += 2; a++; a.push(1) }").unwrap();
    assert_eq!(muts, vec![("f".to_string(), vec!["a".to_string()])]);
}

#[test]
fn fmut_empty_for_pure_function() {
    let muts = function_mutations("function pure(x){ return x * 2 }").unwrap();
    assert_eq!(muts, vec![("pure".to_string(), vec![])]);
}

#[test]
fn fmut_mutation_in_nested_closure_attributed_to_outer() {
    // The nested arrow's mutation is attributed to the top-level function.
    let muts = function_mutations("function reg(){ on('x', () => { fired = true }) }").unwrap();
    assert_eq!(muts, vec![("reg".to_string(), vec!["fired".to_string()])]);
}

#[test]
fn fmut_exported_function_included() {
    let muts = function_mutations("export function inc(){ count++ }").unwrap();
    assert_eq!(muts, vec![("inc".to_string(), vec!["count".to_string()])]);
}

#[test]
fn fmut_non_callable_const_skipped() {
    // A plain `const` (not arrow/function) is not a function; only `f` appears.
    let muts = function_mutations("const total = 0\nconst f = () => { total2 = 1 }").unwrap();
    assert_eq!(muts, vec![("f".to_string(), vec!["total2".to_string()])]);
}

#[test]
fn fmut_member_mutator_receiver_root() {
    let muts = function_mutations("function f(){ store.items.splice(0, 1) }").unwrap();
    assert_eq!(muts, vec![("f".to_string(), vec!["store".to_string()])]);
}

#[test]
fn fmut_multi_declarator_const_functions() {
    // `const a = ..., b = ...` with two arrows yields two entries.
    let muts = function_mutations("const a = () => { p = 1 }, b = () => { q = 2 }").unwrap();
    assert_eq!(
        muts,
        vec![
            ("a".to_string(), vec!["p".to_string()]),
            ("b".to_string(), vec!["q".to_string()]),
        ]
    );
}

#[test]
fn fmut_order_preserved_within_function() {
    let muts = function_mutations("function f(){ b = 1; a = 2; c.push(1) }").unwrap();
    assert_eq!(
        muts,
        vec![(
            "f".to_string(),
            vec!["b".to_string(), "a".to_string(), "c".to_string()]
        )]
    );
}
