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
- [ ] html5lib-tests **tree-construction** (fragment subset) conformance

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
- [~] `free_identifiers` uses a documented flat-scope approximation
- [ ] Full lexical-scope free-variable analysis (correctness refinement)

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

## Out of scope (separate phases, owner decision required)
- [ ] `lunas_compiler` orchestrator + code generator (deferred; needs runtime API)
- [ ] `lunas_css` crate for `style:` scoping (style is raw text today, by design)

## Status

Front end is essentially complete. Remaining *parser-scope* items: full lexical
scoping (Phase 5) and html5lib tree-construction conformance (Phase 2). Worked
next, top to bottom.
