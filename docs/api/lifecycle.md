# Lifecycle, Events, DI, Transitions & Keep-Alive API

Component lifecycle hooks and the `attach` contract, child→parent events
(`emit`), dependency injection (`provide`/`inject`), CSS transitions, and instance
caching (`keepAlive`). See the [lifecycle guide](../guide/lifecycle.md),
[component events](../components/events.md), [provide/inject](../components/provide-inject.md),
and the [transition](../built-ins/transition.md) / [keep-alive](../built-ins/keep-alive.md)
built-ins for the conceptual model.

Import from the package root or the `lunas/lifecycle`, `lunas/emits`,
`lunas/provide`, `lunas/transition`, `lunas/keepalive` subpaths.

---

## Lifecycle hooks

A `component(...)` factory returns a **detached** root; the caller attaches it, so
`onMount` cannot fire at construction — it is queued on the context and drained
when the root becomes live. `mountChild` links `childCtx.parent = c` and registers
the child under `c._children`, so a single top-level [`attach`](#attach) fires the
whole subtree's mount hooks, and a parent teardown recurses into children.

### `onMount`

```ts
function onMount(c: Context, fn: () => void): void
```

Runs `fn` after this component's root attaches to a live tree. If the component is
already mounted, `fn` runs on the next microtask. Fires once.

### `onDestroy`

```ts
function onDestroy(c: Context, fn: () => void): void
```

Runs `fn` when this component is torn down — fires once, on every unmount path
(`mountChild.unmount`, block item teardown, keep-alive eviction), all of which
funnel through `runDestroy(c)`.

### `onUpdate`

```ts
function onUpdate(c: Context, fn: () => void): void
```

Runs `fn` after each flush of `c` that actually ran updates (the core flush loop
invokes `c.onUpdate` when the queue was non-empty).

### `onActivated` / `onDeactivated`

```ts
function onActivated(c: Context, fn: () => void): void
function onDeactivated(c: Context, fn: () => void): void
```

Keep-alive hooks. `onActivated` runs each time the component is (re)activated from
the cache (including the first activation); `onDeactivated` runs each time it is
deactivated (cached, not destroyed). See [`keepAlive`](#keepalive).

### Example

```js
import { onMount, onDestroy } from "lunas";

onMount(c, () => { console.log("mounted"); });
onDestroy(c, () => { clearInterval(timer); });
```

---

## `attach`

### Signature

```ts
function attach<N extends Node>(root: N, host: Node): N
```

### Description

Appends a detached component `root` to a live `host` and fires the whole subtree's
queued `onMount` callbacks. This is the **top-level mount entry**: build a root
with a `component(...)` factory, then `attach(root, document.body)`. Returns
`root`.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `root` | `Node` | The detached component root. |
| `host` | `Node` | A live host to append into. |

### Returns

`N` — the `root` (for chaining).

### Example

```js
import { attach } from "lunas";
import App from "./App.mjs";

attach(App(), document.getElementById("app"));
```

### Notes

- `isLive(node)` reports whether a node is attached to a live tree (uses
  `Node.isConnected` with a walk-to-root fallback for shims).

---

## `emit`

### Signature

```ts
function emit(c: Context, name: string, payload?: unknown): boolean
```

### Description

Raises a child→parent event: invokes the parent's `on<Name>` handler prop if
present, passing `payload`. Returns `true` if a handler ran. `emit` never marks
the parent dirty by itself — the handler decides whether to mutate parent state (a
box setter inside the handler marks the parent as usual).

For `emit` to find handlers, the child must first call `registerEmits(c, props,
declared?)` at the top of its `setup` (the compiler emits this), which stashes the
child's props and optionally records declared event names for warn-only
validation.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The child component's context. |
| `name` | `string` | The event name (e.g. `"save"`). |
| `payload` | `unknown` | Optional payload passed to the handler. |

### Returns

`boolean` — whether a handler ran.

### Example

```js
import { registerEmits, emit } from "lunas";

registerEmits(c, props, ["save"]);           // top of child setup
on(btn, "click", () => emit(c, "save", { id })); // raises parent's onSave
```

### Notes

- `eventPropName(name)` maps an event name to its prop: `eventPropName("save")` →
  `"onSave"`, `eventPropName("save-all")` → `"onSaveAll"`. This is the codegen's
  `@name` → `onName` mapping for component-tag event listeners.
- A `@save="h($event)"` listener on a component tag compiles to an
  `onSave: ($event) => h($event)` entry on the `mountChild` props.

---

## `provide`

### Signature

```ts
function provide<T>(c: Context, key: InjectionKey, value: T): T
type InjectionKey = string | symbol
```

### Description

Registers `key → value` on this component's context (a Map on `c._provides`),
making it available to descendants. Returns `value`. DI walks the
`childCtx.parent` chain that [`mountChild`](./component.md#mountchild) links;
nearest ancestor wins (shadowing).

### Example

```js
import { provide } from "lunas";
provide(c, "theme", "dark");
```

---

## `inject`

### Signature

```ts
function inject<T = unknown>(c: Context, key: InjectionKey, def?: T): T | undefined
```

### Description

Resolves `key` from the nearest ancestor that provided it (self included), else
returns `def` (default `undefined`).

### Example

```js
import { inject } from "lunas";
const theme = inject(c, "theme", "light");
```

---

## `hasInjection`

### Signature

```ts
function hasInjection(c: Context, key: InjectionKey): boolean
```

### Description

Whether any ancestor (or self) provides `key`. Lets a caller distinguish "provided
`undefined`" from "not provided".

```js
if (hasInjection(c, "theme")) { /* … */ }
```

---

## `withTransition`

### Signature

```ts
function withTransition(opts?: TransitionOptions): TransitionController

interface TransitionOptions {
  name?: string;       // class base name (default "v"); classes are `name-enter-from` etc.
  duration?: number;   // fallback timeout (ms) if no transitionend fires (0 → next macrotask)
}
interface TransitionController {
  enter(nodes: Node | Node[], insert: () => void): void;
  leave(nodes: Node | Node[], remove: () => void): void;
}
```

### Description

Builds an enter/leave CSS-class transition controller that composes with a block's
insert/remove closures. `enter(nodes, insert)` inserts the nodes then choreographs
the enter classes; `leave(nodes, remove)` choreographs the leave classes, then
calls `remove()` once every node's leave phase finishes. The class choreography is:

```
enter:  +name-enter-from +name-enter-active
        → (raf) −name-enter-from +name-enter-to
        → (transitionend/timeout) −name-enter-active −name-enter-to
leave:  symmetric; the node is removed only after the leave phase finishes.
```

Degrades to immediate insert/remove (with the class sequence still applied
synchronously) outside a browser (no `requestAnimationFrame`).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `opts` | `TransitionOptions` | `{ name, duration }`. |

### Returns

`TransitionController` — `{ enter, leave }`.

### Example

```js
import { withTransition } from "lunas";
const t = withTransition({ name: "fade", duration: 300 });
t.enter(node, () => anchor.parentNode.insertBefore(node, anchor));
```

### Notes

- The lower-level `runPhase(el, base, phase, opts, done)` runs one enter/leave
  class phase on a single element and returns a `cancel()`; `withTransition` builds
  on it.
- See the [transition built-in](../built-ins/transition.md).

---

## `keepAlive`

### Signature

```ts
function keepAlive(opts?: KeepAliveOptions): KeepAliveController

interface KeepAliveOptions { max?: number }
interface KeepAliveController {
  show<P>(c: Context, anchor: Node, key: unknown, factory: ChildFactory<P>, props?: P): KeptChild;
  has(key: unknown): boolean;
  readonly size: number;
  destroy(): void;
}
interface KeptChild extends MountedChild { ctx?: Context; key?: unknown }
```

### Description

Caches [`mountChild`](./component.md#mountchild)-produced instances by key instead
of destroying them on switch. `show(c, anchor, key, factory, props?)` makes the
instance for `key` the one mounted before `anchor`: it activates a cached instance
or mounts a fresh one, and deactivates the previously-shown instance.
Deactivation **detaches** nodes (keeping the context and reactive state alive);
activation **re-attaches** with no rebuild.

`max` sets an LRU capacity (unbounded when omitted); overflow evicts the
least-recently-shown instance. Real eviction (LRU overflow or `destroy()`) fires
`onDestroy`; activate/deactivate fire [`onActivated`/`onDeactivated`](#onactivated--ondeactivated).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `opts` | `KeepAliveOptions` | `{ max }` — LRU capacity. |

### Returns

`KeepAliveController` — `{ show, has(key), size, destroy() }`.

### Example

```js
import { keepAlive, anchorBefore } from "lunas";

const ka = keepAlive({ max: 3 });
const anchor = anchorBefore(placeholder);
// switching tabs keeps each tab's component state alive:
ka.show(c, anchor, tab.v, views[tab.v]);
```

### Notes

- Only eviction/`destroy()` fires `onDestroy`; a plain switch-away
  deactivates (state preserved).
- See the [keep-alive built-in](../built-ins/keep-alive.md).
