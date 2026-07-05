# Scoped CSS

Styles in a component's `<style>` block are **scoped to that component** — they
apply only to the elements that component renders, not to its children or the
rest of the page. This is the Vue-SFC model, implemented by the `lunas_css`
crate.

> **Status — scoping engine complete, codegen wiring pending.** The scoping
> transform (`lunas_css`) is done and fully tested. Wiring it into the compiler's
> code generation (stamping the scope attribute onto rendered elements and
> injecting the rewritten `<style>`) is tracked as a **later integration task** in
> the crate itself. Treat the syntax below as the designed-for contract; verify
> against your compiler version before relying on scoped output. This page
> documents the real transform and the intended integration.

## The idea

```html
<template>
  <button class="btn">Save</button>
</template>

<style>
  .btn {
    color: white;
    background: rebeccapurple;
  }
</style>
```

The `.btn` rule only styles *this* component's button, not a `.btn` elsewhere in
the app. That isolation is achieved by **attribute stamping**, not by hashed
class names.

## How scoping works — attribute stamping

Each component *type* gets a unique scope attribute, computed once and stable
across builds:

```rust
// lunas_css public API
scope_id(component_source) -> "data-lunas-<fnv1a hex>"
scope_css(css, scope_attr) -> (rewritten_css, diagnostics)
```

Two things then happen:

1. **DOM side:** codegen stamps the scope attribute (as a value-less attribute)
   on the component's root element and **every descendant element it renders**.
   Elements rendered by *child* components do **not** get the parent's attribute
   — which is what makes `:deep()` meaningful.
2. **CSS side:** every compound selector in the `<style>` block gains
   `[scope_attr]`, and the rewritten CSS is injected in a `<style>` tag once per
   component type (first mount inserts it; a set of injected ids prevents
   duplicates).

So the example above becomes, at runtime, roughly:

```html
<button class="btn" data-lunas-1a2b3c>Save</button>
```
```css
.btn[data-lunas-1a2b3c] { color: white; background: rebeccapurple; }
```

The attribute is stable across builds, so **SSR output and client hydration
agree** on the same scope id.

## Selector rewriting

Every compound selector in every rule is scoped:

| You write | Becomes (`scope = data-lunas-x`) |
|---|---|
| `.btn:hover` | `.btn[data-lunas-x]:hover` |
| `ul li` | `ul[data-lunas-x] li[data-lunas-x]` |
| `a > b` | `a[data-lunas-x] > b[data-lunas-x]` |
| `::before` | `[data-lunas-x]::before` |

## Escaping the scope: `:deep()` and `:global()`

Sometimes you need to style something the scope wouldn't reach.

### `:deep()` — pierce into child components

Child-rendered elements don't carry the parent's scope attribute, so a normal
scoped rule can't reach them. `:deep(...)` styles from the scoped side up to the
boundary, then leaves the inner part unscoped:

```css
/* style a .title rendered by a child component inside my .card */
.card :deep(.title) { font-weight: bold; }
```

becomes `.card[data-lunas-x] .title` — the `.card` is scoped to this component,
but `.title` matches regardless of which component rendered it.

### `:global()` — opt out entirely

`:global(...)` unwraps to an unscoped selector:

```css
:global(.modal-open) { overflow: hidden; }
```

becomes `.modal-open` — a truly global rule.

> **Limitation:** a top-level `:global(...)` anywhere in a selector makes the
> **entire** selector global (each wrapper is unwrapped, nothing is scoped).
> Lunas does not support the CSS-Modules-style mixed form where only part of one
> selector is global.

## At-rules

- `@media`, `@supports`, `@layer`, `@container`, `@scope` — recursed into; the
  rules inside them are scoped normally.
- **`@keyframes name`** (including vendor-prefixed) — **renamed** to `name-<hash>`,
  and `animation` / `animation-name` declarations referencing it are rewritten to
  match. Forward references work (a pre-pass collects names first), so you can
  reference an animation before its `@keyframes` block. This keeps one
  component's animation from colliding with another's identically-named one.
- `@font-face`, `@page`, `@import`, `@charset`, and unknown at-rules — passed
  through **untouched**.

## Robustness

`lunas_css` is a hand-written tokenizer and structural walker with **no external
CSS parser dependency**, builds on `wasm32-unknown-unknown`, and **never panics**:
a structurally unparseable region is emitted **verbatim** with a warning
`Diagnostic` rather than throwing. It is not a CSS validator — declarations are
not grammar-checked, and CSS nesting (`&`) is passed through rather than scoped
specially. Output preserves your original formatting (it's a string rewrite, not
an AST re-serialize).

## Gotchas

- **Scoping is per component *type*, not per instance.** Every instance of a
  component shares the same scope attribute and one injected `<style>` tag.
- **Child elements aren't reached by default** — use `:deep()` to style into
  child components on purpose.
- **`:global` is all-or-nothing per selector** (see the limitation above). Split
  a mixed rule into a scoped rule and a separate `:global` rule.
- **`@keyframes` names are rewritten**, so referencing an animation by its raw
  name from *outside* the component (or from unscoped JS) won't find it — the
  runtime name is `name-<hash>`.
- **Diagnostics carry ranges relative to the style block**; the compiler rebases
  them onto the `.lunas` file, so a CSS warning points at the right line in your
  source.

## See also

- [Teleport](../built-ins/teleport.md) — teleported content keeps the declaring
  component's scope attributes, so scoped rules still apply.
- [Tooling](./tooling.md) — how the Vite plugin compiles `.lunas` files.
- `crates/lunas_css/README.md` — the transform's authoritative reference and
  integration contract.
