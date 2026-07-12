# Parser completion roadmap

Tracks the work to make the Lunas **front end** (parsing + analysis + LSP
support) complete. The code *generator* / orchestrator (`lunas_compiler`) is a
separate, deliberately-deferred phase and is out of scope here.

Legend: `[x]` done · `[ ]` remaining · `[~]` partial / documented limitation.

## Phase 1 — Foundation
- [x] `lunas_span`: `TextSize`/`TextRange`, `LineIndex`, `LineCol`, `Diagnostic`
- [x] `TextRange::shifted`, `slice`, `cover`, containment
- [x] `LineIndex` byte↔line/col, CRLF, UTF-16 conversion (`utf16_line_col`/`offset_utf16`)
- [x] `Diagnostic::render` (rustc-like)
- [x] Cargo workspace, pinned toolchain, committed `Cargo.lock`, CI (test + wasm)

## Phase 2 — HTML parser (`lunas_html_parser`)
- [x] Hand-written state-machine lexer (tags, attrs, comments, doctype, raw-text)
- [x] Recursive-descent tree builder → `Dom` with file-absolute spans (`shift_ranges`)
- [x] Void + raw-text elements; attribute forms; `raw_name`; `value_range`
- [x] Error recovery (auto-close, stray/dangling tags) — never panics
- [x] html5lib-tests **tokenizer** conformance (in-scope subset + reported gaps)
- [x] Span-containment property tests; fuzz; attribute/raw-text edge cases
- [~] html5lib-tests **tree-construction**: not pursued. The suite mostly
      exercises full HTML5 tree-construction quirks (implicit `<body>`/`<head>`,
      table foster-parenting, active formatting elements) that this pragmatic
      `.lunas`-template parser intentionally does not implement. Lexer-level
      conformance is covered by the tokenizer suite; basic nesting correctness
      by the span-containment / parser tests.

## Phase 3 — `.lunas` syntax (`lunas_parser`)
- [x] Pest grammar: language blocks + directives (column-0, indented bodies)
- [x] `parse1` → `RawItem`; `lower` → `ParsedFile`
- [x] Inline directive content (`@input name:type[?] [= v]`, `@use Name from "p"`)
- [x] `@useAutoRouting` / `@useRouting`
- [x] Verbatim block extraction (no indentation stripping → exact positions)
- [x] Validation diagnostics (missing html, duplicate blocks, bad directives)

## Phase 4 — Template binding layer (`lunas_parser/template`)
- [x] `${…}` interpolation — brace/string/template-literal balanced scanner
- [x] Attribute classification: `:bound` / `::two-way` / `@event` / static
- [x] Static attribute value interpolation
- [x] `:if`/`:elseif`/`:else` grouped into one `IfChain` at parse time
- [x] `:for` (with `:for` outer / `:if` inner precedence)
- [x] Components resolved via the `@use` name table (case-sensitive)
- [x] Never-panic diagnostics (unterminated/empty `${}`, orphan `:else`, …)
- [x] `Template::visit`, `for_each_expression`; `TemplateText` predicates
- [x] Interpolation scanner: regex-literal + comment awareness (`${ /}/.test(x) }`)

## Phase 5 — JS/TS analysis (`lunas_script`)
- [x] `transform_ts_to_js` (validated on enums/generics/casts/type-only imports/…)
- [x] `parse_to_ast_json` (span-annotated statement projection)
- [x] `parse_for` (for-header binding/iterable)
- [x] Reactivity: `declared_bindings`, `free_identifiers`, `assigned_identifiers`,
      `function_mutations`, `analyze_script`
- [x] LSP spans: `referenced_/free_/declared_*_with_spans`
- [x] `free_identifiers` uses **proper lexical scoping** (scope stack over
      function/arrow params + block-scoped declarations), so an outer free name
      is reported even when an inner scope shadows it

## Phase 6 — Language-server foundation
- [x] `ParsedFile::block_at` (route a position to html/style/script)
- [x] `lunas_to_script` / `script_to_lunas`
- [x] Template find-references / go-to-definition (file-absolute, end-to-end tested)
- [x] UTF-16 position conversion for LSP clients

## Phase 7 — Quality & docs
- [x] Fuzz (never-panic) for every public entry across all crates
- [x] Large-input / deep-nesting perf guards
- [x] Real `.lun`/`.lunas` fixtures + serde round-trip locks
- [x] clippy `-D warnings` clean, rustfmt, wasm32 build
- [x] Doctests on the public API
- [x] `DESIGN.md`, `template-design.md`, `STATUS.md`, README, MIT LICENSE
- [x] Runnable examples: `parse_demo`, `reactivity_demo`, `lsp_demo`, `check`

## Phase 8 — Resolution layer (`lunas_compiler`)
- [x] `resolve(source) -> (ResolvedComponent, Vec<Diagnostic>)`; never panics
- [x] Extract props (`@input`) and child components (`@use`)
- [x] Number reactive variables (declared **and** mutated), with decl spans
- [x] `function_dependencies` (per-function read deps) added to `lunas_script`
- [x] Annotate each dynamic template part (text/attr/two-way/`:if`/`:for`) with
      its reactive read-dependency set, transitively through function calls
- [x] Annotate each `@event` handler with its reactive write set (transitive)
- [x] `Deps` (sorted indices + `mask_u128` bitset); cycle-safe closure
- [x] `examples/resolve_demo.rs`; robustness/never-panic tests

## Remaining (separate phase, owner decision required)
- [~] **Code generator** — emit JS + reactivity wiring from a `ResolvedComponent`
      - [x] Wave 1 (`lunas_compiler::compile`): static skeleton + positional
            refs + runtime text anchors + reactive text/attr binds + event
            listeners; script rewrite (reactive `let` → box, refs → `.v`).
            Verified end-to-end against the real runtime under Node.
      - [ ] Wave 2: `:if` / `:for` / two-way / child components (currently voided
            with a `/* TODO(wave2) */` marker so modules still compile & run).
- [ ] `lunas_css` crate for `style:` scoping (style is raw text today, by design)

## Status

**The parser front end and the resolution layer are complete.** Every Lunas
syntax form is parsed correctly (with proper lexical scoping in the analysis),
and `lunas_compiler::resolve` now produces the full `ResolvedComponent` —
numbered reactive variables, per-dynamic dependency sets, per-handler write
sets — which is exactly the input a code generator consumes. The project is
built up to *just before* code generation.

The only remaining work is the **code generator** itself (emitting JS from a
`ResolvedComponent`), which needs the runtime API spec. The one `[~]` item
(html5lib tree-construction) is a consciously-accepted non-goal.
