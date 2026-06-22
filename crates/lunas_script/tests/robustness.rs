//! The public `lunas_script` entry points wrap SWC and promise never to panic
//! (returning `Result`/`Option` instead). Fuzz them with adversarial and
//! pseudo-random JS/TS-ish input and assert each call returns.

use lunas_script::{parse_for, parse_to_ast_json, transform_ts_to_js};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn no_panic(label: &str, input: &str, f: impl Fn(&str)) {
    let result = catch_unwind(AssertUnwindSafe(|| f(input)));
    assert!(result.is_ok(), "{label} panicked on input: {:?}", input);
}

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
        let _ = lunas_script::declared_bindings(s);
    });
    no_panic("referenced_identifiers", input, |s| {
        let _ = lunas_script::referenced_identifiers(s);
    });
}

#[test]
fn adversarial_scripts_do_not_panic() {
    let cases = [
        "",
        " ",
        "\n",
        "let",
        "let x",
        "let x:",
        "let x: =",
        "= = =",
        "for",
        "for(",
        "for(;;)",
        "item of",
        "of items",
        "[a,b] of",
        "const {a} of x.entries()",
        "function(){",
        "class {}",
        "`unterminated",
        "'unterminated",
        "/* unterminated",
        "({[",
        ")}]",
        "interface A { x: number }",
        "あ of \u{1F600}",
        "\0\0\0",
        "import from",
        "export default",
        "=>",
        "a?.b!.c",
    ];
    for c in cases {
        exercise(c);
    }
    // A long identifier, kept in its own binding to outlive the call.
    let long = "x".repeat(1000);
    exercise(&long);
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
    let frags: &[&str] = &[
        "let ",
        "const ",
        "for",
        "(",
        ")",
        "{",
        "}",
        "[",
        "]",
        " of ",
        " in ",
        ";",
        "=",
        "=>",
        "item",
        "items",
        ".entries()",
        "a",
        "b",
        "1",
        "?",
        "!",
        ":",
        "number",
        "\"s\"",
        "'c'",
        "`t`",
        "${",
        "\n",
        " ",
        "あ",
        "\0",
    ];
    let mut rng = Lcg(0x0bad_f00d_1234_5678);
    for _ in 0..1500 {
        let parts = (rng.next() % 10) as usize;
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(frags[(rng.next() as usize) % frags.len()]);
        }
        exercise(&s);
    }
}
