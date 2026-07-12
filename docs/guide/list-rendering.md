# List rendering

`:for` renders an element once per item of an iterable. Like `:if`, it is an
attribute on the element that becomes the repeated body.

## Basic `:for`

```lunas
html:
    <ul>
        <li :for="n of history">Was ${n}</li>
    </ul>

script:
    let history = [1, 2, 3]
```

The value of `:for` is a JavaScript **for-loop header** — the part inside
`for(...)`. The common form is `item of items`, but the header is real
destructuring/iteration syntax:

```lunas
<li :for="item of items">${item.label}</li>
<li :for="tag of group.tags">${tag}</li>
```

The loop binding (`item`, `tag`, …) **shadows** your reactive variables inside
the item, so `item.label` is read directly — it is item-local data, not a
top-level box.

## Keys

Add `:key` to give each item a stable identity. This is what lets Lunas move
existing DOM nodes on update instead of rebuilding them:

```lunas
html:
    <ul>
        <li :for="item of items" :key="item.id">
            ${item.label}
        </li>
    </ul>

script:
    let items = [{ id: 1, label: "a" }, { id: 2, label: "b" }]
    function add() { items.push({ id: items.length + 1, label: "n" }) }
```

Use a key that is **stable and unique per item** (an `id`, or the item itself for
primitives). Duplicate keys are reported via a warning hook. Without `:key`,
items fall back to identity by value.

## How rendering works

Lunas splits the initial render from updates for speed:

- **Initial render — one bulk parse.** Every item's static skeleton is
  concatenated into a **single `innerHTML`** string and parsed once, then each
  item's dynamic parts are wired. A 1,000-row list is essentially one native
  parse, not 1,000 element constructions.
- **Updates — keyed diff, node identity preserved.** On update the reconciler
  compares keys and applies the minimal set of insert / remove / **move**
  operations. Existing items keep their DOM nodes (and their listeners, focus,
  and input state); only their changed data cells re-run. Moves are minimized
  using a longest-increasing-subsequence (LIS) diff, so reordering touches as few
  nodes as possible.
- **Updates never call `innerHTML`.** Re-parsing would destroy node state; the
  bulk parse is the initial-render fast path only.

You don't call any of this — writing to the array (`items.push(…)`,
`items = [...]`) triggers the diff on the next flush.

### How it compiles

```js
// <li :for="item of items" :key="item.id">${item.label}</li>
forBlock(c, a0, [0], () => Array.from((items.v) || []), {
  html: HTML_1,                 // the item skeleton, hoisted
  wire: (r0, d0) => { let item = d0; /* bind item.label, listeners */ },
  keyOf: (d0) => d0.id,
});
```

The iterable is evaluated in the outer scope (reactive on `items`); each item is
wired from its data cell `d0`, with the loop variable bound to it.

## Nested lists

Lists nest by element nesting; each inner list is per-item and tears down with
its item:

```lunas
html:
    <ul>
        <li :for="group of groups" :key="group.id">
            <b>${group.name}</b>
            <ol>
                <li :for="tag of group.tags" :key="tag">${tag}</li>
            </ol>
        </li>
    </ul>
```

When an outer item's data changes, its whole subtree — including nested `:for`
and `:if` blocks — re-evaluates against the fresh data.

## `:for` + `:if` on one element

An element may carry both `:for` and `:if`. The `:for` is the **outer** block and
`:if` the **inner** condition: the list iterates, and each item is conditionally
rendered.

```lunas
<li :for="item of items" :if="item.visible" :key="item.id">${item.label}</li>
```

If you want to filter *which* items render at all (rather than render-then-hide),
filter the source array in script or a [computed](./computed.md) value instead —
that keeps keys and the diff tidy.

## Related

- [Conditional rendering](./conditional-rendering.md) — `:if` details.
- [Reactivity fundamentals](./reactivity-fundamentals.md) — array mutation vs
  reassignment both trigger updates.
- Rendering a list of child components: [components](../components/).
