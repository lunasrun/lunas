//! Never-panic fuzzing across *every* public `lunas_script` entry point,
//! including the scope-aware and span-returning analyses and the two functions
//! (`function_dependencies`, `analyze_script`, `module_binding_references`,
//! `top_level_declarations`) not exercised by the crate's `robustness.rs`.
//! Public entry points return `Result`/`Option` and must never panic.

use lunas_script::{
    analyze_script, assigned_identifiers, declared_bindings, declared_bindings_with_spans,
    free_identifiers, free_identifiers_with_spans, function_dependencies, function_mutations,
    module_binding_references, parse_for, parse_to_ast_json, referenced_identifiers,
    referenced_identifiers_with_spans, top_level_declarations, transform_ts_to_js,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn no_panic(label: &str, input: &str, f: impl Fn(&str)) {
    let result = catch_unwind(AssertUnwindSafe(|| f(input)));
    assert!(result.is_ok(), "{label} panicked on input: {input:?}");
}

/// Exercise every public entry with `input`.
fn exercise(input: &str) {
    no_panic("parse_for", input, |s| {
        let _ = parse_for(s);
    });
    no_panic("transform_ts_to_js", input, |s| {
        let _ = transform_ts_to_js(s);
    });
    no_panic("parse_to_ast_json", input, |s| {
        let _ = parse_to_ast_json(s);
    });
    no_panic("declared_bindings", input, |s| {
        let _ = declared_bindings(s);
    });
    no_panic("declared_bindings_with_spans", input, |s| {
        let _ = declared_bindings_with_spans(s);
    });
    no_panic("referenced_identifiers", input, |s| {
        let _ = referenced_identifiers(s);
    });
    no_panic("referenced_identifiers_with_spans", input, |s| {
        let _ = referenced_identifiers_with_spans(s);
    });
    no_panic("free_identifiers", input, |s| {
        let _ = free_identifiers(s);
    });
    no_panic("free_identifiers_with_spans", input, |s| {
        let _ = free_identifiers_with_spans(s);
    });
    no_panic("assigned_identifiers", input, |s| {
        let _ = assigned_identifiers(s);
    });
    no_panic("function_mutations", input, |s| {
        let _ = function_mutations(s);
    });
    no_panic("function_dependencies", input, |s| {
        let _ = function_dependencies(s);
    });
    no_panic("analyze_script", input, |s| {
        let _ = analyze_script(s);
    });
    no_panic("module_binding_references", input, |s| {
        let _ = module_binding_references(s);
    });
    no_panic("top_level_declarations", input, |s| {
        let _ = top_level_declarations(s);
    });
}

/// If a call *succeeds*, every returned byte range must be in-bounds and slice
/// back to its identifier text. This guards the span arithmetic against off-by-
/// one / unicode-boundary bugs across arbitrary input.
fn assert_span_consistency(input: &str) {
    if let Ok(ids) = referenced_identifiers_with_spans(input) {
        for (name, range) in &ids {
            assert_eq!(
                range.slice(input),
                Some(name.as_str()),
                "ref span {input:?}"
            );
        }
    }
    if let Ok(ids) = free_identifiers_with_spans(input) {
        for (name, range) in &ids {
            assert_eq!(
                range.slice(input),
                Some(name.as_str()),
                "free span {input:?}"
            );
        }
    }
    if let Ok(decls) = declared_bindings_with_spans(input) {
        for (name, range) in &decls {
            assert_eq!(
                range.slice(input),
                Some(name.as_str()),
                "decl span {input:?}"
            );
        }
    }
    if let Ok(refs) = module_binding_references(input) {
        for r in &refs {
            assert_eq!(
                r.range.slice(input),
                Some(r.name.as_str()),
                "mref span {input:?}"
            );
        }
    }
    if let Ok(ds) = top_level_declarations(input) {
        for d in &ds {
            assert_eq!(
                d.name_range.slice(input),
                Some(d.name.as_str()),
                "tld name span {input:?}"
            );
        }
    }
}

#[test]
fn adversarial_inputs_never_panic() {
    let cases = [
        "",
        " ",
        "\n\n\n",
        "\t",
        "let",
        "let x",
        "let x:",
        "let x: =",
        "const",
        "= = =",
        "=> =>",
        "for",
        "for(",
        "for(;;)",
        "for(let x of)",
        "item of",
        "of items",
        "[a,b] of",
        "{a} of",
        "const {a} of x.entries()",
        "function",
        "function(){",
        "class",
        "class {}",
        "class C extends",
        "`unterminated",
        "'unterminated",
        "\"unterminated",
        "/* unterminated",
        "// only a comment",
        "({[",
        ")}]",
        "...",
        "?.",
        "a?.b!.c",
        "a ?? b ?? c",
        "interface A { x: number }",
        "type T =",
        "enum",
        "enum E {",
        "namespace N {",
        "import",
        "import from",
        "import type",
        "export",
        "export default",
        "export const",
        "@decorator class C {}",
        "あ of \u{1F600}",
        "λ => λ",
        "\0\0\0",
        "let \u{202e}rtl = 1",
        "0xZZ",
        "1_000_000n",
        "#private",
        "class C { #x = 1 }",
        "yield await",
        "a = b = c = d = e",
        "((((((((((",
        "{{{{{{{{{{",
    ];
    for c in cases {
        exercise(c);
        assert_span_consistency(c);
    }
    // Long inputs of various shapes, kept alive across the calls. Nesting depth
    // is kept modest: SWC's recursive-descent parser (like any) can exhaust the
    // stack on pathological nesting, which aborts rather than unwinds and is
    // outside the never-panic (Result-returning) contract. These depths stay
    // comfortably below that to fuzz breadth, not parser recursion limits.
    let long_ident = "x".repeat(2000);
    exercise(&long_ident);
    let deep_nest = format!("{}1{}", "(".repeat(40), ")".repeat(40));
    exercise(&deep_nest);
    let long_chain = format!("{}b", "a.".repeat(200));
    exercise(&long_chain);
    let wide_add = (0..400).map(|i| format!("v{i} + ")).collect::<String>() + "z";
    exercise(&wide_add);
    let many_decls = (0..300)
        .map(|i| format!("let v{i} = {i}\n"))
        .collect::<String>();
    exercise(&many_decls);
    assert_span_consistency(&many_decls);
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
fn pseudo_random_fuzz_never_panics() {
    // A richer fragment set than robustness.rs: adds functions, members,
    // destructuring, TS type syntax, and mutating-method fragments so the
    // scope/mutation/dependency analyses get adversarial trees too.
    let frags: &[&str] = &[
        "let ",
        "const ",
        "var ",
        "function ",
        "class ",
        "return ",
        "for",
        "while",
        "if",
        "(",
        ")",
        "{",
        "}",
        "[",
        "]",
        " of ",
        " in ",
        ";",
        ",",
        "=",
        "=>",
        "+",
        "-",
        "*",
        ".",
        "?.",
        "??",
        "!",
        ":",
        "...",
        "a",
        "b",
        "c",
        "f",
        "x",
        "obj",
        "items",
        ".push(",
        ".map(",
        ".entries()",
        "1",
        "0",
        "\"s\"",
        "'c'",
        "`t`",
        "${",
        "number",
        "string",
        "as ",
        "satisfies ",
        "type ",
        "interface ",
        "enum ",
        "import ",
        "export ",
        "from ",
        "\n",
        " ",
        "あ",
        "λ",
        "\0",
        "@",
        "#",
    ];
    let mut rng = Lcg(0xfeed_face_dead_beef);
    for _ in 0..2500 {
        let parts = (rng.next() % 14) as usize;
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(frags[(rng.next() as usize) % frags.len()]);
        }
        exercise(&s);
        assert_span_consistency(&s);
    }
}
