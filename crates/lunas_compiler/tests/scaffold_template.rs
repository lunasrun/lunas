//! Guards that the `create-lunas` scaffolding template's root component stays
//! valid for the current compiler: it must compile to a module with no error
//! diagnostics. If a compiler change breaks the shipped starter, this fails
//! loudly instead of the user hitting it after `npm create lunas`.

use std::path::PathBuf;

/// Path to `packages/create-lunas/template/src/App.lunas` from this crate.
fn template_app() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/lunas_compiler
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("packages/create-lunas/template/src/App.lunas")
}

#[test]
fn scaffold_template_app_compiles_without_errors() {
    let path = template_app();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    let (code, diags) = lunas_compiler::compile(&source);

    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "template App.lunas has compile errors: {errors:?}"
    );
    assert!(
        code.is_some(),
        "template App.lunas produced no module output"
    );
}
