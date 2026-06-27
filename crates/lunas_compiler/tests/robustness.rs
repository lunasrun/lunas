//! `resolve` must never panic, matching the parser's never-panic guarantee.

use lunas_compiler::resolve;

#[test]
fn malformed_inputs_do_not_panic() {
    let cases = [
        "",
        "html:",
        "script:\n    let",
        "html:\n    <div :if=\"\" :for=\"\" @click=\"\">${}</div>",
        "html:\n    <p>${ a.b.c( }</p>\nscript:\n    let a = 0",
        "@input\n@use\nhtml:\n    <X/>",
        "html:\n    <button @click=\"a = b = c++\">x</button>\nscript:\n    let a=0\n    let b=0",
        "html:\n    <li :for=\"x of\">y</li>\nscript:\n    let z = 0",
        "script:\n    function f(){ return g() }\n    function g(){ return f() }", // mutual recursion
        "🦀 html:\n    <p>${ 日本語 }</p>",
    ];
    for case in cases {
        let (_component, _diags) = resolve(case);
    }
}

#[test]
fn mutually_recursive_functions_terminate() {
    // The transitive-closure walk must handle cycles without looping forever.
    let (c, _) = resolve(
        "\
html:
    <button @click=\"ping()\">${n}</button>
script:
    let n = 0
    function ping(){ n++; pong() }
    function pong(){ ping() }
",
    );
    assert!(c.is_reactive("n"));
    // ping (transitively via pong -> ping) writes n.
    assert_eq!(
        c.handlers[0].writes.indices(),
        &[c.reactive_index("n").unwrap()]
    );
}
