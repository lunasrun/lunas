# Async Components & Suspense API

Two cooperating primitives for lazy components and async boundaries,
dependency-free (ES2015 + `Promise`) and tree-shakeable. See the
[async components guide](../components/async-components.md) and the
[suspense built-in](../built-ins/suspense.md) for the conceptual model.

Import from the package root or the `lunas/async` subpath.

---

## `asyncComponent`

### Signature

```ts
function asyncComponent<P = Record<string, unknown>>(
  loader: AsyncLoader<P>,
  opts?: AsyncComponentOptions<P>
): ChildFactory<P>

type AsyncModule<P> = ChildFactory<P> | { default: ChildFactory<P> }
type AsyncLoader<P> = () => Promise<AsyncModule<P>> | AsyncModule<P>

interface AsyncComponentOptions<P> {
  loading?: ChildFactory<P>;                         // shown while pending, after `delay`
  error?: ChildFactory<P & { error?: unknown }>;     // shown on reject/timeout
  delay?: number;                                    // ms before showing loading (default 200)
  timeout?: number;                                  // ms after which a pending load errors
}
```

### Description

Wraps a lazy module loader — `() => import("./Heavy.mjs")` or similar — into a
mountable child factory obeying the [`mountChild`](./component.md#mountchild)
contract. The module is resolved on first mount (a `default` export or a bare
factory are both accepted) and **cached**, so later mounts build synchronously
with no placeholder.

Vue-style options: `loading` is shown only after `delay` ms (avoiding a flash for
fast loads); `error` is shown on rejection or after `timeout`, receiving
`{ error }` in props.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `loader` | `AsyncLoader<P>` | Returns a module/factory or a promise of one. |
| `opts` | `AsyncComponentOptions<P>` | `loading` / `error` / `delay` / `timeout`. |

### Returns

`ChildFactory<P>` — mount it with [`mountAsyncChild`](#mountasyncchild) (which
threads the context for suspense).

### Example

```js
import { asyncComponent } from "lunas";

const Heavy = asyncComponent(() => import("./Heavy.mjs"), {
  loading: Spinner,
  error: ErrorBox,
  delay: 200,
  timeout: 10000,
});
```

### Notes

- Mount with `mountAsyncChild`, not plain `mountChild`, so the load registers with
  the nearest [`suspenseBlock`](#suspenseblock).

---

## `mountAsyncChild`

### Signature

```ts
function mountAsyncChild<P = Record<string, unknown>>(
  c: Context,
  anchor: Node,
  asyncFactory: ChildFactory<P>,
  props?: P
): MountedChild
```

### Description

Mounts an async component factory (from [`asyncComponent`](#asynccomponent)) at a
text anchor. Same contract as [`mountChild`](./component.md#mountchild), but it
threads the component context so the async component can register with the nearest
[`suspenseBlock`](#suspenseblock). Its `unmount()` cancels any in-flight load, so
a late resolution writes no DOM.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor. |
| `asyncFactory` | `ChildFactory<P>` | An `asyncComponent(...)` result. |
| `props` | `P` | Seed props. |

### Returns

`MountedChild` — `{ root, unmount() }` (plus `ctx`/`setProp` at runtime, like
`mountChild`).

### Example

```js
import { anchorBefore, mountAsyncChild } from "lunas";
const a = anchorBefore(placeholder);
mountAsyncChild(c, a, Heavy);
```

### Notes

- Each async mount owns a live token; `unmount()` flips it so a loader that
  settles afterward writes no DOM and settles no boundary.

---

## `suspenseBlock`

### Signature

```ts
function suspenseBlock(
  c: Context,
  anchor: Node,
  contentFactory: (c: Context) => Node | Node[],
  fallbackFactory?: () => Node | Node[]
): SuspenseHandle

interface SuspenseHandle {
  isSettled(): boolean;
  destroy(): void;
}
```

### Description

An async boundary at a text anchor. It builds the content **immediately** (so
async children begin loading), but shows `fallback` until every async dep
registered under it resolves, then reveals the content. The reveal is batched via
[`afterFlush`](./reactivity.md#afterflush), so a fully synchronous subtree never
flashes the fallback. Nested boundaries handle their own subtree — each async
child registers with its nearest boundary and never leaks pending counts upward.

The boundary publishes itself on the context as `c._suspense` for the duration of
its content build; `mountAsyncChild` reads the *current* `c._suspense` and, if
present, registers with it (bumping the pending counter while loading, settling on
resolve/reject).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor. |
| `contentFactory` | `(c) => Node \| Node[]` | Builds the real content (async kids start loading here). |
| `fallbackFactory` | `() => Node \| Node[]` | Builds the fallback shown while pending. |

### Returns

`SuspenseHandle` — `{ isSettled(), destroy() }`. `isSettled()` reports whether the
boundary has revealed its content; `destroy()` tears the boundary down, cancelling
pending async children.

### Example

```js
import { anchorBefore, suspenseBlock, mountAsyncChild } from "lunas";

const a = anchorBefore(placeholder);
suspenseBlock(
  c, a,
  (bc) => {
    const host = document.createElement("div");
    const anchor = anchorAppend(host);
    mountAsyncChild(bc, anchor, Heavy);
    return host;
  },
  () => document.createTextNode("Loading…")
);
```

### Notes

- The mount entry for the whole app subtree is [`attach`](./lifecycle.md#attach);
  suspense boundaries reveal after their subtree's mount and settle passes.
