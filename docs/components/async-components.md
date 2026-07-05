# Async components — lazy loading with `asyncComponent`

Large or rarely-used components don't need to ship in your initial bundle.
`asyncComponent` wraps a lazy loader (typically a dynamic `import()`) into a
mountable child factory: the component's code is fetched only when it first
mounts, an optional loading/error UI covers the wait, and the resolved module is
**cached** so later mounts are synchronous.

## The basics

Wrap a loader with `asyncComponent`; use the result anywhere a normal component
factory is expected:

```js
import { asyncComponent } from "lunas";

const HeavyChart = asyncComponent(() => import("./HeavyChart.lunas"));
```

The loader is a function returning a `Promise` of the module (or the factory
directly). `asyncComponent` accepts:

- an **ES module namespace** — it uses the `default` export;
- a **bare factory** — used directly;
- a `{ default }` wrapper object — it unwraps `default`.

So `() => import("./HeavyChart.lunas")` works out of the box: the module's default
export (the component factory) is what gets mounted.

## Options — loading, error, delay, timeout

`asyncComponent(loader, opts)` takes an options object:

```js
const HeavyChart = asyncComponent(() => import("./HeavyChart.lunas"), {
  loading: Spinner,     // childFactory shown while pending (only after `delay` ms)
  error:   LoadFailed,  // childFactory shown on rejection or timeout
  delay:   200,         // ms to wait before showing `loading` (default 200)
  timeout: 8000,        // ms after which a still-pending load is treated as an error
});
```

| Option | Default | Meaning |
|---|---|---|
| `loading` | none | a component factory shown **while pending**, but only after `delay` ms elapse — so a fast load never flashes a spinner. |
| `error` | none | a component factory shown on **rejection** or **timeout**. It receives an `error` prop (the rejection reason / timeout error) merged into its props. |
| `delay` | `200` | ms to wait before showing `loading`. `0` shows it immediately. |
| `timeout` | none | ms after which a still-pending load is treated as an error (shows `error`). |

These match Vue's async-component semantics: **loading only after `delay`**,
**error on reject/timeout**.

### Loading/flash behavior

- If the module resolves **before** `delay` elapses, the loading component is
  **never shown** — no flash for fast loads.
- On a **cache hit** (see below), the component mounts **synchronously** with no
  placeholder and no flash at all.
- The reveal is scheduled on the flush boundary, so a micro-tick resolve that
  races a batched update lands without flicker.

## Lazy loading and caching

Each `asyncComponent` wrapper holds a per-wrapper cache:

- The **first** mount triggers the loader (`import()`), showing loading/error UI
  as configured.
- Concurrent mounts of the same wrapper **share one in-flight load** — N
  simultaneous mounts issue one `import()`, not N.
- Once resolved, the factory is **cached**; every **subsequent** mount builds
  **synchronously** — no placeholder, no network, no flash.

So define the wrapper **once** (module scope) and reuse it, to get caching across
all its mounts:

```js
// Good: one wrapper, shared cache.
const HeavyChart = asyncComponent(() => import("./HeavyChart.lunas"));

// Avoid: a fresh wrapper per use defeats the cache.
// asyncComponent(() => import("./HeavyChart.lunas"))  // don't create inline per-mount
```

## Unmount safety

Every async mount owns a liveness token. If the child is **unmounted while its
load is still in flight** (e.g. the user navigates away, or an enclosing `:if`
toggles off), the token flips and a loader that settles afterwards writes no DOM
and touches no boundary — no late render into removed nodes. `timeout` similarly
short-circuits a late resolve.

## How it mounts — `mountAsyncChild`

An async component is mounted through `mountAsyncChild`, which follows the same
contract as `mountChild` but threads the component context so the child can
register with the nearest [Suspense](../built-ins/suspense.md) boundary and so its
`unmount()` cancels in-flight loads:

```js
import { mountAsyncChild } from "lunas";

const handle = mountAsyncChild(c, anchor, HeavyChart, props);
// handle.unmount() cancels any pending load and removes the subtree.
```

You normally don't call this by hand — the compiler emits it for an async
component tag. It is shown here so the runtime shape is documented.

## Integration with Suspense

Async components are designed to cooperate with a **Suspense boundary**. When an
async child mounts inside a `suspenseBlock`, it registers as a pending dependency
with the nearest boundary; the boundary shows its fallback until **every** async
descendant has settled, then reveals the content in one batch (so a
fully-synchronous subtree never flashes the fallback). This lets you coordinate
the loading state of several async children with a single fallback, instead of a
spinner per component.

For the boundary API, nesting rules, and examples, see
[../built-ins/suspense.md](../built-ins/suspense.md). The `loading` option on
`asyncComponent` and a Suspense fallback are complementary: `loading` is per
async component; Suspense coordinates a whole subtree.

## Combining with dynamic components

An `asyncComponent` factory is just a component factory, so it composes with
`<component :is="expr">` — you can switch to a lazily-loaded component. See
[dynamic-components.md](./dynamic-components.md).

## Gotchas

- **Create the wrapper once.** A wrapper made inline on every mount has an empty
  cache each time, forcing a reload. Hoist it to module scope.
- **`delay` guards against spinner flash.** With the default `200` ms, a
  sub-200 ms load shows no loading UI. Lower it (even to `0`) only if you *want*
  the loading component to appear immediately.
- **`error` gets an `error` prop.** The rejection reason (or timeout error) is
  merged into the error component's props under `error`.
- **Unmounting cancels in-flight loads.** A late-settling load after unmount is a
  no-op — safe, but it means a component that unmounts before its load finishes
  never renders.

## Related

- [dynamic-components.md](./dynamic-components.md) — switch to an async component
  at runtime.
- [../built-ins/suspense.md](../built-ins/suspense.md) — coordinate multiple async
  children with one fallback.
- [registration.md](./registration.md) — ordinary (eager) component registration.
- [../api/](../api/) — `asyncComponent`, `mountAsyncChild`, `suspenseBlock`.
