# Computed values

A computed value is a piece of state **derived** from other reactive state. It
recomputes only when one of its inputs changes, and only when it is actually
read — it is both **lazy** and **memoized**.

## Deriving a value

The simplest derived value is a plain expression over reactive state:

```lunas
html:
    <p>${remaining} left</p>

script:
    let items = []
    const remaining = items.filter(i => !i.done).length
```

Because `remaining` reads `items`, the template part that displays it depends on
`items`; when `items` changes, the display re-evaluates.

For a derived value you want to reuse across several template parts — and pay for
only once per change — use the runtime `computed` helper. It gives the derived
value its own reactive identity so downstream reads share a single memoized
result:

```js
// deps are the reactive indices the derived value reads (compiler-supplied)
const fullName = computed(c, i, [firstNameIdx, lastNameIdx],
  () => `${first.v} ${last.v}`);

// read it like any box:
fullName.v;
```

`computed(c, i, deps, fn)` returns a read-only, box-shaped handle whose `.v`
getter yields the memoized result.

## Laziness and memoization

The semantics are precise and worth understanding:

- **Lazy compute.** `fn` runs only when `.v` is *read* **and** an input has
  changed since the last computation. A computed value that is never read never
  recomputes — even if its inputs change constantly.
- **Memoized.** After a compute, the result is cached. Reading `.v` repeatedly
  without an intervening input change returns the cached value; `fn` does not
  re-run.
- **Invalidate, don't recompute eagerly.** When an input changes, the computed
  does **not** recompute immediately. It marks itself stale and marks its own
  reactive index dirty, so any dynamic part that reads it is re-queued. Those
  parts pull the fresh value on their next run, triggering exactly one recompute.

The upshot: N downstream readers plus M input changes cost **one** recompute per
change that is followed by a read — never one per reader, never one per change
that nobody observes.

## computed vs an inline expression

| Use an inline expression when… | Use `computed` when… |
|---|---|
| the derivation is read in one place | the same derivation feeds several parts and you want it computed once |
| it is cheap | it is expensive and you want memoization |
| you don't need to reference it by name elsewhere | you want a named, reusable reactive handle |

## computed vs watch

- **`computed`** produces a *value* you render or read. It is pull-based and lazy.
- **[`watch`](./watchers.md)** runs a *side effect* when state changes. It is
  push-based and eager.

Reach for `computed` when you need a derived value; reach for `watch` when you
need to *do something* (log, fetch, sync to storage) in response to a change.

## Related

- [Reactivity fundamentals](./reactivity-fundamentals.md) — the box model these
  build on.
- [Watchers](./watchers.md) — side effects on change.
- Module-scope derived state: `derivedStore` in [scaling](../scaling/).
