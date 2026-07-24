# Store API

Module-level reactive state living **outside** any component. A store is the
module-scope generalization of [`shared`](./reactivity.md#shared): created once at
module load and imported by however many components want it, with the same
adjacency-dispatch contract (no auto-tracking, no runtime dependency discovery).
Each field is independently subscribable — a write to one field only notifies
components that adopted *that* field.

See the [state management guide](../scaling/state-management.md) for the
conceptual model.

Import from the package root or the `lunas/store` subpath.

---

## `createStore`

### Signature

```ts
function createStore<T extends Record<string, any>>(initial: T): Store<T>

interface Store<T> {
  get<K extends keyof T>(key: K): T[K];
  set<K extends keyof T>(key: K, v: T[K]): void;
  subscribe<K extends keyof T>(key: K, fn: (value: T[K]) => void): Unsubscribe;
}

type Unsubscribe = () => void
```

### Description

Creates a module-level store from a plain object of named initial values. Each key
becomes an independent **field** (its own subscriber list). Object/array field
values get deep-mutation support with no `Proxy`, the same model as
[`deepBox`](./reactivity.md#deepbox): `get(key)` returns the raw value and a deep
mutation is made reactive by `store.touch(key)`, which the compiler injects after
the mutating statement. A value that is already field-shaped — e.g. the result of
[`derivedStore`](#derivedstore) — is kept as-is, so derived values can be declared
inline in the initial object.

- `get(key)` — current (raw) value.
- `touch(key)` — signal a deep mutation of `key`'s value (`store.get(key).push(x)`,
  `store.get(key).field = y`, …); notifies adopters and subscribers.
- `set(key, v)` — write, notifying every component that adopted `key` (batched per
  the normal microtask flush) and every `subscribe` listener (synchronously).
  Same-value writes are no-ops. Throws if `key` holds a derived (read-only) value.
- `subscribe(key, fn)` — outside-component subscription for plain-JS consumers
  (router, devtools, tests); `fn(value)` runs synchronously on every write to
  `key`. Returns an unsubscribe.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `initial` | `T` | Plain object of named initial values. |

### Returns

`Store<T>`.

### Example

```js
import { createStore } from "lunas";

export const appStore = createStore({ count: 0, user: null });

appStore.set("count", appStore.get("count") + 1);
const off = appStore.subscribe("count", (n) => console.log("count:", n));
```

### Notes

- Components adopt a field with [`useStore`](#usestore).
- There is also an internal `_field(key)` accessor used by `useStore`/
  `derivedStore`; prefer `get`/`set`/`subscribe` in application code.

---

## `useStore`

### Signature

```ts
function useStore<T extends Record<string, any>, K extends keyof T>(
  c: Context,
  i: number,
  store: Store<T>,
  key: K
): Unsubscribe
```

### Description

Adopts store field `key` at component context `c`'s reactive index `i`. From then
on, writes to `key` mark index `i` dirty in `c` (batched per the normal microtask
flush) — exactly like a compiler-emitted `shared(...).attach(c, i)` but sourced
from a module-level store. This is the shape the compiler emits: one call per
`(component, field)` adoption.

Returns a `detach()` that undoes the adoption (idempotent). When called while
`c.scope` is open (i.e. from inside a control-flow block's `make()`), the adoption
is **also** torn down automatically by `dropScope(c, scope)` — the same lifecycle
a plain `bind` gets.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `i` | `number` | The reactive index that tracks the field. |
| `store` | `Store<T>` | The store. |
| `key` | `keyof T` | The field to adopt. |

### Returns

`Unsubscribe` — the (idempotent) detach.

### Example

```js
import { useStore, bind } from "lunas";
import { appStore } from "./store.mjs";

useStore(c, 0, appStore, "count");
bind(c, [0], () => { out.textContent = String(appStore.get("count")); });
// handler:
on(btn, "click", () => appStore.set("count", appStore.get("count") + 1));
```

### Notes

- Adopted in a component's top-level setup (no open scope), the store keeps a live
  reference for that component's lifetime; call the returned `detach()` from your
  teardown path when needed.

---

## `subscribe`

`subscribe` is a method on the `Store` (see [`createStore`](#createstore)) and on
every field-shaped handle (including [`derivedStore`](#derivedstore) results). It
is the **outside-component** channel:

```ts
store.subscribe(key, fn) => Unsubscribe
```

`fn(value)` runs **synchronously** on every write to `key` (not batched — there is
no component flush to ride for a non-component listener). This is what the router
uses to observe its `"route"` field, and what devtools/tests use to observe store
state. It returns an unsubscribe function.

```js
const off = appStore.subscribe("user", (u) => render(u));
// later: off();
```

---

## `derivedStore`

### Signature

```ts
function derivedStore<T extends Record<string, any>, R>(
  store: Store<T>,
  deps: (keyof T)[],
  fn: () => R
): StoreField<R> & { stop(): void }

interface StoreField<R> {
  readonly v: R;
  attach(c: Context, i: number): void;
  detach(c: Context): void;
  subscribe(fn: (value: R) => void): Unsubscribe;
}
```

### Description

A read-only value derived from one or more fields of `store`, lazily recomputed
and memoized (like [`computed`](./reactivity.md#computed)) but living at module
scope. `deps` is the list of store keys it reads. The result is **field-shaped**:
place it under a `createStore()` key to let a component `useStore` it, or
`subscribe` to it directly from plain JS.

For component reads it stays lazy (recompute deferred to the next `.v` read after
a dep changes); for plain-JS subscribers it recomputes eagerly so `fn(value)`
hands over the fresh value synchronously. `stop()` unsubscribes from every
upstream field (rarely needed — derived stores are usually app-lifetime).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `store` | `Store<T>` | The upstream store. |
| `deps` | `(keyof T)[]` | The store keys it reads. |
| `fn` | `() => R` | The compute function. |

### Returns

`StoreField<R> & { stop(): void }`.

### Example

```js
import { createStore, derivedStore, useStore } from "lunas";

const cart = createStore({ items: [] });
const total = derivedStore(cart, ["items"], () =>
  cart.get("items").reduce((sum, it) => sum + it.price, 0)
);

const app = createStore({ total });   // field-shaped value passes through
useStore(c, 0, app, "total");         // component adopts the derived value
```

### Notes

- Because the derived result is field-shaped, `createStore` accepts it directly
  under a key without double-wrapping.
