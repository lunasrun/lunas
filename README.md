# Lunas

Lunas is a single-file-component web framework: a `.lunas` file bundles an
`html:` template, a `style:` block, and a `script:` block (TypeScript or
JavaScript), and the compiler turns it into plain JS plus a small runtime.

This branch (`rewrite/beta-11`) is a ground-up rewrite of the compiler front
end in Rust, organized as a Cargo workspace under [`crates/`](crates/).

## Status

The **parser front end** is implemented and well-tested; the code generator /
orchestrator is not built yet (see the pipeline section in the design doc).

| crate | role |
|---|---|
| [`lunas_span`](crates/lunas_span) | shared `TextSize`/`TextRange`, `LineIndex`, `Diagnostic` — the frozen interface between the parser crates |
| [`lunas_html_parser`](crates/lunas_html_parser) | hand-written HTML lexer + recursive-descent parser (no parser library); validated against the html5lib-tests tokenizer suite |
| [`lunas_script`](crates/lunas_script) | the JS/TS "AST parser": SWC-based AST extraction, TypeScript→JavaScript transform, and `for`-header parsing |
| [`lunas_parser`](crates/lunas_parser) | the `.lunas` syntax parser: a Pest grammar splits blocks/directives, then a semantic pass builds the `ParsedFile` and a binding-aware template IR (interpolation, `:if`/`:for`, components). No JS/TS toolchain dependency. |

A `.lunas` example and the full architecture — span model, layering, the
parser-vs-AST-parser split, where TS→JS conversion happens, and the template
binding layer — are documented in
[`crates/lunas_parser/DESIGN.md`](crates/lunas_parser/DESIGN.md) and
[`crates/lunas_parser/docs/template-design.md`](crates/lunas_parser/docs/template-design.md).

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
cargo run -p lunas_parser --example check -- file.lunas   # diagnostic checker
```
