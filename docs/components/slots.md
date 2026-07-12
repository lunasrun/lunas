# Slots — projecting parent content into a child

Props pass *values* to a child; **slots** pass *content* — template markup the
parent writes but the child decides where to render. A child declares outlets
with `<slot>`; the parent fills them by writing content between the child's
tags. Slots come in three forms: the **default slot**, **named slots**, and
**scoped slots**.

Crucially, **parent-provided slot content is wired in the parent's scope** — it
reads and reacts to *parent* state, even though it renders inside the child's
DOM. The child only decides the *location*.

## Default slot + fallback

A child marks where children go with `<slot>`, optionally with fallback content
inside it:

```lunas
<!-- Card.lunas (child) -->
html:
    <section class="card">
        <main><slot>empty</slot></main>
    </section>
```

The parent fills the default slot by writing content between the child's tags:

```lunas
<!-- parent -->
@use Card from "./Card.lunas"
html:
    <Card>
        Hello from the parent, ${user}.
    </Card>
script:
    let user = "ada"
```

- If the parent provides content, it renders in place of `empty`.
- If the parent provides **no** content, the child's fallback (`empty`) renders.

Because the content is wired in the parent, `${user}` is the *parent's* `user`,
and it stays reactive: changing `user` in the parent updates the text in place,
inside the child's `<main>`.

### How it compiles

The child's `<slot>` becomes a `slotBlock` at a text anchor; the parent's content
becomes a `default` factory on the child's `$slots` object:

```js
// child: <slot>empty</slot>
slotBlock(c, anchor, props.$slots && props.$slots["default"], () => /* fallback: "empty" */);

// parent: the content between <Card>…</Card>
mountChild(c, anchor, Card, {
  $slots: {
    default: (slotProps, onCleanup) =>
      slotContent(c, (slotProps) => { /* wire content in the PARENT */ }, slotProps, onCleanup),
  },
});
```

- `slotBlock` renders the parent-provided factory if present, else the child's
  own fallback, else nothing. It is null-safe.
- `slotContent` opens a scope on the **parent** context, wires the content there,
  and registers teardown via `onCleanup` so the content's binds are dropped when
  the child unmounts — no leaks, no late writes.

## Named slots

A child can expose several outlets by naming them:

```lunas
<!-- Card.lunas (child) -->
html:
    <section class="card">
        <header><slot name="head">untitled</slot></header>
        <main><slot>empty</slot></main>
        <footer><slot name="foot"></slot></footer>
    </section>
```

The parent targets a named slot with `<template #name>`:

```lunas
<!-- parent -->
@use Card from "./Card.lunas"
html:
    <Card>
        <template #head>Dashboard for ${user}</template>
        The counter starts here.
        <template #foot>the footer</template>
    </Card>
script:
    let user = "ada"
```

- `<template #head>…</template>` fills the `head` slot.
- Bare content (not inside a `<template>`) goes to the **default** slot.
- A named slot with no matching parent content shows its own fallback (`untitled`
  for `head`), or nothing if it has none.

### `<template #x>` syntax variants

These are equivalent ways to target a named slot from the parent:

| Syntax | Meaning |
|---|---|
| `<template #head>…</template>` | shorthand — fills the `head` slot. |
| `<template slot="head">…</template>` | long form of the same. |
| `<template>…</template>` (no name) | inlines its children into the **default** slot. |

### How named slots compile

Each `<template #x>` becomes an `x` entry on the parent's `$slots` object; the
child's `<slot name="x">` reads `props.$slots["x"]`:

```js
// child: <slot name="head">untitled</slot>
slotBlock(c, anchor, props.$slots && props.$slots["head"], () => /* "untitled" */);

// parent
mountChild(c, anchor, Card, {
  $slots: {
    head:    (sp, onCleanup) => slotContent(c, (sp) => { /* wire in parent */ }, sp, onCleanup),
    default: (sp, onCleanup) => slotContent(c, (sp) => { /* wire in parent */ }, sp, onCleanup),
  },
});
```

## Scoped slots

Sometimes the child has data the parent's slot content wants to render — e.g. a
list row, or a computed count. The child exposes it as a **scoped prop** on the
`<slot>`, and the parent receives it in the `<template>` binding:

```lunas
<!-- Card.lunas (child) -->
html:
    <footer><slot name="foot" :count="rows"></slot></footer>
script:
    let rows = 3
```

```lunas
<!-- parent -->
@use Card from "./Card.lunas"
html:
    <Card>
        <template #foot="s">rows: ${s.count}</template>
    </Card>
```

- The child's `<slot :count="rows">` exposes `{ count: rows }` up to the parent.
- The parent binds those props to `s` via `#foot="s"` and reads `s.count`.
- `slot-scope="s"` is the long form of `#foot="s"`. On a named slot it pairs with
  `slot="foot"` (`<template slot="foot" slot-scope="s">`).
- **Default scoped slot.** A `<template slot-scope="p">` with no `slot=`/`#name`
  (equivalently `<template #="p">`) binds `p` to the **default** slot's props —
  the Vue-2 long form for scoping the default slot.

### How scoped slots compile

The child's `<slot :count="rows">` emits a trailing scoped-props getter; the
parent's `#foot="s"` binds it into the content build:

```js
// child: <slot name="foot" :count="rows"></slot>
slotBlock(c, anchor, props.$slots && props.$slots["foot"], fallbackOrNull,
          () => ({ count: rows }));   // scoped-props getter

// parent: <template #foot="s">rows: ${s.count}</template>
{
  foot: (s, onCleanup) => slotContent(c, (s) => { /* read s.count */ }, s, onCleanup),
}
```

### Scoped-slot reactivity — the implemented semantics (read this)

Lunas implements a **restricted, honest** form of scoped-slot reactivity. Know
exactly what you get:

- **What works today (the common case):** the child's scoped props are captured
  **once, at slot build time**, via the scoped-props getter (`slotPropsOf()`).
  They are a **snapshot object**, not a live reactive channel. The parent's slot
  content reads them through the bound name (`s.count`) and renders **correctly on
  first paint**. Scoped props whose shape and values are fixed at build — which is
  the common case, e.g. a stable row object or a count that doesn't change after
  mount — work fine.
- **Parent state stays fully reactive.** Any *parent* state the slot content also
  reads (`${user}`, other parent variables) is wired with parent-scope binds and
  updates normally. Only the *child-supplied* scoped values are the snapshot.
- **The current limitation:** if the **child later mutates** a scoped value
  (`rows = 4` after mount) and expects the parent's slot content to re-render
  from that change, **it will not** — the snapshot is not a live channel. Making
  child→parent scoped props reactive needs a per-scoped-prop bridge (analogous to
  `setProp`, but flowing child → parent), which is **deferred**. It is an additive
  change on top of the current contract: nothing you write today breaks when it
  lands.

In short: scoped props render correctly on first paint and reflect their value at
build time; live child→parent scoped-prop updates after mount are not yet wired.
Don't rely on a child mutating a scoped value to push a parent re-render.

## Teardown

Slot content is torn down with the child. When the child unmounts — e.g. it lives
inside an `:if` that toggles off — the parent-owned slot binds are dropped via
the `onCleanup` teardown registered in `slotContent`. They do not fire
afterwards: no leaks, no late writes into removed DOM.

## Content can contain components

Slot content is ordinary parent template markup, so it can itself include child
components. A component inside slot content mounts inside the outer child's slot
outlet, wired in the parent's scope as usual:

```lunas
@use Card    from "./Card.lunas"
@use Counter from "./Counter.lunas"
html:
    <Card>
        <Counter :start="seed"/>   <!-- mounts inside Card's default slot -->
    </Card>
script:
    let seed = 10
```

## Gotchas

- **Slot content reads *parent* state, not child state.** It is wired in the
  parent scope. To read child data, use a **scoped slot** (`:x` on the child
  `<slot>` → `#name="p"` in the parent).
- **Scoped props are a build-time snapshot.** Correct on first paint; not a live
  child→parent channel (see above). Deferred.
- **Bare content is the default slot.** Only `<template #x>` / `<template
  slot="x">` route to a named slot.
- **A named slot with no content shows its fallback** (or nothing) — parent
  content is optional per slot.

## Related

- [props.md](./props.md) — passing *values* (slots pass *content*).
- [events.md](./events.md) — child → parent events.
- [registration.md](./registration.md) — `@use` the component you fill slots on.
- [../built-ins/](../built-ins/) — `<template>` and control-flow built-ins.
- [../api/](../api/) — `slotBlock`, `slotContent` runtime helpers.
