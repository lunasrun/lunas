# Blocks & Control Flow API

Control-flow blocks are anchored at **permanent empty text nodes** (never
comments — comments drop Blink off its fast-path parser). Each block collects the
binds created inside its content into a *scope* and drops that scope when the
content is removed, so removed content never receives updates and never leaks.

See the guides on [conditional rendering](../guide/conditional-rendering.md) and
[list rendering](../guide/list-rendering.md), and the built-ins on
[teleport](../built-ins/teleport.md), for the conceptual model.

Import from the package root or the `lunas/dom` and `lunas/blocks` subpaths.

---

## Anchor helpers

Anchors are created at wiring time (after the static parse) so the static HTML
stays comment-free. Each returns the created anchor text node.

### `anchorBefore`

```ts
function anchorBefore(node: Node): Text
```

Creates a permanent empty text-node anchor immediately **before** an existing
node.

### `anchorBeforeSplit`

```ts
function anchorBeforeSplit(textNode: Text, utf16Offset: number): Text
```

Splits a static text node at the given UTF-16 offset and places the anchor
between head and tail (the anchor sits **before** the tail). Used when a dynamic
seam falls inside a text run.

### `anchorAppend`

```ts
function anchorAppend(parent: Node): Text
```

Creates an anchor as the **last child** of `parent` (e.g. a `:for` slot at the
end of a container).

```js
import { anchorBefore, anchorAppend } from "lunas";
const ifAnchor  = anchorBefore(ul);      // an :if slot before <ul>
const forAnchor = anchorAppend(ul);      // a :for slot inside <ul>
```

---

## `ifBlock`

### Signature

```ts
function ifBlock(
  c: Context,
  anchor: Node,
  deps: number[],
  cond: () => boolean,
  make: () => BlockNodes
): BlockHandle

type BlockNodes = Node | Node[]
interface BlockHandle { destroy(): void }
```

The runtime handle also exposes `update()` (re-evaluate now; used by `:for` item
patching).

### Description

A single conditional block. When `cond()` becomes truthy, `make()` builds the
branch (via its own `innerHTML`) and inserts it before the permanent `anchor`;
when it becomes falsy, the branch's nodes are removed and its scope torn down.
`make()` may return a single node (single-root branch) or an array of nodes
(multi-root branch — the compiler knows which and emits accordingly).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor marking the slot. |
| `deps` | `number[]` | Reactive indices the condition reads. |
| `cond` | `() => boolean` | The condition. |
| `make` | `() => BlockNodes` | Builds the branch content. |

### Returns

`BlockHandle` — `{ update(), destroy() }`.

### Example

```js
ifBlock(c, anchor, [0], () => count.v > 0, () => {
  const el = fromHTML(`<p>positive</p>`, anchor).childNodes[0];
  return el;
});
```

### Notes

- For an `:if` / `:elseif` / `:else` cascade, the compiler emits a single
  [`ifChain`](#ifchain) rather than nested `ifBlock`s.

---

## `ifChain`

### Signature

```ts
function ifChain(
  c: Context,
  anchor: Node,
  deps: number[],
  which: () => number,
  makes: Array<() => BlockNodes>
): BlockHandle
```

### Description

One `:if` / `:elseif` / `:else` cascade at a single permanent anchor. `which()`
returns the index into `makes` of the branch to show, or `-1` for "no branch" (a
cascade without `:else` whose conditions are all false). Exactly one branch is
alive at a time; switching tears the old branch's scope down and builds the new
one via its own `make`.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor. |
| `deps` | `number[]` | Reactive indices the cascade's conditions read. |
| `which` | `() => number` | Index of the branch to show, or `-1`. |
| `makes` | `Array<() => BlockNodes>` | One builder per branch, in cascade order. |

### Returns

`BlockHandle` — `{ update(), destroy() }`.

### Example

```js
ifChain(c, anchor, [0], () => (n.v < 0 ? 0 : n.v > 0 ? 1 : 2), [
  () => neg(), () => pos(), () => zero(),
]);
```

### Notes

- `which()` is compiled from the cascade's conditions; the runtime never
  re-evaluates individual conditions itself.

---

## `forBlock`

### Signature

```ts
function forBlock<T = unknown>(
  c: Context,
  anchor: Node,
  deps: number[],
  items: () => T[],
  opts: ForBlockOpts<T>
): BlockHandle

interface ForBlockOpts<T> {
  make?(itemData: T, key: unknown, index: number): BlockNodes;
  html?: string;
  wire?(root: Node, itemData: T, index: number): ((d: T, i: number) => void) | void;
  keyOf?: (itemData: T, i: number) => Key;
  patch?(handle: unknown, itemData: T, i: number): void;
  onWarn?(message: string): void;
  seed?: ForSeed<T>;
}
```

### Description

A keyed list block. `items` is a closure returning the current array, read
**lazily at flush time**. The compiler picks one of two item-construction modes:

- **`make(itemData, key, index)`** — build one item, returning a node or node
  array.
- **Compiled mode (`html` + `wire`)** — `html` is the item's single-root static
  skeleton. The **initial render** concatenates it N times into **one bulk
  `innerHTML` parse**, then wires each item via `wire(root, d, i)`. On updates, a
  new item parses its own copy. `wire` may return a patch closure `(d, i) => …`
  that updates the item's data cell; after it runs, the item's whole scope is
  re-run so every item-local bind (including nested block binds) sees new data.

Updates go through the keyed [LIS reconciler](./reactivity.md) (`reconcile` from
`for_diff`): prefix/suffix trimming, a key→index map, and a
longest-increasing-subsequence pass minimize node moves. `innerHTML` is **never**
used on update. `opts.keyOf` supplies the `:key` extractor (falls back to item
identity/index); `opts.seed` skips the initial reconcile when items are already
mounted from an external bulk render.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor marking the list slot. |
| `deps` | `number[]` | Reactive indices the list expression reads. |
| `items` | `() => T[]` | Lazily read current array. |
| `opts` | `ForBlockOpts<T>` | Item construction, key, patch, warn, and seed hooks. |

### Returns

`BlockHandle` — `{ update(), destroy() }`.

### Example

```js
forBlock(c, anchor, [1], () => todos.v, {
  keyOf: (t) => t.id,
  html: `<li></li>`,
  wire(root, t) {
    root.textContent = t.text;
    return (d) => { root.textContent = d.text; }; // patch on data change
  },
});
```

### Notes

- The low-level reconciler (`createForState`, `seedForState`, `reconcile`,
  `longestIncreasingSubsequence`) is exported from `lunas/for_diff` for advanced
  use; `forBlock` wraps it with the DOM host.

---

## `dynamicBlock`

### Signature

```ts
function dynamicBlock(
  c: Context,
  anchor: Node,
  deps: number[],
  factoryOf: () => ChildFactory | null | undefined,
  props: Record<string, unknown>
): DynamicHandle

interface DynamicHandle {
  readonly handle: MountedChild | null;
  update(): void;
  setProp(name: string, value: unknown): void;
  destroy(): void;
}
```

### Description

The dynamic-component helper — the codegen target for `<component :is="e">`.
`factoryOf()` returns the current child factory (a `component(...)` result), or a
falsy value for "render nothing". Whenever the factory **identity** changes (its
deps flush), the old child is unmounted and the new one is mounted at the same
anchor via [`mountChild`](./component.md#mountchild), so prop passing and child
reactivity keep working. `props` is the same shape `mountChild` takes; it is
reused across remounts and re-seeds each fresh child. `setProp` forwards to the
live child.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor. |
| `deps` | `number[]` | Reactive indices `:is` reads. |
| `factoryOf` | `() => ChildFactory \| falsy` | Resolves the current child factory. |
| `props` | `object` | Seed props (getters for reactive, values for static). |

### Returns

`DynamicHandle` — `{ handle, update(), setProp(name, value), destroy() }`.

### Example

```js
dynamicBlock(c, anchor, [0], () => views[tab.v], { data: () => payload.v });
```

### Notes

- See the [dynamic components guide](../components/dynamic-components.md).

---

## `teleportBlock`

### Signature

```ts
function teleportBlock(
  c: Context,
  anchor: Node,
  targetOf: () => string | Element | null,
  build: () => BlockNodes
): TeleportHandle

interface TeleportHandle {
  nodes: Node[];
  destroy(): void;
}
```

### Description

Teleport/portal — the codegen target for `<teleport to="…">`. `build()` returns
the content node(s) (like an `:if` branch). `targetOf()` resolves the mount
target: a selector string (`querySelector`) or an `Element`. The content is
inserted into the target instead of inline at `anchor`; on `destroy` the nodes are
removed. A permanent text anchor still marks the inline slot, so surrounding
layout is undisturbed and teardown order stays deterministic. Content binds are
collected in a scope, so `destroy` tears down every inner bind — no leaks.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent inline text anchor. |
| `targetOf` | `() => string \| Element \| null` | Resolves the mount target. |
| `build` | `() => BlockNodes` | Builds the teleported content. |

### Returns

`TeleportHandle` — `{ nodes, destroy() }`.

### Example

```js
teleportBlock(c, anchor, () => "#modals", () => {
  return fromHTML(`<div class="modal">Hi</div>`, anchor).childNodes[0];
});
```

### Notes

- If the target cannot be resolved, the content is not inserted (never-panic).
- See the [teleport built-in](../built-ins/teleport.md).

---

## `slotBlock`

### Signature

```ts
function slotBlock(
  childCtx: Context,
  anchor: Node,
  factory: SlotFactory | null | undefined,
  fallback?: (slotProps?: unknown) => BlockNodes,
  slotPropsOf?: () => unknown
): { nodes: Node[] }

type SlotFactory = (slotProps: unknown, onCleanup: (fn: () => void) => void) => BlockNodes
```

### Description

Renders slot content at a `<slot>` anchor **inside a child component**. This is
the child half of the slot mechanism.

- `factory` — the parent-provided slot content factory (from the reserved
  `props.$slots` object), or absent when the parent passed no content for this
  slot. It is wired against the **parent** context (parent reactivity drives it);
  `onCleanup(fn)` ties its teardown to the child's `onDestroy`.
- `fallback` — the child's own fallback (`() => nodes`), wired in the **child**
  scope, shown only when `factory` is absent.
- `slotPropsOf` — optional getter returning scoped-slot props
  (`<slot :item="e"/>`) passed up to the parent content. Captured once at build.

Whichever content is used is inserted before `anchor`; null/undefined entries are
skipped defensively (never-panic).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `childCtx` | `Context` | The child component's context (where the `<slot>` lives). |
| `anchor` | `Node` | The permanent text anchor marking the slot position. |
| `factory` | `SlotFactory \| falsy` | Parent-provided content, or absent. |
| `fallback` | `(slotProps?) => BlockNodes` | The child's own fallback. |
| `slotPropsOf` | `() => unknown` | Scoped-slot props getter. |

### Returns

`{ nodes: Node[] }`.

### Notes

- Scoped-slot props are a **snapshot** captured at build time — the child later
  mutating a scoped value and re-rendering parent content is deferred (see the
  [slots guide](../components/slots.md)). Fixed-shape scoped props work today.

---

## `slotContent`

### Signature

```ts
function slotContent(
  parentCtx: Context,
  build: (slotProps: unknown) => BlockNodes,
  slotProps: unknown,
  onCleanup: (fn: () => void) => void
): BlockNodes
```

### Description

Builds the **parent** half of a slot factory. Per slot it fills, the parent emits
a factory of shape `(slotProps, onCleanup) => nodes`; this helper wraps the actual
wiring: it opens a fresh scope on the **parent** context (homed at the scope open
when the parent mounted the child, so nested `:for`-item slot content tears down
with the item), runs `build(slotProps)` to create and wire the content against the
parent, registers the scope's `dropScope` through `onCleanup`, and returns the
nodes.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `parentCtx` | `Context` | The parent component's context. |
| `build` | `(slotProps) => BlockNodes` | Wires the content against the parent. |
| `slotProps` | `unknown` | Scoped-slot props forwarded from the child. |
| `onCleanup` | `(fn) => void` | Registers teardown tied to the child's unmount. |

### Returns

`BlockNodes` — the produced nodes.

### Example (emitted parent side)

```js
mountChild(c, anchor, Child, {
  $slots: {
    default: (sp, onCleanup) =>
      slotContent(c, () => {
        const r = fromHTML(`<span></span>`, anchor).childNodes[0];
        bind(c, [0], () => { r.textContent = title.v; });
        return r;
      }, sp, onCleanup),
  },
});
```

### Notes

- See the [slots guide](../components/slots.md) for `<template #name>` and scoped
  slots.
