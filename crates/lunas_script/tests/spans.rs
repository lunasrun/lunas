//! Byte-range correctness for the span-returning analyses — the input to
//! LSP go-to-definition, find-references, rename, and in-place reactive
//! rewrites. Every reported range must slice back to the exact identifier /
//! initializer / statement text, and distinct occurrences must have distinct
//! ranges.

use lunas_script::{
    declared_bindings_with_spans, free_identifiers_with_spans, module_binding_references,
    referenced_identifiers_with_spans, top_level_declarations, BindingRef, DeclKind,
};
use lunas_span::TextRange;

// Helper: assert every (name, range) slices back to `name` in `code`.
fn assert_slices_back(code: &str, ids: &[(String, TextRange)]) {
    for (name, range) in ids {
        assert_eq!(
            range.slice(code),
            Some(name.as_str()),
            "bad span for {name}"
        );
    }
}

// --- referenced_identifiers_with_spans ---

#[test]
fn ref_spans_multiline_offsets() {
    let code = "a +\n  bbb *\n    cc";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["a", "bbb", "cc"]);
    assert_slices_back(code, &ids);
}

#[test]
fn ref_spans_repeated_name_distinct_ranges() {
    let code = "x + x + x";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    assert_eq!(ids.len(), 3);
    assert_ne!(ids[0].1, ids[1].1);
    assert_ne!(ids[1].1, ids[2].1);
    assert_eq!(ids[0].1.start().raw(), 0);
    assert_eq!(ids[2].1.start().raw(), 8);
}

#[test]
fn ref_spans_unicode_multibyte() {
    let code = "café + naïve + 日本語";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    assert_slices_back(code, &ids);
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["café", "naïve", "日本語"]);
}

#[test]
fn ref_spans_member_root_only() {
    let code = "obj.deep.prop";
    let ids = referenced_identifiers_with_spans(code).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].1.slice(code), Some("obj"));
}

// --- declared_bindings_with_spans ---

#[test]
fn decl_spans_point_at_names_not_values() {
    let code = "let count = 100\nconst label = \"hi\"";
    let decls = declared_bindings_with_spans(code).unwrap();
    let names: Vec<_> = decls.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["count", "label"]);
    assert_slices_back(code, &decls);
    // The span is the name, not the initializer.
    assert_eq!(decls[0].1.start().raw(), 4);
}

#[test]
fn decl_spans_destructuring_each_name() {
    let code = "const { a, b: c } = obj\nconst [d, e] = xs";
    let decls = declared_bindings_with_spans(code).unwrap();
    let names: Vec<_> = decls.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["a", "c", "d", "e"]);
    assert_slices_back(code, &decls);
}

#[test]
fn decl_spans_imports() {
    let code = "import def, { named as alias } from 'm'";
    let decls = declared_bindings_with_spans(code).unwrap();
    let names: Vec<_> = decls.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["def", "alias"]);
    assert_slices_back(code, &decls);
}

#[test]
fn decl_spans_function_and_class_names() {
    let code = "function compute(){}\nclass Widget {}";
    let decls = declared_bindings_with_spans(code).unwrap();
    assert_eq!(decls[0].1.slice(code), Some("compute"));
    assert_eq!(decls[1].1.slice(code), Some("Widget"));
}

#[test]
fn decl_spans_object_assign_default_uses_key() {
    // `{ x = 5 }` — the span is the key `x`, not the default.
    let code = "const { x = 5 } = obj";
    let decls = declared_bindings_with_spans(code).unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].0, "x");
    assert_eq!(decls[0].1.slice(code), Some("x"));
}

// --- free_identifiers_with_spans ---

#[test]
fn free_spans_report_outer_not_shadow() {
    let code = "value + list.map(value => value)";
    let ids = free_identifiers_with_spans(code).unwrap();
    let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["value", "list"]);
    // The kept `value` is the leading (outer) occurrence.
    assert_eq!(ids[0].1.start().raw(), 0);
    assert_slices_back(code, &ids);
}

// --- module_binding_references ---

fn mrefs(code: &str) -> Vec<BindingRef> {
    module_binding_references(code).expect("parse ok")
}

#[test]
fn mref_spans_slice_back_to_identifier() {
    let code = "let count = 0\nfunction inc(){ count = count + 1 }";
    let refs = mrefs(code);
    assert_eq!(refs.len(), 2);
    for r in &refs {
        assert_eq!(r.range.slice(code), Some("count"));
        assert_eq!(r.name, "count");
        assert!(!r.shorthand);
    }
    // Two distinct occurrences.
    assert_ne!(refs[0].range, refs[1].range);
}

#[test]
fn mref_declaration_and_shadow_excluded() {
    let code = "let n = 1\nconst f = (n) => n + 1\nfunction g(){ return n }";
    let refs = mrefs(code);
    // Only `return n` in g refers to the top-level binding.
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "n");
    assert_eq!(refs[0].range.slice(code), Some("n"));
}

#[test]
fn mref_block_shadow_only_outer_counts() {
    let code = "let n = 1\nfunction g(){ { let n = 2; n } return n }";
    let refs = mrefs(code);
    assert_eq!(refs.len(), 1);
    // The surviving ref is the later `return n`, past the shadowing block.
    assert!(refs[0].range.start().raw() > code.find("return").unwrap() as u32 - 1);
}

#[test]
fn mref_shorthand_flag_and_expansion_range() {
    let code = "let count = 0\nfunction f(){ return { count } }";
    let refs = mrefs(code);
    assert_eq!(refs.len(), 1);
    assert!(refs[0].shorthand);
    assert_eq!(refs[0].range.slice(code), Some("count"));
}

#[test]
fn mref_catch_param_shadows_top_level() {
    let code = "let e = 1\nfunction f(){ try { g() } catch (e) { return e } return e }";
    let refs = mrefs(code);
    // Only the outer `return e` (outside catch) is a top-level ref.
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "e");
}

#[test]
fn mref_param_default_expression_is_reference() {
    let code = "let base = 1\nconst f = (x = base) => x";
    let refs = mrefs(code);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "base");
    assert_eq!(refs[0].range.slice(code), Some("base"));
}

#[test]
fn mref_computed_member_key_is_reference_static_prop_not() {
    let code = "let k = 'a'\nlet obj = {}\nfunction f(){ return obj[k].k }";
    let refs = mrefs(code);
    // obj (once) and computed k (once); the static `.k` prop is not a ref.
    let ks = refs.iter().filter(|r| r.name == "k").count();
    let objs = refs.iter().filter(|r| r.name == "obj").count();
    assert_eq!(ks, 1);
    assert_eq!(objs, 1);
}

#[test]
fn mref_named_fn_expr_self_name_not_top_level() {
    // The named fn-expr `count` self-name shadows the top-level binding inside.
    let code = "let count = 0\nconst f = function count(){ return count }";
    let refs = mrefs(code);
    // The inner `return count` is the fn-expr's own name, not the top-level one.
    assert!(refs.is_empty(), "got {refs:?}");
}

// --- top_level_declarations ---

#[test]
fn tld_init_and_stmt_ranges() {
    let code = "let count = 1 + 2";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].name, "count");
    assert_eq!(ds[0].kind, DeclKind::Let);
    assert_eq!(ds[0].name_range.slice(code), Some("count"));
    assert_eq!(ds[0].init_range.unwrap().slice(code), Some("1 + 2"));
    assert_eq!(ds[0].stmt_range.slice(code), Some("let count = 1 + 2"));
    assert_eq!(ds[0].declarators_in_stmt, 1);
}

#[test]
fn tld_no_init_var() {
    let code = "var loose";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].kind, DeclKind::Var);
    assert!(ds[0].init_range.is_none());
    assert_eq!(ds[0].name_range.slice(code), Some("loose"));
}

#[test]
fn tld_multi_declarator_shares_stmt_range() {
    let code = "const a = 1, b = 2, c = 3";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 3);
    for d in &ds {
        assert_eq!(d.declarators_in_stmt, 3);
        assert_eq!(d.stmt_range.slice(code), Some(code));
    }
    assert_eq!(ds[0].init_range.unwrap().slice(code), Some("1"));
    assert_eq!(ds[2].name_range.slice(code), Some("c"));
}

#[test]
fn tld_destructured_declarators_skipped() {
    let code = "const { a } = o\nconst [b] = xs\nlet plain = 1";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].name, "plain");
}

#[test]
fn tld_exported_included_stmt_excludes_export_keyword() {
    let code = "export let shown = true";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    // stmt_range covers the VarDecl, not the `export` keyword.
    assert_eq!(ds[0].stmt_range.slice(code), Some("let shown = true"));
}

#[test]
fn tld_typescript_annotation_init_range() {
    let code = "let n: number = 42";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    // The init range is the value, not the annotation.
    assert_eq!(ds[0].init_range.unwrap().slice(code), Some("42"));
}

#[test]
fn tld_multiline_offsets() {
    let code = "let a = 1\nlet b = 2\nlet c = 3";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 3);
    assert_eq!(ds[1].name_range.slice(code), Some("b"));
    assert_eq!(ds[1].init_range.unwrap().slice(code), Some("2"));
    assert_eq!(ds[2].stmt_range.slice(code), Some("let c = 3"));
}

#[test]
fn tld_function_and_class_not_variable_decls() {
    let code = "function f(){}\nclass C {}\nlet only = 1";
    let ds = top_level_declarations(code).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].name, "only");
}
