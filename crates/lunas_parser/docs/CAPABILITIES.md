# What the parser can do

A capability reference for the Lunas front end: what you put in, what you get
back, and what you can build on top. For *architecture* see `DESIGN.md`; for
*progress* see `ROADMAP.md`; this file answers **"how far does the parser take
me?"**

## TL;DR

Given a `.lunas` source string, the front end gives you:

1. the file split into **`html` / `style` / `script` blocks** + **directives**,
   each with exact byte spans;
2. the template parsed into a **DOM** and a **binding-aware IR**
   (interpolations, `:if`/`:for`, events, two-way bindings, components);
3. **diagnostics** for anything wrong — it never throws and never panics;
4. a **reactivity analysis** suite over the embedded JS/TS (what each expression
   depends on, what each handler mutates);
5. the **position-mapping + reference primitives** a language server needs.

It does **not** generate runtime code, scope CSS, or type-check. Those are
downstream phases (see [Boundaries](#what-it-deliberately-does-not-do)).

```
                       ┌────────────────────────────────────────────┐
  ".lunas" source ──▶  │  lunas_parser::parse()                      │
                       │    → ParsedFile { html, style, script,      │
                       │                   directives, line_index }  │
                       │    + Vec<Diagnostic>                        │
                       └───────────────┬────────────────────────────┘
                                       │  html.dom / html.template (IR)
                                       │  script.source (raw JS/TS text)
                                       ▼
                       ┌────────────────────────────────────────────┐
  reactivity / LSP ──▶ │  lunas_script::{free_identifiers,           │
  consumers            │     analyze_script, *_with_spans, parse_for,│
                       │     transform_ts_to_js, parse_to_ast_json}  │
                       └────────────────────────────────────────────┘
```

---

## Capabilities

### 1. Split a `.lunas` file into blocks + directives

`parse(source) -> (ParsedFile, Vec<Diagnostic>)` is the single entry point.

```rust
use lunas_parser::parse;

let (file, diags) = parse(src);
file.html;        // Option<HtmlBlock>   — template
file.style;       // Option<StyleBlock>  — raw CSS text (not parsed further)
file.script;      // Option<ScriptBlock> — raw JS/TS text
file.directives;  // Vec<Directive>      — @input / @use / routing
file.line_index;  // LineIndex           — byte ↔ line/col for the whole file
```

Every block keeps its **verbatim source and a file-absolute byte range** — no
indentation is stripped — so `block.source.text == range.slice(file)` holds
exactly. That invariant is what makes position mapping (capability 5) reliable.

### 2. A DOM + a binding-aware template IR

`file.html` gives you two views of the template:

- `html.dom` — a hand-written HTML parse: elements (with case-sensitive
  `raw_name`), attributes (with `value_range`), text, comments, void & raw-text
  elements, with error recovery (auto-closing, stray tags).
- `html.template` — the **semantic IR** the code generator/LSP actually want.
  `Template { nodes: Vec<TemplateNode> }`, where a node is one of:

| `TemplateNode` | meaning |
|---|---|
| `Element` | an HTML element with **classified** attributes |
| `Component` | a tag whose name is in the `@use` table |
| `Text` | a run of `Literal` / `Interpolation` (`${…}`) segments |
| `Comment` | an HTML comment |
| `If` | a grouped `:if` / `:elseif` / `:else` cascade (`IfChain`) |
| `For` | a `:for` loop wrapping one body node (`ForBlock`) |

Attributes are classified into `TemplateAttr`:

| form | variant | holds |
|---|---|---|
| `class="x ${y}"` | `Static` | literal + interpolation segments |
| `:value="expr"` | `Bound` | the bound `Expr` (text + span) |
| `::value="lv"` | `TwoWay` | the writeback l-value `Expr` |
| `@click="h()"` | `Event` | event name + handler `Expr` |

Every interpolation and bound expression is stored as **raw JS text plus a
file-absolute span** (`Interpolation.expr` / `expr_range`, `Expr.text` /
`range`). The parser carries no JS toolchain — JS work happens on demand in
`lunas_script`. `Template::visit` / `Template::for_each_expression` walk the
tree for you.

The interpolation scanner is robust: it balances braces and skips string
literals, template-literal `${…}` substitutions, **regex literals** (so
`${ s.replace(/}/g,'') }` is not cut short), and `//` / `/* */` comments.

### 3. Directives: typed props and component imports

```
@input title:string
@input count:number = 0
@input open:boolean?            // optional
@use Card from "./Card.lunas"
```

become `Directive::Input(PropInput { name, type_annotation, default_value,
nullable, range })` and `Directive::UseComponent(UseComponent { component_name,
path, range })`. The `@use` table is what turns a `<Card>` tag into a
`TemplateNode::Component` (case-sensitive match on `raw_name`). Routing
directives map to `Directive::UseAutoRouting` / `Directive::UseRouting`.

### 4. Diagnostics, never a panic

`parse` always returns a `ParsedFile`; problems come back as `Diagnostic`s
(`Error` / `Warning` / `Hint`) with a byte `range` and a message:

- missing `html:` block, duplicate blocks;
- malformed directives;
- unterminated or empty `${…}`;
- orphan `:elseif` / `:else`, reserved bound names (`innerHtml`/`textContent`);
- HTML recovery notes.

`Diagnostic::render` formats them rustc-style (caret + line/col). The
never-panic guarantee is enforced by fuzz tests across every public entry.
`examples/check.rs` is a ready CLI: `cargo run -p lunas_parser --example check
-- file.lunas` (exits non-zero on error).

### 5. Language-server primitives

Everything needed to proxy a `.lunas` file to a TypeScript language server and
to navigate bindings:

| need | API |
|---|---|
| Which block is the cursor in? | `ParsedFile::block_at` / `block_at_line_col` → `BlockKind` |
| Map a `.lunas` position into the script | `lunas_to_script` / `script_to_lunas` |
| byte ↔ UTF-16 (LSP clients count UTF-16) | `LineIndex::utf16_line_col` / `offset_utf16` |
| Go to a binding's definition | `declared_bindings_with_spans` (declaration sites) |
| Find all references in the template | `referenced_identifiers_with_spans` + `Template::for_each_expression` |
| Scope-correct rename | `free_identifiers_with_spans` (shadowed occurrences excluded) |

`examples/lsp_demo.rs` prints a binding's declaration and every template
reference in `line:col`.

### 6. Reactivity analysis (over the `script:` block)

`lunas_script` answers "what reacts to what" without you parsing JS yourself:

| function | returns |
|---|---|
| `declared_bindings` | top-level names a script declares (let/const/var/fn/class/import) |
| `referenced_identifiers` | identifiers an expression reads (raw, member/key rules) |
| `free_identifiers` | reads **after proper lexical scoping** — the real dependency set of an expression |
| `assigned_identifiers` | identifiers an expression/handler mutates (`=`, `+=`, `++`; member→root) |
| `function_mutations` | per top-level function/arrow → the set it mutates (for `@click="add()"`) |
| `analyze_script` | bindings **and** function mutations in one parse (`ScriptAnalysis`) |

The intended flow (see `examples/reactivity_demo.rs`): intersect a template
expression's `free_identifiers` with the script's `declared_bindings` to get the
component state it depends on; use `function_mutations` to know that a handler
which calls `add()` re-renders whatever `add` mutates.

### 7. JS/TS services

| function | does |
|---|---|
| `transform_ts_to_js` | strip TypeScript → JS (SWC); validated on enums/generics/casts/type-only imports |
| `parse_to_ast_json` | parse TS/JS **natively** (no pre-strip) to a span-annotated statement projection |
| `parse_for` | parse a `:for` header into `ParsedFor { kind, binding, iterable }` (`ForKind::Of` / `ForKind::In`) |

All of `lunas_script` (the whole SWC stack) builds for
`wasm32-unknown-unknown`, so the front end can run in a browser compiler/LSP.

### 8. Resolve a component for a code generator (`lunas_compiler`)

`lunas_compiler::resolve(source) -> (ResolvedComponent, Vec<Diagnostic>)` ties
capabilities 1–7 together into the model a generator consumes — without
generating any code:

```rust
use lunas_compiler::resolve;
let (c, _diags) = resolve(src);
c.props;          // @input props
c.imports;        // @use child components
c.reactive_vars;  // top-level bindings that change, each with a stable bit index
c.dynamics;       // each reactive template expr + the reactive vars it reads (Deps)
c.handlers;       // each @event handler + the reactive vars it writes (Deps)
c.template;       // the IR (structure); script / style (raw)
```

- A **reactive variable** is a top-level binding that is mutated somewhere; each
  gets an `index` (so a dependency set is a bitmask — `Deps::mask_u128`).
- Each **dynamic part** (`${…}` text, `:attr`, `::two-way`, `:if`, `:for`
  iterable) carries the reactive indices it reads, expanded **transitively
  through function calls** (`${ total() }` depends on what `total` reads).
- Each **handler** carries the reactive indices it writes, likewise transitive
  (`@click="add()"` dirties what `add` mutates). Cycles terminate.

See `cargo run -p lunas_compiler --example resolve_demo`. This is the boundary
the project is built up to: the next phase is the generator that turns a
`ResolvedComponent` into JS.

---

## What it deliberately does **not** do

These are downstream phases, not gaps in the front end:

- **Code generation / runtime** — turning a `ResolvedComponent` into JS +
  reactivity wiring is the generator, intentionally not built yet (it needs the
  runtime API spec). Everything up to its input is done (capability 8).
- **CSS parsing / scoping** — `style:` is kept as raw text; a `lunas_css` crate
  is an open owner decision.
- **Type checking** — `lunas_script` parses and analyzes TS but does not
  type-check; that is the TypeScript LS's job (which the LSP primitives proxy).
- **Full HTML5 tree construction** — this is a pragmatic fragment parser, not a
  spec-complete HTML5 engine (no implicit `<body>`/`<head>`, table
  foster-parenting, etc.). Lexer conformance is covered by the html5lib
  tokenizer suite.

---

## Public API quick reference

**`lunas_parser`**
`parse` · `ParsedFile` (`html`/`style`/`script`/`directives`/`line_index`,
`lunas_to_script`/`script_to_lunas`/`block_at`/`block_at_line_col`) · `BlockKind`
· IR: `Template`/`TemplateNode`/`TemplateElement`/`ComponentUse`/`TemplateAttr`/
`TemplateText`/`TextSegment`/`Interpolation`/`Expr`/`IfChain`/`IfBranch`/
`BranchKind`/`ForBlock`/`ForHeader`/`StaticValue` · `HtmlBlock`/`ScriptBlock`/
`StyleBlock`/`Directive`/`PropInput`/`UseComponent`/`BlockSource` · re-exported
spans: `Diagnostic`/`Severity`/`LineCol`/`LineIndex`/`TextRange`/`TextSize`.

**`lunas_script`**
`declared_bindings`(`_with_spans`) · `referenced_identifiers`(`_with_spans`) ·
`free_identifiers`(`_with_spans`) · `assigned_identifiers` · `function_mutations`
· `function_dependencies` · `analyze_script`/`ScriptAnalysis` · `parse_to_ast_json`
· `parse_for`/`ForKind`/`ParsedFor` · `transform_ts_to_js`.

**`lunas_compiler`**
`resolve` · `ResolvedComponent` (`props`/`imports`/`style`/`script`/`template`/
`reactive_vars`/`dynamics`/`handlers`, `reactive_index`/`is_reactive`) ·
`ReactiveVar` · `DynamicPart`/`DynamicKind` · `ResolvedHandler` · `Deps`
(`indices`/`mask_u128`/`contains`/`is_empty`).

## Run it yourself

```sh
cd crates
cargo run -p lunas_parser --example parse_demo        # blocks + template IR
cargo run -p lunas_parser --example reactivity_demo   # dependency / mutation flow
cargo run -p lunas_parser --example lsp_demo          # go-to-def + find-references
cargo run -p lunas_parser --example check -- f.lunas  # diagnostics CLI
cargo run -p lunas_compiler --example resolve_demo    # resolved model for codegen
```
