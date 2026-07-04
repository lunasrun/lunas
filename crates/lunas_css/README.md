# lunas_css

Component-scoped CSS for the Lunas compiler, Vue-SFC style. Hand-written
tokenizer and structural walker — no external CSS parser dependencies, builds
on `wasm32-unknown-unknown`, and never panics (malformed regions pass through
as-is with a `lunas_span::Diagnostic`).

## Public API

```rust
pub fn scope_css(css: &str, scope_attr: &str) -> (String, Vec<Diagnostic>);
pub fn scope_id(component_source: &str) -> String; // "data-lunas-<fnv1a hex>"
```

## What the transform does

Every compound selector in every rule gains `[scope_attr]`:

| input | output (`scope_attr = data-lunas-x`) |
| --- | --- |
| `.btn:hover` | `.btn[data-lunas-x]:hover` |
| `ul li` | `ul[data-lunas-x] li[data-lunas-x]` |
| `a > b` | `a[data-lunas-x] > b[data-lunas-x]` |
| `::before` | `[data-lunas-x]::before` |
| `.a :deep(.b)` | `.a[data-lunas-x] .b` |
| `:global(.modal)` | `.modal` |

At-rules:

- `@media` / `@supports` / `@layer` / `@container` / `@scope` — recursed into;
  rules inside are scoped.
- `@keyframes name` (incl. vendor-prefixed) — renamed `name-<hash>`, and
  `animation` / `animation-name` declarations referencing it are rewritten
  (forward references work; a pre-pass collects names).
- `@font-face` / `@page` / `@import` / `@charset` / unknown — passed through
  untouched.

## How codegen will use this (integration surface)

Integration into `lunas_compiler` is a later task; the contract is:

1. Compute the component's scope attribute once per component *type*:
   `let attr = scope_id(component_source);` — stable across builds, so SSR
   output and client hydration agree.
2. **DOM side:** codegen stamps `attr` (as a value-less attribute) on the
   component's root element and every descendant element it renders. Elements
   rendered by *child* components do not get the parent's attribute — that is
   exactly what makes `:deep()` meaningful.
3. **CSS side:** the raw `style:` block text (from
   `lunas_parser::ParsedFile::style`, verbatim with a file-absolute
   `TextRange`) is passed through `scope_css(css, &attr)`. The rewritten CSS is
   injected in a `<style>` tag once per component type (first mount inserts it,
   a refcount or `Set` of injected ids prevents duplicates).
4. Diagnostics returned by `scope_css` carry ranges relative to the style
   block's text; rebase them onto the `.lunas` file with
   `TextRange::shifted(style_block_start)` before rendering.

## Non-goals / known limitations

- Not a CSS validator: structurally unparseable regions are emitted verbatim
  with a warning diagnostic; declarations are not grammar-checked.
- CSS nesting (`&`) is not scoped specially — nested blocks inside a
  qualified rule's declaration block are passed through as declarations.
- A top-level `:global(...)` anywhere makes the *entire* selector global
  (each wrapper is unwrapped, nothing is scoped) — Lunas does not support the
  CSS-Modules-style mixed form where only part of a selector is global.
- String-based rewrite: output preserves the input's formatting rather than
  re-serializing an AST.
