# Reactivity API

The reactive core of the Lunas runtime. These are the primitives the compiler
emits calls into — see the [reactivity fundamentals](../guide/reactivity-fundamentals.md),
[computed](../guide/computed.md), and [watchers](../guide/watchers.md) guides for
the conceptual model, and the [architecture overview](../architecture.md) for how
compile-time dependency dispatch works.

Every reactive helper is keyed on a **context** `c` (created internally by
`component(...)`, see [component API](./component.md)) and a numeric **reactive
index** `i`. The compiler assigns each reactive variable an index and computes,
for each dynamic part, the exact set of indices it reads (its `deps`). There is
no runtime dependency tracking.

Import from the package root or the `lunas/boxes`, `lunas/core`, `lunas/computed`,
`lunas/watch`, `lunas/batch` subpaths.

---

## `box`

### Signature

```ts
function box<T>(c: Context, i: number, v: T): Box<T>
```

`Box<T>` is `{ v: T }`.

### Description

Creates a **reassign-only** reactive cell at reactive index `i`. This is the
lightest reactive path: a plain getter/setter, no `Proxy`. Reading `.v` returns
the current value; assigning `.v` writes it and marks index `i` dirty, scheduling
a microtask flush. Same-value writes (`x !== v` fails) are no-ops.

The compiler chooses `box` for variables that are only ever reassigned
(`x = …`), never deeply mutated.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `i` | `number` | The reactive index of this variable. |
| `v` | `T` | The initial value. |

### Returns

`Box<T>` — a cell exposing a single `v` accessor.

### Example

```js
import { component, box, refs, bind, on } from "lunas";

component("div", {}, `<button>+1</button><span></span>`, (c) => {
  const count = box(c, 0, 0);
  const [btn, out] = refs(c.root, [[0], [1]]);
  on(btn, "click", () => { count.v++; });
  bind(c, [0], () => { out.textContent = String(count.v); });
});
```

### Notes

- Writing an equal value never re-runs dependents.
- For deep mutation of an array/object value, use [`deepBox`](#deepbox).

---

## `deepBox`

### Signature

```ts
function deepBox<T>(c: Context, i: number, v: T): Box<T>
```

### Description

Creates a **deeply-mutated** reactive cell at reactive index `i`. Reads through
`.v` return a `Proxy` that marks index `i` dirty on any nested `set` or `delete`
— including array mutators (`push`, `splice`, …) which run with the proxy as
`this`. Nested objects are wrapped lazily on property access; wrappers are cached
per underlying object (via a `WeakMap`) so proxy identity is stable across reads.
Replacing the whole value (`box.v = next`) also marks dirty and re-wraps.

The compiler chooses `deepBox` for variables it observes being deeply mutated
(`arr.push(...)`, `obj.k = …`).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `i` | `number` | The reactive index of this variable. |
| `v` | `T` | The initial value (typically an array or object). |

### Returns

`Box<T>` — a cell whose `.v` getter yields the deep proxy.

### Example

```js
const todos = deepBox(c, 1, []);
todos.v.push({ text: "buy milk" }); // marks index 1 dirty
todos.v[0].text = "buy oat milk";   // nested set — also marks dirty
```

### Notes

- Only the reactive floor `Proxy` (ES2015) is used; no `BigInt`.
- The Proxy handler is shared with module-level stores (see [store API](./store.md)).

---

## `shared`

### Signature

```ts
function shared<T>(v: T): Shared<T>

interface Shared<T> {
  v: T;
  attach(c: Context, i: number): void;
  detach(c: Context): void;
}
```

### Description

Creates a value **shared across components** — the classic "a prop is passed
down and mutated" case. Each dependent component `attach`es with its own context
and reactive index; a write to `.v` marks the variable dirty in **every** attached
component. Same-value writes are no-ops. `detach(c)` removes every attachment
belonging to context `c` (used on teardown).

For state that lives outside any component and is imported by many, prefer a
module-level store — see [`createStore`](./store.md#createstore), which
generalizes this concept.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `v` | `T` | The initial shared value. |

### Returns

`Shared<T>` — `{ v, attach(c, i), detach(c) }`.

### Example

```js
const theme = shared("light");
theme.attach(childCtx, 3);      // child index 3 tracks the theme
theme.v = "dark";               // marks childCtx index 3 dirty
```

### Notes

- `attach` does not deduplicate; detach with `detach(c)` per context.

---

## `computed`

### Signature

```ts
function computed<T>(c: Context, i: number, deps: number[], fn: () => T): Computed<T>
```

`Computed<T>` is `{ readonly v: T }`.

### Description

A **lazy, memoized** derived value at reactive index `i` reading the upstream
reactive indices in `deps`. `fn` runs only when the value is actually read
**and** an upstream dep has changed since the last computation. Reading `.v`
from inside a `bind` that declares `i` in its deps keeps the consumer reactive to
the computed's own index.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `i` | `number` | The reactive index this computed publishes on. |
| `deps` | `number[]` | Upstream reactive indices it reads. |
| `fn` | `() => T` | The compute function. |

### Returns

`Computed<T>` — a read-only box-shaped handle whose `.v` getter yields the
memoized result.

### Example

```js
const first = box(c, 0, "Ada");
const last  = box(c, 1, "Lovelace");
const full  = computed(c, 2, [0, 1], () => `${first.v} ${last.v}`);

bind(c, [2], () => { nameEl.textContent = full.v; });
```

### Notes

- Never eager: if nothing reads `.v`, `fn` never runs after invalidation.
- See the [computed guide](../guide/computed.md).

---

## `watch`

### Signature

```ts
function watch(c: Context, deps: number[], cb: () => void, opts?: WatchOpts): StopHandle

interface WatchOpts { immediate?: boolean }
type StopHandle = () => void
```

### Description

Runs `cb` after any of `deps` changes. By default the first (synchronous) run is
**suppressed**, so the callback fires only on subsequent changes. Pass
`{ immediate: true }` to also invoke it once at registration time. The watcher is
collected by the currently-open scope (so it is torn down by `dropScope` when its
enclosing control-flow block is removed) in addition to the returned `stop()`.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `deps` | `number[]` | Reactive indices to watch. |
| `cb` | `() => void` | Callback run after a change. |
| `opts` | `WatchOpts` | `{ immediate }` — also run once now (default `false`). |

### Returns

`StopHandle` — a function that unregisters the watcher.

### Example

```js
const stop = watch(c, [0], () => {
  console.log("count changed to", count.v);
});
// later: stop();
```

### Notes

- See the [watchers guide](../guide/watchers.md).

---

## `watchEffect`

### Signature

```ts
function watchEffect(c: Context, deps: number[], fn: () => void): StopHandle
```

### Description

Runs `fn` **immediately** and again after any of `deps` changes — no distinction
between the initial and later runs. Like `watch`, the returned `stop()` and the
open scope both tear it down.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `deps` | `number[]` | Reactive indices to watch. |
| `fn` | `() => void` | Effect run now and on every change. |

### Returns

`StopHandle`.

### Example

```js
watchEffect(c, [0], () => {
  document.title = `count: ${count.v}`;
});
```

### Notes

- Because Lunas resolves deps at compile time, you pass the index list explicitly
  rather than having them auto-tracked.

---

## `batch`

### Signature

```ts
function batch<T>(c: Context, fn: () => T): T
```

### Description

Runs `fn` (which may write many boxes) and **flushes synchronously afterward**,
collapsing the whole group of writes into a single update pass that has completed
by the time `batch()` returns. Nested batches on the same context flush only at
the outermost call. Returns whatever `fn` returns.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `fn` | `() => T` | A function performing one or more writes. |

### Returns

`T` — the return value of `fn`.

### Example

```js
batch(c, () => {
  a.v = 1;
  b.v = 2;
  c2.v = 3;
}); // one update pass, already applied to the DOM here
```

### Notes

- Without `batch`, multiple writes in the same tick already coalesce into one
  microtask flush; `batch` makes the flush **synchronous** and immediate.

---

## `nextTick`

### Signature

```ts
function nextTick(c: Context): Promise<void>
```

### Description

Returns a `Promise` resolved after the next flush completes (i.e. after the DOM
update pass). If nothing is pending, a flush is still scheduled, so
`await nextTick(c)` always lands after the current tick's update pass.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |

### Returns

`Promise<void>`.

### Example

```js
count.v++;
await nextTick(c);
// the DOM now reflects the new count
```

### Notes

- Built on [`afterFlush`](#afterflush).

---

## `afterFlush`

### Signature

```ts
function afterFlush(c: Context, cb: () => void): void
```

### Description

Runs `cb` once, after the next flush completes. If a flush is already pending the
callback rides that one; otherwise a flush is scheduled so the callback still
fires this tick. This is the primitive behind [`nextTick`](#nexttick) and the
reveal-after-settle behavior of [`suspenseBlock`](./async.md#suspenseblock).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `cb` | `() => void` | Callback run after the next flush. |

### Returns

`void`.

### Notes

- Prefer `nextTick` for a promise-shaped API; `afterFlush` is the callback form.

---

## Low-level core

These are the raw primitives the compiler and the higher-level helpers are built
on. You rarely call them by hand — [`box`](#box)/[`deepBox`](#deepbox) already
call `markVar`, and `component(...)` already calls `createContext`. Documented
here for completeness.

### `bind`

```ts
function bind(c: Context, deps: number[], fn: () => void): BindRecord
```

Registers an update function that reads the reactive indices in `deps`. Runs `fn`
**once immediately** (so the first paint is correct without a flush). Returns the
bind record (needed for [`unbind`](#unbind)). This is the workhorse behind every
reactive text node, attribute, and control-flow block.

### `markVar`

```ts
function markVar(c: Context, i: number): void
```

Signals that reactive variable `i` changed: enqueues its dependents (deduplicated)
and schedules a microtask [`flush`](#flush). **Low-level / internal** — the box
setters call this for you. Call it directly only when writing a custom reactive
cell.

### `flush`

```ts
function flush(c: Context): void
```

Runs every queued update once, then drains any callbacks registered via
[`afterFlush`](#afterflush). Only affected parts run (cost is O(affected)).
**Low-level / internal** — flushes are normally scheduled automatically on the
microtask queue by `markVar`; call `flush` directly only to force a synchronous
update pass (which is what [`batch`](#batch) does for you).

### `createContext`

```ts
function createContext<R = unknown>(root: R): Context<R>
```

Creates a fresh reactive context rooted at `root` (typically the component's root
DOM node). `component(...)` calls this; you rarely call it directly.

### `unbind`

```ts
function unbind(c: Context, s: BindRecord): void
```

Permanently unregisters a bind record. Safe to call while a flush containing `s`
is pending (the dead record is skipped at flush time).

### `beginScope` / `endScope` / `dropScope`

```ts
function beginScope(c: Context): Scope
function endScope(c: Context): void
function dropScope(c: Context, scope: Scope): void
```

Scope machinery for control-flow blocks. `beginScope` opens a collection scope
(nested under the currently-open one); every `bind` created until the matching
`endScope` is collected into it. `dropScope` unregisters every bind in the scope
recursively (including nested child scopes) and detaches it from its parent — the
mechanism by which removed `:if`/`:for` content never receives updates and never
leaks. See [blocks and control flow](./blocks-and-control-flow.md).
