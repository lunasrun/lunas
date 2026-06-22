# Lunas Template-Binding Layer — Design

Status: **design proposal, not implemented.** This document specifies how the
Rust rewrite should model Lunas template bindings (interpolation, control flow,
event/attribute bindings, components). It does not touch `lower.rs`, `ir.rs`, or
any `.rs` file — those are being changed concurrently.

The syntax described here is taken from the **`main` branch** (the pre-rewrite
implementation), specifically the old `lunas_generator` and `lunas_parser`
crates. Every binding form below was verified against real code on `main`, not
assumed from generic framework conventions.

---

## 1. Survey of the actual Lunas template syntax

Lunas templates are HTML with a small binding overlay. All bindings live in one
of two places: **`${...}` interpolations in text/attribute values**, or
**special attribute keys** (`:`, `::`, `@` prefixes, plus the control-flow
attributes `:if` / `:elseif` / `:else` / `:for`).

### 1.1 Text interpolation — `${ expr }`

```
<div>${count}</div>
<button>${interval==null?"Game Start":"Stop"}</button>
<div>Message from parent: ${message1}</div>
```

`${` … `}` wraps a JS expression. On `main` this was extracted by a *naive scan*
(`replace_text_with_reactive_value` in `transformers/utils.rs`):

```rust
let start_tag = "${";
let end_tag = "}";
while let Some(start) = code[last_end..].find(start_tag) { /* find next "}" */ }
```

Consequences we must improve on, but be aware of:

- The scan finds the **first** `}` after `${`. It does **not** balance braces,
  so `${ {a:1}.a }` or a `}` inside a string literal would mis-terminate. This
  is a known limitation on `main`; the new parser should brace/string-balance
  (see §4).
- Multiple interpolations per text run are allowed and each becomes its own
  binding. Static text between them is preserved verbatim.
- The expression is arbitrary JS (ternaries, member access, calls were all in
  use). Reactivity (`.v` accessor injection) is a *generator* concern, not a
  parser concern — the template IR stores the **raw expression text + span**.

### 1.2 Attribute binding — `:name="expr"`

```
<input :value="title" />
<div :class="isActive ? 'on' : 'off'">
```

A `:`-prefixed attribute means "the value is a JS expression, bind reactively."
The raw attribute name is `name = &key[1..]`. Two reserved names errored on
`main`: `:innerHtml` and `:textContent` ("not supported"). `:id` was special-
cased (not removable). Everything else became a reactive attribute whose value
is a JS expression.

### 1.3 Two-way binding — `::name="lvalue"`

```
<input ::value="title" />
```

A `::`-prefixed attribute (`binding_attr = &key[2..]`) generates **both**
directions on `main`:

- an `input` event listener: `title.v = event.target.value`
- a reactive attribute: `value = title.v`

So `::value="title"` is sugar for `:value="title"` + `@input` that writes back to
`title`. The value is an **lvalue expression** (assignment target), not a general
expression.

### 1.4 Event handler — `@event="handler"`

```
<button @click="toggle">…</button>
<input @input="onType" />
```

`@`-prefixed attribute (`action_name = &key[1..]`). The value is a JS expression
evaluated as the handler. On `main` it was wrapped by `EventTarget::new(...)`,
which decided whether the value was a bare function reference (`toggle`) or an
inline statement. The template IR should keep the raw handler text + span and
leave that classification to the generator (or a light helper).

### 1.5 Conditional rendering — `:if` / `:elseif` / `:else`

```
<div :if="(!interval&&count==100)">Success</div>
<div :elseif="other">…</div>
<div :else>Fallback</div>
```

Key facts from `main` (`html_utils.rs`):

- These are **attributes on an element**, not wrapper tags. The element they sit
  on is the conditional's body.
- `:if` carries a JS condition expression. `:elseif` carries a condition.
  `:else` carries **no value** (`:else` may be value-less).
- They form a **cascade**: `:elseif` / `:else` must be matched to a preceding
  sibling `:if`. On `main` the cascade was reconstructed *after the fact* by
  scanning sibling positions (`elm_loc`) and a shared `cascade_block_id`, and the
  effective condition for an `:else` branch was synthesized as
  `!(cond1) && !(cond2) && …`. **This sibling-grouping must move into the parser**
  in the rewrite: the IR should represent an if/elseif/else chain as one node, so
  the generator does not re-derive adjacency.
- `:else` with no matching `:if` is an error ("No matching :if statement found").

### 1.6 List rendering — `:for="header"`

```
<li :for="item of items">${item}</li>
<li :for="const [i, v] of data.entries()">…</li>
<li :for="key in mapObj">…</li>
```

`:for` carries a JS for-loop **header** (the part inside `for(...)`). This is
*already* handled by `lunas_script::parse_for`, which returns:

```rust
pub struct ParsedFor { pub kind: ForKind /* Of|In */, pub iterable: String, pub raw: String }
```

`raw` is the binding pattern (`item`, `[i, v]`, `{ k, v }`), `iterable` is the
RHS expression. So the template layer does **not** re-implement for-header
parsing; it calls `parse_for` and stores the result (plus the original header
span). `:for` and `:if` can co-occur conceptually; on `main` both are handled per
element so nesting is by element nesting.

### 1.7 Components (PascalCase) and props

```
@use()
Button from "./Button"

html:
  <Button label="click" :count="count" />
```

- A tag whose name is in the set of imported component names (declared via
  `@use` → `UseComponent { component_name, path }`, already in `ir.rs`) is a
  **component use**, not an HTML element. On `main` the check was
  `component_names.contains(&element.tag_name)` — i.e. it is *name-table driven*,
  not purely "is the first letter uppercase." PascalCase is the convention, but
  membership in the `@use` table is the actual discriminator. The rewrite should
  follow the same rule: resolve component-ness against the `@use` set.
- Props are the element's attributes. From `ComponentArgs::new` on `main`:
  a plain attribute `label="x"` is a **static** prop; a `:`-prefixed attribute
  `:count="count"` is a **bound** (reactive expression) prop. `bind = key.starts_with(":")`.
- Components are self-closing or have children (slots were not in evidence in the
  surveyed fixtures; treat children as a future concern, but the IR should not
  forbid them).

### 1.8 `@input` directive (not a template binding)

`@input message1:string` at column 0 is a **prop declaration directive**, already
modeled as `Directive::Input(PropInput)` in `ir.rs`. It is unrelated to the
`@input` *event* on an element. Listed here only to disambiguate.

### Syntax summary table

| Form | Example | Value is | Notes |
|---|---|---|---|
| Interpolation | `${count}` | JS expr | in text & attr values |
| Attr binding | `:class="x?'a':'b'"` | JS expr | `:innerHtml`/`:textContent` rejected |
| Two-way | `::value="title"` | JS lvalue | = `:value` + `@input` writeback |
| Event | `@click="toggle"` | JS expr/handler | |
| If chain | `:if` / `:elseif` / `:else` | JS cond (`:else` none) | sibling cascade |
| For | `:for="x of xs"` | for-header | via `parse_for` |
| Component | `<Button :p="x" q="y"/>` | per-attr | name-table driven; `:`=bound prop |

---

## 2. Representation decision

### Options

**A. Make `lunas_html_parser` template-aware.** Teach the HTML lexer/parser to
recognize `${…}`, `:`/`::`/`@` attributes, and `:if`/`:for` directly, emitting
typed template nodes.

**B. Keep the HTML parser pure; post-process in `lunas_parser`.** `lunas_html_parser`
keeps producing a plain `Dom` (it already does, with file-relative-then-rebased
spans). A new pass *inside `lunas_parser`* walks the `Dom`, parses `${…}` out of
`Text` and attribute values, recognizes the special attributes, groups if-chains,
and lowers everything into a richer **template IR**. Embedded JS expressions hand
off to `lunas_script`.

**C. A separate `lunas_template` crate.** Same logic as B, but in its own crate
between `lunas_html_parser` and `lunas_parser`.

### Recommendation: **Option B** (post-processing pass inside `lunas_parser`).

Rationale:

1. **Keeps `lunas_html_parser` reusable and spec-aligned.** That crate is
   validated against html5lib-tests and is described in `DESIGN.md` as "a
   pragmatic HTML tokenizer." `${…}` and `:if` are *not HTML*; baking them in
   (Option A) would (a) corrupt its html5lib conformance story, (b) make it
   non-reusable as a plain HTML parser, and (c) entangle it with `lunas_script`
   for embedded-JS parsing — a dependency it must not have. Reject A.

2. **Span fidelity is already solved for the input.** The concurrent fix makes
   `Dom` node ranges **file-absolute**. The template pass therefore inherits
   correct outer spans for free and only needs to compute *sub-spans* inside text
   runs and attribute values by simple offset arithmetic against those absolute
   ranges (see §5). No re-tokenization of the whole file.

3. **Embedded JS belongs to `lunas_parser`'s lowering stage, which is where
   sub-parsers already get called.** `DESIGN.md` already states that `lower.rs`
   "calls HTML/JS sub-parsers." `parse_for` lives in `lunas_script`, which the
   *orchestrator* — not `lunas_parser` — depends on today. This is the one real
   tension (see "Dependency note" below).

4. **C (separate crate) is not justified yet.** The pass is a tree walk plus a
   brace scanner plus calls into `lunas_script`; it shares the `@use` component
   table and `ParsedFile` assembly that already live in `lunas_parser`. A crate
   boundary would force re-exposing those. Promote to a `lunas_template` crate
   later *only if* the language server needs template analysis without the rest of
   `lunas_parser`. Until then, keep it as a module (`template/`) in `lunas_parser`.

#### Dependency note (the one architectural cost of B)

`DESIGN.md` currently asserts `lunas_parser` has **no SWC/JS dependency** and that
TS→JS / `parse_for` are invoked by the future orchestrator. Parsing `:for` headers
and (optionally) validating `${…}` expressions needs `lunas_script`. Three ways to
keep the dependency budget honest, in order of preference:

- **B1 (recommended): the template pass stores raw expression text + spans only,
  and does NOT call `lunas_script` itself.** `:for` headers are stored as a raw
  `ForHeader { text, range }`; the orchestrator calls `parse_for` afterward. This
  preserves the "no SWC in `lunas_parser`" invariant exactly as written in
  `DESIGN.md`, and matches how `script:` is already treated (raw text, parsed
  downstream). The template IR is purely syntactic: it locates and delimits every
  binding and its span, but treats the JS inside as opaque text.
- **B2: add `lunas_script` as a dependency of `lunas_parser`** and call `parse_for`
  during the pass. Cleaner IR (typed `ParsedFor` in the node) but breaks the
  stated dependency budget and pulls SWC into the language-server path.
- **B3: feature-flag** the `lunas_script` call.

**Choose B1.** It keeps `lunas_parser` SWC-free, keeps the pass a pure syntactic
splitter, and defers all JS semantics (reactivity, `.v` injection, for-header
destructuring) to the generator — exactly the layering `DESIGN.md` prescribes for
the `script:` block. The only cost is that an invalid `:for` header is reported by
the orchestrator, not the parser; the parser still reports *structural* problems
(unbalanced `${`, `:else` without `:if`, etc.).

---

## 3. Template IR

A new module `lunas_parser/src/template/ir.rs` (sketch — **not** to be added to the
existing `ir.rs`, which is owned by the concurrent change). All ranges are
`.lunas`-file-absolute `TextRange`.

```rust
/// A template node: the binding-aware analogue of html_parser::Node.
pub enum TemplateNode {
    Element(TemplateElement),
    Component(ComponentUse),
    Text(TemplateText),          // a run of static text + interpolations
    Comment(Comment),            // reuse html_parser::Comment (span only)
    If(IfChain),                 // a whole if/elseif/else cascade
    For(ForBlock),
}

/// A static/dynamic text run. `segments` interleaves literals and interpolations.
pub struct TemplateText {
    pub segments: Vec<TextSegment>,
    pub range: TextRange,
}
pub enum TextSegment {
    Literal { text: String, range: TextRange },
    Interpolation(Interpolation),
}
/// `${ expr }` — `expr` is the inner JS text; `range` covers the whole `${…}`,
/// `expr_range` covers only the expression (for LSP / diagnostics into JS).
pub struct Interpolation {
    pub expr: String,
    pub range: TextRange,
    pub expr_range: TextRange,
}

pub struct TemplateElement {
    pub name: String,
    pub kind: ElementKind,             // reuse html_parser::ElementKind
    pub attrs: Vec<TemplateAttr>,
    pub children: Vec<TemplateNode>,
    pub range: TextRange,
    pub open_tag_range: TextRange,
}

/// One element/component attribute after binding classification.
pub enum TemplateAttr {
    Static { name: String, value: Option<StaticValue>, range: TextRange },
    Bound  { name: String, expr: Expr, range: TextRange },              // :name
    TwoWay { name: String, lvalue: Expr, range: TextRange },            // ::name
    Event  { event: String, handler: Expr, range: TextRange },         // @name
}
/// A static value may itself contain ${…} interpolations.
pub struct StaticValue { pub segments: Vec<TextSegment>, pub range: TextRange }
/// Raw JS expression text + span (NOT parsed here — see §2 B1).
pub struct Expr { pub text: String, pub range: TextRange }

pub struct ComponentUse {
    pub name: String,                  // resolved against the @use table
    pub props: Vec<TemplateAttr>,      // Static => static prop, Bound => reactive prop
    pub children: Vec<TemplateNode>,   // future: slots
    pub range: TextRange,
    pub open_tag_range: TextRange,
}

/// A complete if-cascade, grouped at parse time (NOT reconstructed downstream).
pub struct IfChain {
    pub branches: Vec<IfBranch>,       // [If, ElseIf*, Else?]
    pub range: TextRange,
}
pub struct IfBranch {
    pub kind: BranchKind,              // If | ElseIf | Else
    pub condition: Option<Expr>,       // None only for Else
    pub body: Box<TemplateElement>,    // the element the directive sat on
    pub range: TextRange,
}

pub struct ForBlock {
    pub header: ForHeader,             // raw header text + span (B1)
    pub body: Box<TemplateElement>,
    pub range: TextRange,
}
pub struct ForHeader { pub text: String, pub range: TextRange }
```

### Relation to the existing `Dom`

The template IR **wraps**, not replaces, the HTML parse. The pass consumes a
`Dom` and produces a `Template` (a `Vec<TemplateNode>` + diagnostics). It reuses
`html_parser::{ElementKind, Comment}` for span-only carriers. `HtmlBlock` would
gain a `template` field alongside `dom` (see §7), so consumers that want the raw
HTML tree keep it, and consumers that want bindings use `template`.

---

## 4. Grammar / parsing approach for the binding mini-language

There is **no formal grammar** for the binding overlay (and we should not add a
Pest grammar for it — the surface is tiny and embedded JS is not Pest-parseable).
Use small hand-written scanners, mirroring the HTML parser's hand-written style.

### 4.1 Interpolation scanner (text & attr values)

Replace `main`'s naive first-`}` scan with a **brace-and-string-balanced** scan:

- On `${`, scan forward tracking `{`/`}` depth, **skipping over** string literals
  (`'…'`, `"…"`, `` `…` `` including `${}` nesting inside template literals) and
  not counting braces inside them. Terminate at the depth-0 `}`.
- Emit `Interpolation { expr, range, expr_range }`. Material between
  interpolations becomes `Literal` segments.
- Unterminated `${` (EOF / end-of-value before depth-0 `}`) → a diagnostic and a
  recovery choice: treat the remainder as literal text so the tree still builds
  (never panic — see §6).

The expression text is **not** parsed here (B1); the scanner only needs to find
the matching close brace, which requires the string/brace awareness above but not
a full JS parse.

### 4.2 Attribute classification

Pure prefix dispatch on the attribute key (matches `main`):

```
key.starts_with("::")  -> TwoWay  (name = &key[2..])
key.starts_with(":")   -> :if/:elseif/:else/:for are control flow (handled in 4.3),
                          otherwise Bound (name = &key[1..])
key.starts_with("@")   -> Event   (event = &key[1..])
otherwise              -> Static   (value may contain ${…} → scan it)
```

Reserved-name rejection (`:innerHtml`, `:textContent`) → diagnostic, drop the
attribute.

### 4.3 Control-flow grouping (`:if` cascade, `:for`)

This is the one piece that needs **sibling context**, so it runs while walking a
parent's `children` list, left to right:

- When an element has `:if`, start an `IfChain` and consume following **adjacent**
  siblings that carry `:elseif` / `:else` into the same chain. (On `main` "adjacent"
  tolerated intervening whitespace-only text nodes; replicate that — skip
  whitespace-only `Text` between branches.)
- `:elseif`/`:else` with no open chain → diagnostic ("`:else`/`:elseif` without
  matching `:if`"); emit the element as a plain element for recovery.
- `:for` wraps its single element into a `ForBlock`. The header is stored raw
  (`ForHeader`). An element may carry both `:if` and `:for`; pick a documented
  precedence (recommend: `:for` is the outer block, `:if` the inner condition, to
  match how nested control flow reads) and record both.

### 4.4 Hand-off to `lunas_script` (downstream, not in the pass)

Per §2-B1, the pass stores raw JS text + spans. The **orchestrator** later:

- calls `lunas_script::parse_for(for_header.text)` → `ParsedFor`;
- optionally parses interpolation / attr `expr` text for analysis.

Because every `Expr`/`ForHeader` carries a `.lunas`-absolute `range`, the
orchestrator can rebase any span `lunas_script` reports (which are relative to the
snippet it was handed) back to the file by adding `range.start` (§5).

---

## 5. Span handling

Invariant: **every template node, segment, and `Expr`/`ForHeader` carries a
`.lunas`-file-absolute `TextRange`.**

- **Inputs are already absolute.** The concurrent HTML fix makes `Dom` ranges
  file-absolute (the `parse_html` doc comment notes the caller rebases by the
  block's byte offset; `lower.rs` does this). The template pass therefore starts
  from absolute element/attr/text ranges.
- **Sub-spans by arithmetic, not re-parsing.** For an interpolation found at byte
  offset `i` within a `Text` whose absolute range starts at `T.start`, the
  interpolation's absolute range is `T.start + i .. T.start + j`. Same technique
  for `expr_range` inside `${…}` and for expression slices inside attribute values.
  This requires the scanner to work on the **original sliced source** with known
  absolute base offsets — so the pass should carry each text/attr value together
  with its absolute start offset, not a detached `String`. (Note: `Dom`'s `Text`
  and `Attribute` currently expose `value: String` + `range`; the absolute base is
  `range.start`, which is sufficient as long as `value` is the verbatim slice. If
  the HTML parser ever normalizes text — e.g. entity decoding — the pass must scan
  the original source slice `[range]`, not the normalized `value`, to keep offsets
  exact. Recommend scanning `&source[range]`.)
- **Rebasing JS sub-parser spans.** When the orchestrator parses an `Expr`, the
  resulting JS spans are snippet-relative; add `expr_range.start` to lift them to
  `.lunas` coordinates. `LineIndex` (in `ParsedFile`) then converts to line/col for
  the LSP, exactly as `DESIGN.md` describes for the script block.

---

## 6. Diagnostics (never panic)

The pass follows the crate-wide model (`DESIGN.md` "Error model"): always produce
a tree, attach `Diagnostic { range, severity, message }`, never `panic!` /
`unwrap` on input. Note `main`'s `parse_for` and `html_utils` are riddled with
`panic!`/`unwrap` and `Result<_, String>` — the rewrite must **not** carry those
in; the pass collects diagnostics and recovers. Cases to handle:

| Condition | Severity | Recovery |
|---|---|---|
| Unbalanced / unterminated `${` | Error | treat remainder as literal text |
| Empty interpolation `${}` | Warning | emit interpolation with empty expr |
| `:elseif` / `:else` without preceding `:if` | Error | emit element as plain element |
| `:else` carrying a value | Warning | ignore the value |
| `:if` / `:elseif` with empty condition | Error | branch kept; condition empty |
| `:for` with empty header | Error | emit body as plain element |
| Reserved bound attr (`:innerHtml`, `:textContent`) | Error | drop attribute |
| Unknown directive-looking key (e.g. `:unknownReserved`) | (none) | treat as ordinary bound attr — `:` is generic |
| Component name not in `@use` table but PascalCase | Hint/Warning | treat as element OR component? **Decide: treat as element**, warn, since `main` keyed off the table |

JS-level errors (malformed expression, bad `:for` header) are **out of scope for
the pass** under B1 and are reported by the orchestrator when it parses them; the
parser only owns structural/delimiter diagnostics. Diagnostics merge into the
`Vec<Diagnostic>` already returned by `parse`.

---

## 7. Integration with `ParsedFile`

> The concrete edits below touch `ir.rs` / `lower.rs`, which are owned by the
> concurrent change. They are specified here as the *target shape*; whoever lands
> the template work coordinates the field addition with that change rather than
> editing in parallel.

- **`HtmlBlock` gains `template`**, keeping `dom`:

  ```rust
  pub struct HtmlBlock {
      pub source: BlockSource,
      pub dom: Dom,                 // unchanged: raw HTML tree
      pub template: Template,       // NEW: binding-aware IR (§3)
  }
  pub struct Template {
      pub nodes: Vec<TemplateNode>,
      // diagnostics flow into ParsedFile's Vec<Diagnostic>, not stored here
  }
  ```

  Keeping both is cheap (the template borrows structure, owns small strings) and
  preserves the html5lib-tested `Dom` for tools that want plain HTML. If memory
  matters later, `dom` can become opt-in, but default to keeping both.

- **`lower.rs` runs the pass** after the HTML sub-parse: `parse_html` → rebase to
  absolute (already happening) → `template::build(&source[block], &dom, &use_table)`
  where `use_table` is derived from the already-lowered `Directive::UseComponent`
  list. Diagnostics append to the returned vec.

- **Orchestrator (`lunas_compiler`) consumption.** Per `DESIGN.md`, the future
  orchestrator drives codegen. The primitives it needs already exist:
  - `Template::visit` walks `HtmlBlock.template` in pre-order (no need to
    re-implement the recursive descent);
  - `lunas_script::declared_bindings(script)` gives the component's binding set;
    for each `Interpolation` / `Bound` / `Event` / `TwoWay` expression,
    `lunas_script::referenced_identifiers(expr) ∩ bindings` is the reactive
    dependency set (replacing `main`'s `append_v_to_vars_in_html` string munging),
    and `assigned_identifiers(handler) ∩ bindings` is what an event handler
    mutates (what to re-render);
  - `lunas_script::parse_for(for_header.text)` recovers the `ForBlock` binding /
    iterable;
  - it then emits DOM-construction + anchor + event-listener code, mapping spans
    back via `LineIndex`. The grouped `IfChain` means the generator no longer
    reconstructs sibling cascades (a notable simplification over `main`'s
    `elm_loc` scanning).

  In short, the only thing left to build is the codegen itself — every analysis
  input it consumes is implemented and tested.

---

## 8. Open questions & phasing

**Open questions**

1. **`:if` + `:for` on one element — precedence.** Recommend `:for` outer, `:if`
   inner; confirm against intended runtime semantics before locking the IR.
2. **Component children / slots.** No slot syntax surfaced in the surveyed
   fixtures. IR allows `ComponentUse.children`; defer slot *semantics*.
3. **Whitespace between cascade branches.** `main` tolerated whitespace-only text
   nodes between `:if`/`:elseif`/`:else`. Confirm formatter guarantees this and
   decide whether non-whitespace text breaks a chain (recommend: yes, it breaks).
4. **PascalCase-but-not-imported tags.** Decided above (treat as element + warn);
   revisit if implicit/global components are introduced.
5. **`${…}` inside attribute *names* or boolean attrs.** Not observed; assume
   interpolation only in values and text.
6. **Should the pass eventually call `lunas_script` (B2)?** Revisit once the
   language server needs in-editor `:for` diagnostics; until then stay B1.

**Suggested incremental order**

1. **IR + module skeleton** (`template/ir.rs`, `template/mod.rs`) with the types
   in §3. No logic. Re-export nothing yet.
2. **Interpolation scanner** (§4.1) over text runs only, with brace/string
   balancing + diagnostics. Unit-test against the `${...}` cases from
   `counter-game.lun` / `pass-value.lun`.
3. **Attribute classifier** (§4.2): `:` / `::` / `@` / static, plus reserved-name
   diagnostics and interpolation-in-static-values.
4. **Element/Component split** using the `@use` table; props classification.
5. **Control-flow grouping** (§4.3): `:for` wrapping, then the `:if`/`:elseif`/
   `:else` cascade with sibling walking and recovery.
6. **`HtmlBlock.template` wiring** in `lower.rs` (coordinated with the concurrent
   change) and diagnostic plumbing.
7. **(Later, orchestrator)** consume the IR for codegen and call `parse_for` /
   reactivity analysis on the raw expression text.

Phases 1–5 are pure, panic-free, SWC-free, and independently testable through
`lunas_parser`'s public `parse`, keeping the dependency budget in `DESIGN.md`
intact.
