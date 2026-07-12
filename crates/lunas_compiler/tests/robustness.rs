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

#[test]
fn inline_handler_mutations_never_panic() {
    // Inline `@event` handler analysis (assignment detection + `.v` rewrite via
    // the program-mode parser) must never panic, however malformed the handler.
    let cases = [
        // Well-formed inline mutations of various shapes.
        "html:\n    <button @click=\"n = n + 1\">${n}</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"n++\">${n}</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"obj.k = 1\">${obj.k}</button>\nscript:\n    let obj = {}",
        "html:\n    <button @click=\"a++; b++\">${a}${b}</button>\nscript:\n    let a=0\n    let b=0",
        "html:\n    <button @click=\"arr[i] = 1\">x</button>\nscript:\n    let arr=[]\n    let i=0",
        // Malformed / partial handler bodies.
        "html:\n    <button @click=\"n = \">x</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"++\">x</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"@#$%(\">x</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"n = n +;;\">x</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"() => { n = \">x</button>\nscript:\n    let n = 0",
        "html:\n    <button @click=\"日本語 = 1; 🦀++\">x</button>\nscript:\n    let n = 0",
        // Inline mutation with no script block at all.
        "html:\n    <button @click=\"n = 1\">x</button>",
    ];
    for case in cases {
        let (_c, _d) = lunas_compiler::resolve(case);
        // The full compile path (which runs the `.v` handler rewrite) must also
        // never panic.
        let (_js, _d2) = lunas_compiler::compile(case);
    }
}
