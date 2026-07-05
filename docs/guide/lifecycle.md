# Lifecycle

Lifecycle hooks let you run code at defined moments in a component's life:
after it mounts, when it is destroyed, and after each update. They are runtime
helpers imported from `lunas`.

## The hooks

```js
import { onMount, onDestroy, onUpdate } from "lunas";
```

- **`onMount(c, fn)`** — runs `fn` after the component's root is attached to a
  live tree. Fires once.
- **`onDestroy(c, fn)`** — runs `fn` when the component is torn down (unmounted).
  Fires exactly once.
- **`onUpdate(c, fn)`** — runs `fn` after each [flush](./reactivity-fundamentals.md)
  of this component that actually ran updates.

Two more exist for [keep-alive](../built-ins/) cached components:
`onActivated(c, fn)` and `onDeactivated(c, fn)` fire when a cached instance is
re-activated or deactivated (rather than destroyed).

## When onMount fires — the attach contract

This is the key detail. A component factory returns a **detached** root; the
*caller* attaches it (that's what [`attach`](./quick-start.md) does at the app
root, and what mounting a child does internally). So `onMount` cannot fire at
construction — it is queued and drained the moment the root becomes part of a
live tree:

```js
import { attach } from "lunas";
import App from "./App.lunas";

attach(App(), document.getElementById("app"));
// ↑ appends the root AND fires the whole subtree's onMount callbacks
```

A single top-level `attach` fires `onMount` for the root **and every child**
mounted during setup, because children register with their parent. If you
register `onMount` after the component is already live, `fn` still runs (on the
next microtask), so you always observe a mounted, painted tree.

Because of this, `onMount` is the right place for anything that needs the element
to be **in the document**: focus, layout measurement, scroll position, or
starting a subscription that touches the DOM.

```js
import { onMount, onDestroy } from "lunas";

let field;                         // a :ref
onMount(c, () => { field.focus(); });
```

## Cleanup patterns

Pair every subscription, timer, or listener started in `onMount` with teardown in
`onDestroy`. `onDestroy` fires on **every** unmount path — a child unmounting, a
`:if` branch hiding, a `:for` item leaving, or keep-alive eviction — and fires
exactly once:

```js
import { onMount, onDestroy } from "lunas";

let id;
onMount(c, () => { id = setInterval(tick, 1000); });
onDestroy(c, () => { clearInterval(id); });
```

For effects that respond to reactive changes rather than mount timing, prefer a
[watcher](./watchers.md) — watchers created in a control-flow block are torn down
automatically with that block.

## onUpdate

`onUpdate` runs after any flush that ran updates for this component. Use it
sparingly — for syncing to something the DOM update implies (e.g. reflecting
final layout after content changed). It does **not** run on the initial mount
(that's what `onMount` is for), only after subsequent update passes.

```js
import { onUpdate } from "lunas";
onUpdate(c, () => { /* runs after each update flush */ });
```

## Ordering summary

1. Component setup runs (script executes, bindings wired) — off-DOM.
2. Caller attaches the root → `onMount` fires (children first, then parent).
3. State changes → flush → DOM updated → `onUpdate` fires.
4. Component unmounted → `onDestroy` fires once.

## Related

- [Template refs](./template-refs.md) — read refs inside `onMount`.
- [Watchers](./watchers.md) — change-driven effects with automatic teardown.
- [Reactivity fundamentals](./reactivity-fundamentals.md) — the flush that drives
  `onUpdate`.
- Keep-alive activation hooks: [built-ins](../built-ins/).
