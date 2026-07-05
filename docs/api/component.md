# Component API

The DOM construction and wiring helpers a compiled component uses: the factory
itself, positional refs, event wiring, class/style normalization, and child
mounting. See the [components guide](../components/registration.md) for the
conceptual model and the [architecture overview](../architecture.md) for why
Lunas builds via `innerHTML` and navigates positionally.

Import from the package root or the `lunas/dom` and `lunas/blocks` subpaths.

---

## `component`

### Signature

```ts
function component<P = Record<string, unknown>>(
  tag: string,
  attrs: RootAttrs,
  HTML: string,
  setup: SetupFn<P>
): ComponentFactory<P>

type RootAttrs = Record<string, string>
type SetupFn<P> = (c: Context<Element>, props: P) => void
type ComponentFactory<P> = (props?: P) => Element
```

### Description

The compiled-component factory. Given a root `tag`, static `attrs`, a static
`HTML` skeleton, and a `setup` function, it returns a **factory** that builds one
instance per call:

1. `document.createElement(tag)` and apply `attrs` via `setAttribute`.
2. `root.innerHTML = HTML` — one native, detached parse of the comment-free,
   whitespace-free static skeleton (dynamic seams are excluded and become runtime
   anchors).
3. Expose the reactive context on `root.__lunasCtx` (so a parent's
   [`mountChild`](#mountchild) can push reactive props into the child).
4. Run `setup(c, props)` — all wiring happens **off-DOM**.
5. Return `root`. The caller attaches it to the live DOM **once** (see
   [`attach`](./lifecycle.md#attach)).

`HTML` is hoisted at module scope so it is defined once and shared by every
instance (the string, not the DOM — each instance re-parses it, which is cheaper
than cloning).

For a multi-root template (more than one top-level node), the compiler emits
`fragment(attrs, HTML, setup)` instead — same contract, but no wrapper element:
the factory returns an `Array` of nodes carrying `__lunasCtx`, mountable and
movable as a unit. `fragment` is compiler-facing; you do not normally call it by
hand.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `tag` | `string` | The root element tag. |
| `attrs` | `RootAttrs` | Static attributes for the root. |
| `HTML` | `string` | The static skeleton (comment-free, whitespace-free). |
| `setup` | `SetupFn<P>` | Wires binds, refs, listeners, and child blocks. |

### Returns

`ComponentFactory<P>` — `(props?) => Element`.

### Example

```js
import { component, box, refs, on, bind } from "lunas";

const HTML = `<button>+1</button><span></span>`;

export default component("div", { class: "counter" }, HTML, (c, props) => {
  const count = box(c, 0, props.start ?? 0);
  const [btn, out] = refs(c.root, [[0], [1]]);
  on(btn, "click", () => { count.v++; });
  bind(c, [0], () => { out.textContent = String(count.v); });
});
```

### Notes

- The returned root is **detached**; call [`attach(root, host)`](./lifecycle.md#attach)
  to mount a top-level component and fire its `onMount` hooks.
- In practice you write `.lunas` files and the compiler emits this call.

---

## `refs`

### Signature

```ts
function refs(root: Node, paths: number[][]): Node[]
```

### Description

Positional navigation to the dynamic elements of a parsed tree. Each entry in
`paths` is a list of child indices from `root`; `refs` walks
`childNodes[i]` for each step and returns the resolved nodes in order. This
replaces `id` + `getElementById`: it is ~2× faster, works on a **detached** tree,
and needs no id bytes or cleanup.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `root` | `Node` | The root to navigate from (usually `c.root`). |
| `paths` | `number[][]` | One child-index path per target node. |

### Returns

`Node[]` — the resolved nodes, positionally aligned with `paths`.

### Example

```js
// root's child 0, and root's child 1's child 2:
const [btn, label] = refs(c.root, [[0], [1, 2]]);
```

### Notes

- Relies on the static HTML being whitespace-free so `childNodes` positions are
  stable — this is guaranteed by the compiler's output.

---

## `on`

### Signature

```ts
function on(el: EventTarget, ev: string, fn: EventListenerOrEventListenerObject): void
```

### Description

`addEventListener` shorthand. Because box setters notify dependents, an event
handler that mutates state needs no explicit write bookkeeping — mutating a box
inside the handler marks the right indices dirty on its own.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `el` | `EventTarget` | The element to listen on. |
| `ev` | `string` | The event name (`"click"`, `"input"`, …). |
| `fn` | `EventListener` | The handler. |

### Returns

`void`.

### Example

```js
on(btn, "click", () => { count.v++; });
```

### Notes

- See the [event handling guide](../guide/event-handling.md) and
  [forms & two-way binding](../guide/forms-and-two-way.md) for `::model`
  write-back.

---

## `normClass`

### Signature

```ts
function normClass(value: unknown): string
```

### Description

Flattens a `:class` binding into a space-separated class string, with Vue-parity
semantics: a string passes through (trimmed); an array is flattened recursively;
an object contributes each key whose value is truthy; other values stringify.
Falsy entries are dropped.

The compiler pairs this with `setClass(el, staticClass, value)`, which merges the
normalized dynamic value with the element's static `class` attribute and writes
the whole attribute (removing it when empty).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `value` | `string \| Record<string, boolean> \| Array \| unknown` | The class binding. |

### Returns

`string` — the normalized class string.

### Example

```js
normClass(["btn", { active: isOn, disabled: false }]);
// isOn === true  -> "btn active"
```

### Notes

- See the [class and style guide](../guide/class-and-style.md).

---

## `normStyle`

### Signature

```ts
function normStyle(value: unknown): string
```

### Description

Flattens a `:style` binding into a `prop: value;` string. A string passes through
(trimmed); an object maps camelCase keys to kebab-case CSS properties (custom
`--props` and already-kebab names pass through); arrays merge left-to-right (later
entries win). Null/false object values are skipped.

The compiler pairs this with `setStyle(el, staticStyle, value)`, which merges the
static `style` string with the normalized dynamic value.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `value` | `string \| Record<string, unknown> \| Array` | The style binding. |

### Returns

`string` — the normalized inline-style string.

### Example

```js
normStyle({ backgroundColor: "red", fontSize: "14px" });
// -> "background-color: red; font-size: 14px;"
```

### Notes

- See the [class and style guide](../guide/class-and-style.md).

---

## `mountChild`

### Signature

```ts
function mountChild<P = Record<string, unknown>>(
  c: Context,
  anchor: Node,
  childFactory: ChildFactory<P>,
  props?: P
): MountedChild

type ChildFactory<P> = (props?: P) => Node
interface MountedChild {
  root: Node;
  unmount(): void;
}
```

The runtime handle also carries `ctx` and `setProp(name, value)` (see below).

### Description

Instantiates a child component and inserts its root **before** the `anchor` (a
permanent text node marking the child's slot). `props` seeds the child once:
static props are plain values; reactive props are getter functions
(`{ p: () => expr }`) invoked once at construction to seed the child's reactive
prop box. The parent keeps a reactive prop live by calling the handle's
`setProp(name, value)` inside its own `bind` — that writes the child's
`_props[name]` box, so the child's own template binds react.

The two contexts stay **independent**: pushing a prop marks only the child dirty;
a child event marks only the child. `mountChild` also links `childCtx.parent = c`
(so [`provide`/`inject`](./lifecycle.md#provide) walk the chain) and registers the
child under `c._children` (so a parent attach/teardown recurses into it). If the
insertion point is already live, the child's queued `onMount` callbacks fire
immediately; otherwise a later ancestor `attach()` drains them. `unmount()` fires
the child's `onDestroy` exactly once and removes every node of the group (handling
multi-root fragment children).

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The parent context. |
| `anchor` | `Node` | The permanent text anchor the child mounts before. |
| `childFactory` | `ChildFactory<P>` | A `component(...)` (or `fragment(...)`) result. |
| `props` | `P` | Seed props: values for static, getters for reactive. |

### Returns

`MountedChild` — `{ root, ctx, setProp(name, value), unmount() }`.

### Example

```js
import { anchorBefore, mountChild, bind } from "lunas";
import Child from "./Child.mjs";

const anchor = anchorBefore(hostNode.childNodes[k]);
const ch = mountChild(c, anchor, Child, { label: () => title.v, tone: "muted" });
bind(c, [/* deps of title */ 0], () => ch.setProp("label", title.v));
```

### Notes

- Slot content rides in via a reserved `props.$slots` object — see
  [slots](./blocks-and-control-flow.md#slotblock).
- For lazy children see [`mountAsyncChild`](./async.md#mountasyncchild); for
  dynamic `:is` see the [dynamic-component helper](./blocks-and-control-flow.md#dynamicblock).
