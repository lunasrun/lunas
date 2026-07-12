# Reactivity fundamentals

Lunas reactivity is designed to feel invisible: you write ordinary JavaScript in
`script:`, and mutating a top-level variable updates the DOM. There is no
`ref()`, no `$state`, no wrapper to remember. This page explains what actually
makes a variable reactive and how updates are scheduled.

## What makes a variable reactive

A top-level `let` that is **mutated** somewhere in your script is reactive:

```lunas
html:
    <p>${count}</p>
    <button @click="inc()">+1</button>

script:
    let count = 0            // reactive: reassigned in inc()
    function inc() { count = count + 1 }
```

The rules the compiler applies:

- A `let` that is **reassigned** (`count = …`, `count++`) is reactive.
- A `let`/`const` that is **deeply mutated** (`arr.push(…)`, `obj.k = …`) is
  reactive.
- A variable that is never mutated is treated as a constant — template reads of
  it are assigned once at build time with no update binding. (A `const` you only
  read is the common case.)

You don't annotate any of this; the compiler analyzes your script and decides.

## How it works under the hood

Each reactive variable is assigned a compile-time **index** (0, 1, 2, …) and
wrapped in a *box*. In the compiled output your `let count = 0` becomes:

```js
const count = box(c, 0, 0);   // reactive index 0, initial value 0
```

and every read/write of `count` in your code is rewritten to go through the box
(`count.v`). You never write `.v` yourself — that's the compiler's job.

Crucially, the compiler also knows **which template parts read which indices**.
Each dynamic part is registered with the exact set of indices it depends on:

```js
bind(c, [0], () => { t0.data = `${count.v}`; });  // depends on index 0
```

A write to a box marks its index dirty and enqueues exactly the update functions
registered for that index. **There is no runtime dependency discovery** — the
whole graph is known at compile time, so reactivity setup is near-free and an
update touches neither unrelated static DOM nor unrelated dynamic parts.

### box vs deepBox

The compiler picks the box kind per variable:

| Your code | Box | Cost |
|---|---|---|
| Reassigned only (`x = …`, `x++`) | `box` — plain getter/setter | lightest, no Proxy |
| Deeply mutated (`arr.push`, `obj.k = …`) | `deepBox` — Proxy that notifies on nested mutation | Proxy only where needed |

So you can mutate an object or array in place and the DOM stays in sync:

```lunas
html:
    <p>${o.k}</p>
    <button @click="add()">set</button>

script:
    let o = {}                 // deeply mutated -> deepBox
    function add() { o.k = 1 }  // nested set notifies index for `o`
```

`deepBox` wraps nested objects lazily through a `Proxy`, caching wrappers so
object identity stays stable. Mutating array methods (`push`, `splice`, …) work
because they run with the proxy as `this`.

> **Reassign vs mutate.** Both work. `items = [...items, x]` (reassign) and
> `items.push(x)` (deep mutate) both trigger updates — the compiler picks `box`
> for the first pattern and `deepBox` for the second.

## The flush model

Writes do **not** update the DOM synchronously. Instead:

1. A write marks the variable's index dirty and enqueues its dependent update
   functions (deduplicated — an update queued twice runs once).
2. A **microtask flush** is scheduled (via `queueMicrotask`).
3. On flush, only the queued update functions run, once each.

This means **many synchronous writes coalesce into one update pass**:

```js
function inc() {
    count = count + 1     // enqueues the count binding
    total = total + 1     // enqueues the total binding
    // both flush together on the next microtask — one DOM update pass
}
```

The first paint needs no flush: each binding runs once at wiring time, so the
initial render is correct immediately.

### Observing updates deterministically

Two helpers from the runtime let you cross the flush boundary when you need to:

- `await nextTick(c)` — resolves *after* the next flush, when the DOM reflects
  your latest writes.
- `batch(c, fn)` — run `fn`, then flush **synchronously**, so a group of writes
  is applied before `batch` returns. Nested batches flush once, at the outermost
  call.

These are advanced tools; most components never need them. (`c` is the component
context; see the [API reference](../api/) for how to obtain it.)

## Related

- [Computed values](./computed.md) — lazy, memoized derived state.
- [Watchers](./watchers.md) — run side effects when state changes.
- [Template syntax](./template-syntax.md) — where reactive reads appear.
