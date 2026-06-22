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

These are adopted from how production-quality parsers (SWC, rust-analyzer, Go `go/parser`) are structured:

1. **Lossless representation first** — the CST/AST retains every byte of the input, including whitespace and offsets. Analyses discard what they don't need; the parser never decides for them.
2. **Error recovery over hard failure** — the parser always produces a tree. Errors are attached to nodes, not thrown. A broken `script:` block does not prevent the HTML from being accessible.
3. **Strict layering** — each layer has one job and no knowledge of the layer above it:
   - `source` → raw text with a `SourceFile` wrapper
   - `lexer` → flat token stream
   - `parser` → CST (lossless)
   - `ast` → semantic tree (lossy, typed)
   - `lower` → domain-specific output structs consumed by the code generator
4. **Spans everywhere** — every node carries a `TextRange(start: TextSize, end: TextSize)` in byte offsets. Line/column is derived on demand via a `LineIndex`. This is the approach used by rust-analyzer and SWC.
5. **Immutable input** — the parser borrows `&str`. It never copies the source except when building owned AST string values.

---

## Layer architecture

```
┌─────────────────────────────────────────────────────────┐
│  source.rs    SourceFile { text: &str, path: Option }   │
└──────────────────────────┬──────────────────────────────┘
                           │ &str
┌──────────────────────────▼──────────────────────────────┐
│  lexer.rs     Lexer → Iterator<Item = Token>            │
│               Token { kind: TokenKind, range: TextRange }│
└──────────────────────────┬──────────────────────────────┘
                           │ Vec<Token>
┌──────────────────────────▼──────────────────────────────┐
│  parser.rs    Parser → GreenNode (lossless CST)         │
│               Every trivia (whitespace, newlines)        │
│               is preserved as a trivia token.            │
└──────────────────────────┬──────────────────────────────┘
                           │ GreenNode
┌──────────────────────────▼──────────────────────────────┐
│  ast/         Typed wrappers over CST nodes             │
│  ast/file.rs  LunasFile, LanguageBlock, Directive       │
└──────────────────────────┬──────────────────────────────┘
                           │ ast::LunasFile
┌──────────────────────────▼──────────────────────────────┐
│  lower.rs     Lowers AST → ParsedFile                   │
│               Validates, extracts spans, calls HTML/JS   │
│               sub-parsers.                               │
└──────────────────────────┬──────────────────────────────┘
                           │ ParsedFile (public output)
```

The **public entry point** (`parse`) returns a `ParsedFile` plus a `Vec<Diagnostic>`. Callers that only need the final output never touch the CST. Callers that need IDE features (hover, go-to-definition) can walk the CST directly.

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
    /// Sorted byte offsets of every '\n'.
    newlines: Vec<TextSize>,
}

pub struct LineCol { pub line: u32, pub col: u32 }  // 0-based

impl LineIndex {
    pub fn line_col(&self, offset: TextSize) -> LineCol;
    pub fn offset(&self, lc: LineCol) -> TextSize;
}
```

`TextRange` is attached to every CST node, every AST node, and every `Diagnostic`. The `LineIndex` is constructed from the source text and stored in `ParsedFile`. The LS proxy calls `line_col` / `offset` to convert between `.lunas` positions and extracted-block positions without any re-parsing.

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

## JS/TS sub-parser

Script block content is passed to SWC. TypeScript is first stripped to JavaScript (`ts_to_js`), then parsed to an AST (`swc_parser`). SWC provides its own span model (byte offsets relative to the start of its input); spans exposed to callers carry the raw SWC `lo`/`hi` so they can be correlated.

### AST representation

`ScriptBlock.ast` holds a **span-annotated JSON projection** of the top-level statements (`{ "type": "Module", "body": [{ "type": …, "span": { lo, hi } }] }`), not the full SWC AST tree.

The full tree would require SWC's `serde-impl` feature, whose `ast_node`-generated deserializer references `swc_common::private::content`, a path that does not resolve against the `serde`/`swc_common` versions currently published on crates.io — the original `main` tree no longer builds for the same reason. Rather than pin the entire dependency graph to a yanked/older `serde`, the projection captures what the code generator and language server actually need (statement kinds + locations). Consumers requiring the complete AST re-parse `ScriptBlock.js` with SWC directly. This is revisited if/when the upstream `serde-impl` alignment is fixed.

### Reproducible builds

Because the working `serde`/`swc` version set is narrow (see above), `Cargo.lock` is committed. A fresh clone must build against the frozen versions, not whatever the registry resolves to today.

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
├── lunas_parser/
│   ├── Cargo.toml
│   ├── DESIGN.md
│   └── src/
│       ├── lib.rs              pub use parse; pub use types::*;
│       ├── source.rs           SourceFile, TextSize, TextRange, LineIndex
│       ├── lexer.rs            Lexer, Token, TokenKind
│       ├── parser.rs           CST builder using token stream
│       ├── ast/
│       │   ├── mod.rs
│       │   ├── generated.rs    typed CST wrappers (LunasFile, LanguageBlock, …)
│       │   └── traits.rs       AstNode trait
│       ├── lower.rs            AST → ParsedFile
│       ├── js/
│       │   ├── mod.rs
│       │   ├── swc_parse.rs    invoke SWC, rebase spans
│       │   └── ts_to_js.rs     strip TS types via SWC transforms
│       ├── for_parser.rs       for..of / for..in expression parser
│       └── error.rs            Diagnostic, Severity
│
└── lunas_html_parser/
    ├── Cargo.toml
    └── src/
        ├── lib.rs              pub use parse_html; pub use dom::*;
        ├── lexer.rs            state-machine tokenizer
        ├── parser.rs           recursive descent tree builder
        ├── dom.rs              Dom, Node, Element, Attr, DomKind, ElementKind
        └── error.rs            Diagnostic (re-uses same shape)
```

---

## Dependency budget

| crate | direct deps |
|---|---|
| `lunas_span` | `serde` |
| `lunas_html_parser` | `lunas_span`, `thiserror` |
| `lunas_parser` | `lunas_span`, `lunas_html_parser`, `swc_core`, `swc_ecma_*`, `thiserror`, `serde`, `serde_json` |

No parser-combinator library (`nom`, `pest`, `chumsky`) in `lunas_html_parser` — the hand-written lexer+parser is simpler than the format warrants. `pest` is used only for the `.lunas` outer format in `lunas_parser` where the grammar is genuinely declarative and benefits from being readable as a specification.
