# Lunas

Lunas is a single-file-component web framework: a `.lunas` file bundles an
`html:` template, a `style:` block, and a `script:` block (TypeScript or
JavaScript), and the compiler turns it into plain JS plus a small runtime.

This branch (`rewrite/beta-11`) is a ground-up rewrite of the compiler front
end in Rust, organized as a Cargo workspace under [`crates/`](crates/).

## Status

The **parser front end**, the JS/TS static-analysis suite, the language-server
foundation, and the **resolution layer** (`lunas_compiler`, which produces the
`ResolvedComponent` a code generator consumes) are implemented and well-tested.
The project is built up to *just before* code generation; the generator itself
is the remaining phase.

- **What you can actually do with it** (input → output, with examples):
  [`crates/lunas_parser/docs/CAPABILITIES.md`](crates/lunas_parser/docs/CAPABILITIES.md)
- What's done / what's next handoff:
  [`crates/lunas_parser/docs/STATUS.md`](crates/lunas_parser/docs/STATUS.md)
- Completion checklist:
  [`crates/lunas_parser/docs/ROADMAP.md`](crates/lunas_parser/docs/ROADMAP.md)

| crate | role |
|---|---|
| [`lunas_span`](crates/lunas_span) | shared `TextSize`/`TextRange`, `LineIndex`, `Diagnostic` — the frozen interface between the parser crates |
| [`lunas_html_parser`](crates/lunas_html_parser) | hand-written HTML lexer + recursive-descent parser (no parser library); validated against the html5lib-tests tokenizer suite |
| [`lunas_script`](crates/lunas_script) | the JS/TS "AST parser" (SWC): AST extraction, TS→JS transform, `for`-header parsing, and a static-analysis suite for reactivity and the language server |
| [`lunas_parser`](crates/lunas_parser) | the `.lunas` syntax parser: a Pest grammar splits blocks/directives, then a semantic pass builds the `ParsedFile` and a binding-aware template IR (interpolation, `:if`/`:for`, components). No JS/TS toolchain dependency. |
| [`lunas_compiler`](crates/lunas_compiler) | the resolution layer: combines the parser and analysis into a `ResolvedComponent` — numbered reactive variables, each dynamic template part with its dependency set, each handler with its write set. The input a code generator consumes; it does **not** generate code. |

A `.lunas` example and the full architecture — span model, layering, the
parser-vs-AST-parser split, where TS→JS conversion happens, and the template
binding layer — are documented in
[`crates/lunas_parser/DESIGN.md`](crates/lunas_parser/DESIGN.md) and
[`crates/lunas_parser/docs/template-design.md`](crates/lunas_parser/docs/template-design.md).

## Analysis & language-server support

Beyond producing the IR, the front end exposes the primitives a code generator
and a language server need. All embedded JS stays raw text + file-absolute
spans in the parser; `lunas_script` analyzes it on demand:

- **Reactivity** — `analyze_script` (a script's bindings + per-function mutation
  sets in one parse), `free_identifiers` (the reactive dependencies of an
  expression), `assigned_identifiers` / `function_mutations` (what a handler, or
  a function it calls, mutates). `examples/reactivity_demo.rs` shows the full
  flow: `@click="add()"` re-renders `items`/`count` via `add`'s mutation set.
- **Navigation** — `referenced_identifiers_with_spans` +
  `Template::for_each_expression` locate every use of a binding across the
  template (find-references / highlight / rename); `declared_bindings_with_spans`
  gives the declaration site (go-to-definition). `ParsedFile::block_at` and
  `lunas_to_script` route positions to the right backend; `Diagnostic::render`
  formats errors. `examples/lsp_demo.rs` prints a binding's declaration and all
  template references in `line:col`.

## Design principles

- **Lossless, span-everywhere parsing** — every node carries a file-absolute
  byte range; line/column is derived on demand via `LineIndex` (for the
  language server).
- **Error recovery over hard failure** — the parser always returns a tree and
  reports problems as `Diagnostic`s; it never panics (enforced by fuzz tests).
- **Strict layering** — the `.lunas` syntax parser carries no SWC/JS dependency;
  all JS/TS work is isolated in `lunas_script`, invoked by the future
  orchestrator.
- **wasm-ready** — every crate (including `lunas_script`'s SWC stack) builds for
  `wasm32-unknown-unknown`, so the front end can run in the browser compiler and
  language server. CI guards this.

## Building and testing

The workspace lives in `crates/`. The toolchain is pinned by
[`rust-toolchain.toml`](rust-toolchain.toml).

```sh
cd crates
cargo test --all          # run all tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo run -p lunas_parser --example parse_demo        # end-to-end demo
cargo run -p lunas_parser --example reactivity_demo   # reactivity analysis flow
cargo run -p lunas_parser --example lsp_demo          # go-to-def + find-references
cargo run -p lunas_parser --example check -- file.lunas   # diagnostic checker
cargo run -p lunas_compiler --example resolve_demo    # resolved model for codegen
```
