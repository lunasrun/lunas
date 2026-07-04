# Rewrite status & next steps

A handoff snapshot of the `rewrite/beta-11` front end. For architecture see
[`DESIGN.md`](../DESIGN.md) and [`template-design.md`](template-design.md).

## Done (front end + analysis + LSP foundation)

- **Parsing** — `lunas_parser::parse` → `ParsedFile { html, style, script,
  directives, line_index }`. HTML via the hand-written `lunas_html_parser`;
  template binding IR (interpolation, `:`/`::`/`@` attrs, grouped `:if`
  cascades, `:for`, `@use` components). No SWC dependency in the syntax parser.
- **JS/TS** (`lunas_script`) — `transform_ts_to_js`, `parse_to_ast_json`,
  `parse_for`, and a static-analysis suite: `analyze_script` (bindings +
  per-function mutation sets in one parse), `free_identifiers` (reactive deps),
  `assigned_identifiers` / `function_mutations`, and `*_with_spans` variants for
  the language server.
- **LSP foundation** — `block_at` routing, `lunas_to_script`, `LineIndex`
  byte<->UTF-16 conversion, `Diagnostic::render`, template find-references /
  go-to-definition (all proven end-to-end in `tests/handoff.rs`).
- **Quality** — 331 tests + 13 doctests, fuzz (never-panic) for every public
  entry, perf/deep-nesting guards, span-containment invariants, real `.lun`
  fixtures, clippy `-D warnings` clean, rustfmt, wasm32 build, CI.

## Not done — the orchestrator (`lunas_compiler`)

The only remaining major piece is the **code generator**, deliberately not
started (see the "Compilation pipeline" section of `DESIGN.md`). It is blocked
on two things:

1. A decision to start it (it was explicitly deferred).
2. The **runtime API** it targets (the npm `lunas` runtime was removed in the
   monorepo reset) — codegen output shape depends on it.

### What the orchestrator will do (inputs are all ready)

For each component:

1. `parse(src)` -> `ParsedFile`.
2. `lunas_script::analyze_script(script.text)` -> component bindings + function
   mutation sets. Add `@input` props to the binding set.
3. Walk `html.template.for_each_expression`: for each expression,
   `free_identifiers(expr)` intersected with bindings = its reactive
   dependencies. For `ForBlock`s, `parse_for(header)` first and analyze the
   iterable. For event handlers, union direct `assigned_identifiers` with the
   mutation sets of any functions they call (`function_mutations`).
4. Emit DOM-construction + reactive-update + event-listener code against the
   runtime, mapping spans back via `LineIndex`.

`examples/reactivity_demo.rs` is a working, end-to-end reference for steps 2-3.

## Smaller follow-ups (optional, non-blocking)

- `lunas_css` crate for `style:` scoping (mirrors `lunas_script`'s layering;
  the parser keeps style as raw text today).
- Full lexical-scope free-variable analysis (current `free_identifiers` uses a
  documented flat-scope approximation; adequate for template expressions).
- html5lib tree-construction tests (the tokenizer suite is already wired up).
