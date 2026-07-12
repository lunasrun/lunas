# Lunas Parser — Design

## The `.lunas` file format

A `.lunas` file is a single-file component. It consists of optional metadata directives followed by indentation-delimited language blocks.

```
@input(optional)
propName: Type = defaultValue

@use()
Button from "./Button"

html:
    <div class="counter">
        <Button label="click" />
    </div>

style:
    .counter { display: flex; }

script:
    let count = 0
    const increment = () => count++
```

**Language blocks** (`html:`, `style:`, `script:`) — keyword at column 0, body indented by at least one level.

**Metadata directives** (`@input`, `@use`, `@useAutoRouting`, `@useRouting`) — `@keyword(params?)` at column 0, optional body on the following line(s).

---

## Guiding principles

These borrow from how production-quality parsers (SWC, rust-analyzer, Go `go/parser`) are structured:

1. **Verbatim extraction** — every block's `source.text` equals its slice of the input (no indentation stripping), so any position inside a block maps to the extracted text by a constant shift. The parser preserves bytes; analyses decide what to discard.
2. **Error recovery over hard failure** — the parser always produces output. Problems are attached as `Diagnostic`s, never thrown. A broken `script:` block does not prevent the HTML from being accessible. The public `parse` returns `(ParsedFile, Vec<Diagnostic>)` — no `Result`, no panics.
3. **Strict layering** — each stage has one job: the Pest grammar (`grammar/lunas.pest`) recognizes line structure, `parser1` produces raw items, `lower` does semantics and calls the HTML/template sub-parsers, and `lunas_script` (a separate crate) owns all JS/TS work.
4. **Spans everywhere** — every output node carries a `TextRange(start: TextSize, end: TextSize)` in byte offsets, file-absolute. Line/column is derived on demand via a `LineIndex` (the rust-analyzer / SWC approach). This — not a trivia-preserving CST — is how losslessness is achieved.
5. **Immutable input** — the parser borrows `&str` and copies the source only into the owned strings it returns.

---

## Layer architecture

```
┌─────────────────────────────────────────────────────────┐
│  source &str                                            │
└──────────────────────────┬──────────────────────────────┘
                           │ Pest grammar (grammar/lunas.pest)
┌──────────────────────────▼──────────────────────────────┐
│  parser1.rs   parse1 → Vec<RawItem>                     │
│               raw language blocks + directives, each     │
│               carrying a byte-offset TextRange. Pure     │
│               line structure; no semantics.              │
└──────────────────────────┬──────────────────────────────┘
                           │ Vec<RawItem>
┌──────────────────────────▼──────────────────────────────┐
│  lower.rs     lower → ParsedFile                        │
│   · validate block uniqueness / presence                │
│   · extract each block verbatim (BlockSource)           │
│   · html: parse_html → Dom, rebased file-absolute,      │
│           then template::build → Template (binding IR)  │
│   · directives → Directive (inline @input/@use/…)       │
└──────────────────────────┬──────────────────────────────┘
                           │ ParsedFile + Vec<Diagnostic>
                           ▼  (public output)
```

The **public entry point** (`parse`) returns a `ParsedFile` plus a
`Vec<Diagnostic>`; it never returns `Err` and never panics. The outer `.lunas`
structure is described declaratively by the Pest grammar (which reads as a
specification of the line format), while the bespoke logic — HTML parsing, the
template binding pass, directive parsing — lives in hand-written Rust in
`lower.rs` and the `template/` module. JS/TS is not parsed here at all; the
`script:` block is extracted as raw text for the `lunas_script` crate
downstream.

> Note: this is a pragmatic two-stage design (Pest → semantic lowering), not a
> full lossless green-node CST. The lossless/spans-everywhere property is
> achieved by attaching a file-absolute `TextRange` to every output node rather
> than by retaining a trivia-preserving syntax tree.

---

## Span model

Adopted from rust-analyzer's `text-size` crate.

```rust
/// Byte offset into the source text. Newtype over u32.
pub struct TextSize(u32);

/// A half-open byte range [start, end).
pub struct TextRange { start: TextSize, end: TextSize }

/// Maps byte offsets to (line, col) on demand.
/// Built once per file; O(log n) per query.
pub struct LineIndex {
    /// Byte offset of the start of each line (line_starts[0] == 0).
    line_starts: Vec<TextSize>,
    len: TextSize,
}

pub struct LineCol { pub line: u32, pub col: u32 }  // 0-based

impl LineIndex {
    pub fn line_col(&self, offset: TextSize) -> LineCol;
    pub fn offset(&self, lc: LineCol) -> TextSize;
}
```

`TextRange` is attached to every output node (HTML `Dom`, template IR, directives) and every `Diagnostic`. The `LineIndex` is constructed from the source text and stored in `ParsedFile`. The LS proxy calls `line_col` / `offset` to convert between `.lunas` positions and extracted-block positions without any re-parsing.

### LSP position mapping

```rust
impl ParsedFile {
    /// Convert a (line, col) in the .lunas file to the equivalent position
    /// inside the extracted script text. Returns None if outside the block.
    pub fn lunas_to_script(&self, pos: LineCol) -> Option<LineCol>;

    /// Inverse mapping: script position → .lunas position.
    pub fn script_to_lunas(&self, pos: LineCol) -> LineCol;
}
```

Both functions use `LineIndex` arithmetic only — no re-parsing.

No block is indentation-stripped: each block's `source.text` is exactly
`range.slice(file)`. The `script:` block still needs `lunas_to_script` because
its text is *extracted* (starts at a non-zero file line), but since extraction
is verbatim the mapping is a pure line shift with the **column unchanged** —
which would not hold if indentation were stripped. The `html:` block's `Dom`
node ranges are rebased onto the file by a single constant offset
(`Dom::shift_ranges`) during lowering, so HTML positions are already
`.lunas`-absolute and feed straight into `LineIndex`. `ParsedFile::block_at`
reports which block an offset falls in, so the language server can route a
request to the right backend.

---

## HTML sub-parser

The HTML parser is a **hand-written, two-phase recursive descent parser** living in its own crate (`lunas_html_parser`). It follows the same span model.

### Phase 1 — Lexer

The lexer is a state machine over `&str` that emits a flat `Vec<Token>`. It never allocates for the source text itself — `Token` contains a `TextRange` and a `TokenKind`; string content is sliced from the original `&str` in phase 2.

```
TokenKind:
  Doctype
  OpenTagStart(name)       <!-- <div -->
  Attribute(key, value)
  OpenTagEnd               <!-- > -->
  SelfCloseTagEnd          <!-- /> -->
  CloseTag(name)           <!-- </div> -->
  Text
  Comment
  RawText                  <!-- content of script/style/title/textarea -->
  Error(char)              <!-- unexpected character, enables recovery -->
```

### Phase 2 — Tree builder

A recursive descent parser consumes the token stream with an explicit element stack. Rules:

- **Void elements** (`area`, `base`, `br`, `col`, `embed`, `hr`, `img`, `input`, `link`, `meta`, `param`, `source`, `track`, `wbr`) — never pushed to the stack, never expect a close tag.
- **Raw-text elements** (`script`, `style`, `title`, `textarea`) — after the open tag the lexer is switched to raw mode; content is a single `RawText` token ending at `</name>`.
- **Mismatched close tags** — emit a `Diagnostic`, pop to the nearest matching ancestor (same recovery strategy as browsers).
- **Dangling open tags** — implicitly closed at EOF; no error for block-level elements (matches browser behaviour for `<p>`, `<li>`, etc.).

### DOM output

```rust
pub struct Dom {
    pub kind: DomKind,           // Document | Fragment | Empty
    pub children: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum Node {
    Element(Element),
    Text { text: String, range: TextRange },
    Comment { text: String, range: TextRange },
}

pub struct Element {
    pub name: String,
    pub kind: ElementKind,       // Normal | Void
    pub attributes: Vec<Attr>,
    pub children: Vec<Node>,
    pub range: TextRange,
    pub open_tag_range: TextRange,
}

pub struct Attr {
    pub name: String,
    pub value: Option<String>,
    pub range: TextRange,
}
```

---

## Template layer (binding overlay)

The plain `Dom` is post-processed into a binding-aware **template IR** by a pass
in `template/`. It splits `${…}` interpolations out of text and attribute
values (a brace/string-balanced scan), classifies `:`/`::`/`@` attributes,
resolves components against the `@use` table (case-sensitive, via
`Element::raw_name`), and groups `:if`/`:elseif`/`:else` cascades and `:for`
loops at parse time. It is purely syntactic — embedded JS is stored as raw text
plus a file-absolute span and parsed downstream — so this crate stays SWC-free.
`HtmlBlock` carries both `dom` (raw HTML) and `template` (the IR). The full
rationale, IR types, and phasing are in [`docs/template-design.md`](docs/template-design.md).

---

## Script handling: parser vs. AST parser

The `.lunas` syntax parser does **not** parse or transform script contents. It
locates the `script:` block and extracts its raw text + range into
`ScriptBlock`. That is the full extent of the parser's responsibility for
scripts. As a result `lunas_parser` has **no SWC / JS-toolchain dependency**.

All JavaScript/TypeScript work lives in a separate crate, `lunas_script` (the
"AST parser"):

- `parse_to_ast_json` — parses a script into an AST.
- `transform_ts_to_js` — lowers TypeScript to JavaScript.
- `parse_for` — parses a `for` loop header's JS binding/iterable.

### TypeScript is parsed natively — no pre-conversion

A common misconception is that TypeScript must be converted to JavaScript before
it can be parsed into an AST. It does not: SWC parses TypeScript directly. The
old pipeline (`ts_to_js` *then* parse) parsed twice and stringified in between:

```
  TS text → [parse TS, strip types, codegen] → JS text → [parse JS] → AST   ✗ two parses
  TS text → [parse TS] → AST                                                ✓ one parse
```

So `lunas_script::parse_to_ast_json` parses with TS syntax in a single pass.
Type stripping (`transform_ts_to_js`) is an independent downstream transform
that operates after parsing, not a prerequisite for it.

### AST representation

`parse_to_ast_json` returns a **span-annotated JSON projection** of the
top-level statements (`{ "type": "Module", "body": [{ "type": …, "span": { lo, hi } }] }`),
not the full SWC AST tree.

The full tree would require SWC's `serde-impl` feature, whose `ast_node`-generated
deserializer references `swc_common::private::content`, a path that does not
resolve against the `serde`/`swc_common` versions currently published on
crates.io — the original `main` tree no longer builds for the same reason.
Rather than pin the entire dependency graph to a yanked/older `serde`, the
projection captures what the code generator and language server actually need
(statement kinds + locations). Consumers requiring the complete AST re-parse the
script text with SWC directly. Revisited if/when the upstream `serde-impl`
alignment is fixed.

### Reproducible builds

Because the working `serde`/`swc` version set is narrow (see above), `Cargo.lock`
is committed. A fresh clone must build against the frozen versions, not whatever
the registry resolves to today.

---

## Compilation pipeline & orchestration

The crates in this workspace are **parts**, not a pipeline. None of them calls
another to drive an end-to-end compile; they expose pure library functions. The
wiring is owned by a future top-level **orchestrator crate** (working name
`lunas_compiler`) — the equivalent of the old `lunas_compiler` / `lunas_generator`
pair, and the artifact that gets compiled to WASM for the npm `lunas` package.

```
        ┌────────────────────────────────────────────────┐
        │ lunas_compiler  (orchestrator — NOT YET BUILT)  │
        │   1. parse        2. script transform/AST        │
        │   3. code generation → JS + runtime              │
        │   compiled to WASM; called by the npm `lunas` pkg│
        └───────┬──────────────────────────┬──────────────┘
                │                           │
     ┌──────────▼──────────┐     ┌──────────▼───────────┐
     │ lunas_parser         │     │ lunas_script          │
     │ .lunas → ParsedFile  │     │ parse_to_ast_json     │
     │ (script = raw text)  │     │ transform_ts_to_js    │
     └──────────────────────┘     └───────────────────────┘
```

Responsibilities of the orchestrator (when built):

1. `lunas_parser::parse(src)` → `ParsedFile`. The `script:` block is raw text.
2. For the script block, call **`lunas_script`**:
   `transform_ts_to_js(script.source.text)` for the emitted JS, and/or
   `parse_to_ast_json(...)` for analysis.
3. Generate the component output (DOM construction + reactivity + the lowered
   JS) and stitch in source positions via the `LineIndex`.

**Where TS→JS happens:** the *function* lives in `lunas_script` (JS/TS domain),
but it is *invoked* by the orchestrator crate — never by `lunas_parser`, which
stays a pure syntax parser with no JS/TS toolchain. This keeps the dependency
direction one-way (orchestrator → parts) and lets tools that only need parsing
(e.g. the language server) depend on `lunas_parser` alone.

This crate (`lunas_compiler`) is intentionally not created yet; it is added once
there is a generator to drive.

---

## Error model

```rust
pub struct Diagnostic {
    pub range: TextRange,
    pub severity: Severity,   // Error | Warning | Hint
    pub message: String,
}

pub enum Severity { Error, Warning, Hint }
```

A `ParsedFile` always exists. `diagnostics` is empty on success. Callers check `diagnostics` instead of matching `Result::Err`. This is the model used by rust-analyzer and Roslyn.

The public `parse` function signature:

```rust
pub fn parse(source: &str) -> (ParsedFile, Vec<Diagnostic>);
```

No `Result` — the parser never panics and always returns something useful.

---

## Testing

Tests live in independent `tests/` directories (Rust integration tests), driven
through the public APIs; only genuinely white-box unit tests (e.g. the Pest
stage, SWC glue) remain inline in `src/`.

The HTML tokenizer is additionally validated against **html5lib-tests**, the
standard cross-implementation conformance suite, vendored under
`lunas_html_parser/tests/html5lib/`. Because our parser is a pragmatic
tokenizer for `.lunas` templates rather than a spec-complete HTML5 engine, the
harness runs the in-scope subset (~400 cases) and asserts an exact match,
counts and reports the out-of-scope categories it deliberately does not
implement (character references, DOCTYPE internals, alternate tokenizer states,
NUL/CR normalization, adversarial mid-tag EOF recovery), and pins a short,
explicit list of known per-character recovery divergences as a regression
guard. Running the full ~1800-case suite keeps the coverage picture honest:
new divergences outside the known list fail the build.

---

## Crate layout

The span model and diagnostic types are shared by both parser crates, so they
live in a tiny leaf crate `lunas_span` that depends on nothing but `serde`.
This freezes the interface boundary: both `lunas_parser` and
`lunas_html_parser` are written against the same `TextRange` / `Diagnostic`
definitions without one depending on the other for primitives.

```
crates/
├── Cargo.toml                  workspace root
│
├── lunas_span/                 shared foundation (no parser logic)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── text_size.rs        TextSize, TextRange
│       ├── line_index.rs       LineIndex, LineCol
│       └── diagnostic.rs       Diagnostic, Severity
│
├── lunas_parser/                .lunas syntax only — no JS/TS toolchain
│   ├── Cargo.toml
│   ├── DESIGN.md
│   ├── examples/parse_demo.rs
│   ├── tests/integration.rs     black-box tests via the public `parse`
│   └── src/
│       ├── lib.rs               public API: `parse`, `ParsedFile`, IR re-exports
│       ├── grammar/lunas.pest   Pest grammar for the outer format
│       ├── parser1.rs           Stage 1: Pest → Vec<RawItem>
│       ├── lower.rs             Stage 2: RawItem → ParsedFile (+ HTML sub-parse)
│       ├── ir.rs               public output types (ScriptBlock = raw text only)
│       └── template/           binding-aware template IR over the HTML Dom
│           ├── mod.rs           Dom → Template pass (interpolation, :if/:for, …)
│           ├── ir.rs            template node types (see docs/template-design.md)
│           └── scan.rs          balanced ${…} interpolation scanner
│
├── lunas_script/                the JS/TS "AST parser", built on SWC
│   ├── Cargo.toml
│   ├── tests/{ast,transform,for_header}.rs
│   └── src/
│       ├── lib.rs               pub use parse_to_ast_json, transform_ts_to_js, parse_for
│       ├── ast.rs               parse script (TS natively) → AST JSON projection
│       ├── transform.rs         downstream TS → JS lowering
│       └── for_header.rs        for..of / for..in header parser
│
└── lunas_html_parser/           hand-written HTML parser — no parser library
    ├── Cargo.toml
    ├── tests/{lexer,parser,html5lib_tokenizer}.rs + html5lib/ (vendored)
    └── src/
        ├── lib.rs               pub use parse_html; pub use dom::*; (+ hidden internals)
        ├── lexer.rs             state-machine tokenizer
        ├── parser.rs            recursive descent tree builder
        └── dom.rs               Dom, Node, Element, Attribute, DomKind, ElementKind
```

---

## Dependency budget

| crate | direct deps |
|---|---|
| `lunas_span` | `serde` |
| `lunas_html_parser` | `lunas_span`, `thiserror`, `serde` |
| `lunas_parser` | `lunas_span`, `lunas_html_parser`, `pest`, `thiserror`, `serde` |
| `lunas_script` | `lunas_span`, `swc_core`, `swc_ecma_*`, `thiserror`, `serde`, `serde_json` |

The SWC/JS-toolchain dependency is confined to `lunas_script`. `lunas_parser`
depends only on `pest` (for the `.lunas` outer grammar) and the HTML parser, so
the syntax parser builds without the heavy SWC graph. No parser-combinator
library appears in `lunas_html_parser` — the hand-written lexer+parser is
simpler than the format warrants. `pest` is used only for the `.lunas` outer
format, where the grammar reads as a specification.
