# State management

For state that outlives any single component and is shared across many, Lunas
provides **stores**: module-level reactive state that any number of components
can import and adopt. A store is the module-scope generalization of a shared prop
— instead of one value passed *down* the tree, it's created once at module load
and imported by however many components want it.

Stores keep Lunas's compile-time reactivity model: **no auto-tracking, no
runtime dependency discovery**. Each store field is independently subscribable, so
writing one field only marks dirty the components that adopted *that* field.

## Creating a store

```js
// store.mjs
import { createStore } from "lunas";

export const appStore = createStore({
  count: 0,
  user: null,
});
```

Each key becomes an **independent field** with its own subscriber list. A write
to `count` never notifies a component that only reads `user`.

## Reading and writing

From anywhere — a component handler or plain module code — read and write by key:

```js
appStore.get("count");                             // read
appStore.set("count", appStore.get("count") + 1);  // write
```

- Object/array values are **deep-mutation reactive**: they're lazily wrapped in a
  Proxy, so mutating a nested property notifies subscribers (same strategy as the
  component-local `deepBox`).
- **Same-value writes are no-ops** (`set(k, v)` where `v` is unchanged notifies
  nobody).

## Adopting a store in a component

When a component reads a store field in its template or handlers, the compiler
emits a `useStore(c, i, store, key)` call that adopts the field at the
component's reactive index `i`. From then on, writes to that field re-render the
parts of the component that read it:

```html
<script>
  import { appStore } from "./store.mjs";
</script>

<p>Count: {appStore.get("count")}</p>
<button @click="appStore.set('count', appStore.get('count') + 1)">
  Increment
</button>
```

Every component that adopts `count` re-renders on a write — that's
**cross-component reactivity** with no prop drilling and no event bus. Two sibling
components both reading `appStore.get("count")` stay in sync automatically.

`useStore` is scope-aware: adopted inside a control-flow block, its adoption is
torn down automatically when the block's content is removed (the same lifecycle a
`bind` gets), so a torn-down piece of UI stops being notified.

## Subscribing from plain JS

Outside a component context (tests, devtools, glue code), subscribe directly:

```js
const stop = appStore.subscribe("count", (value) => {
  console.log("count is now", value);
});
// …later
stop();
```

`fn(value)` runs **synchronously** on every write to that key (there is no
component flush to batch onto for a non-component listener). The
[router](./routing.md) is built on exactly this: its current route is a store
field named `route`, and the outlet subscribes to it.

## Derived values: `derivedStore`

`derivedStore(store, deps, fn)` is a read-only value computed from one or more
store fields, lazily recomputed and memoized (like a component's `computed`) but
living at module scope. `deps` is the list of store keys it reads:

```js
import { createStore, derivedStore } from "lunas";

const cart = createStore({ items: [] });

const total = derivedStore(cart, ["items"], () =>
  cart.get("items").reduce((sum, it) => sum + it.price, 0)
);
```

A derived value is **field-shaped**, so you can place it under a `createStore`
key and let components adopt it like any other field:

```js
const app = createStore({ total }); // field-shaped value passes through
// inside a component the compiler emits:
//   useStore(c, i, app, "total");
//   app.get("total")   // read the derived value
```

- Recomputation is **lazy for component reads** (each component reads on its own
  schedule) but **eager for plain-JS subscribers** (a `subscribe` listener is
  handed the fresh value synchronously, like every other store subscribe).
- `derivedStore` returns a handle with a `stop()` to unsubscribe from its
  upstream fields — rarely needed, since derived stores are normally
  module-scoped for the app's lifetime.

## When to use a store — vs props / provide

Lunas gives you three ways to share state; pick by scope and direction:

| Mechanism | Best for | Direction |
|---|---|---|
| **Props** (`@input`) | passing data to a direct child | parent → child, explicit |
| **provide / inject** | data for a whole subtree without threading props | ancestor → descendants |
| **Store** | app-wide state shared by unrelated components | any → any, no tree relationship |

Rules of thumb:

- **Reach for props first.** Direct parent-to-child data is clearest as a prop.
  Don't put something in a store just to hand it to one child.
- **Use provide/inject** when a value is needed deep in a subtree and threading
  it through every intermediate component as props would be noise — but the
  consumers are all *descendants* of the provider.
- **Use a store** when the sharers have **no tree relationship** (a header and a
  cart page), when the state must **outlive** the components that use it, or when
  many independent components need to react to the same source of truth.

## Gotchas

- **Reading `get(key)` in a handler doesn't declare a dependency** — adoption
  (the compiler-emitted `useStore`) is what wires reactivity. Read a store field
  in your template/reactive expressions so the compiler adopts it; a bare read in
  imperative code just fetches the current value.
- **Field independence is per key.** If you want a write to notify a set of
  components together, they must all adopt the same key. Splitting related state
  across keys means each key notifies only its own adopters.
- **Derived fields are read-only** — `set` on a store key holding a derived value
  is not a valid write. Change the upstream fields instead.
- **Deep mutation is reactive, but reassign the top level for clarity** when it's
  a small value: `set("count", n)` is simpler to reason about than mutating.

## See also

- [Routing](./routing.md) — a router is a store with one `route` field.
- [Component props & provide/inject](../components/props.md) — the other sharing
  mechanisms.
- [Runtime API](../api/runtime.md) — `createStore`, `useStore`, `derivedStore`.
