# Watchers

A watcher runs a **side effect** in response to reactive state changing — logging,
persisting to storage, kicking off a fetch, imperative DOM work. Where a
[computed](./computed.md) value is pull-based and produces a value, a watcher is
push-based and produces an effect.

Watchers are runtime helpers, `watch` and `watchEffect`, imported from `lunas`.

## `watch`

`watch(c, deps, cb, opts?)` runs `cb` **after** any of its dependencies changes.
By default the first (synchronous) run is suppressed, so the callback fires only
on subsequent changes:

```js
import { watch } from "lunas";

// runs when `count` changes (deps are the reactive indices it reads)
watch(c, [countIdx], () => {
  console.log("count is now", count.v);
});
```

### `immediate`

Pass `{ immediate: true }` to also invoke the callback once at registration time:

```js
watch(c, [countIdx], () => {
  localStorage.setItem("count", String(count.v));
}, { immediate: true });   // also persists the initial value
```

### Stopping and cleanup

`watch` returns a `stop()` handle that unregisters the watcher:

```js
const stop = watch(c, [countIdx], () => { /* … */ });
// later:
stop();
```

Watchers are also **scope-aware**: a watcher created inside a control-flow block
(a `:if` branch, a `:for` item) is torn down automatically when that content is
removed. You get cleanup for free in the common case, and `stop()` for explicit
control.

## `watchEffect`

`watchEffect(c, deps, fn)` runs `fn` **immediately** and again after any
dependency changes — with no distinction between the initial and later runs. It
is the effect-style shorthand for a watcher that always runs on registration:

```js
import { watchEffect } from "lunas";

watchEffect(c, [widthIdx, heightIdx], () => {
  canvas.width = width.v;
  canvas.height = height.v;
});
```

Like `watch`, it returns a `stop()` and is collected by the current scope.

> `watchEffect(c, deps, fn)` is equivalent to
> `watch(c, deps, fn, { immediate: true })` with the effect naming.

## When to use a watcher vs computed

| Use `computed` when… | Use `watch` / `watchEffect` when… |
|---|---|
| you need a **derived value** to render or read | you need to **do something** (I/O, logging, imperative DOM) |
| the result is pure and cacheable | the effect is impure or asynchronous |
| readers should pull lazily | you want to react eagerly to a change |

Rule of thumb: if the output is a value you display, reach for
[`computed`](./computed.md); if the output is an action, reach for a watcher.

## Timing

Watcher callbacks run as part of the reactive [flush](./reactivity-fundamentals.md)
after their dependencies change, so they observe the batched, post-write state.
Multiple writes in one tick coalesce, so a watcher runs once per flush, not once
per write.

## Related

- [Computed values](./computed.md).
- [Reactivity fundamentals](./reactivity-fundamentals.md) — the flush model.
- [Lifecycle](./lifecycle.md) — `onMount` / `onDestroy` for mount-tied effects.
