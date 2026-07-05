# Template refs

`:ref` exposes a DOM element — or a mounted component instance — to your script,
so you can reach it imperatively (focus an input, measure a node, call a child's
method).

## Referencing an element

Declare a `let` for the ref and bind it with `:ref`:

```lunas
html:
    <div>
        <input :ref="field">
        <button @click="fill()">fill</button>
    </div>

script:
    let field
    function fill() {
        field.value = "typed"
        field.focus()
    }
```

At wire time, `field` is assigned the element. Because the ref is assigned to a
top-level variable, it becomes reactive — the compiler numbers it like any other
reactive variable, so template parts that read it react if it changes.

### How it compiles

`:ref="field"` assigns the element into the ref's box during wiring:

```js
field.v = e0;   // `field` is a reactive box; the ref makes it reactive
```

Declare the variable with a bare `let field` (no initializer); the `:ref`
assignment is what makes it reactive, so the resolver numbers it.

## Referencing a component

`:ref` on a component tag exposes the child's **mount handle** rather than a DOM
node:

```lunas
html:
    <Child :ref="childHandle" />

script:
    let childHandle
    // childHandle exposes the mounted child instance (its handle)
```

Through the handle you can drive the child imperatively (for example pushing a
prop or unmounting it), per the [component](../components/) mount contract.

## When the ref is available

The ref is assigned during **wiring**, before the component attaches to the live
DOM. If you need the element to be *in the document* (for layout measurement,
focus, or scroll), read the ref inside [`onMount`](./lifecycle.md):

```js
import { onMount } from "lunas";

let field;
onMount(c, () => { field.focus(); });   // element is live here
```

Reading a ref at the top level of `script:` (during setup) is too early — the
node exists but is not yet attached.

## Notes

- Give each ref its own variable; a ref inside a [`:for`](./list-rendering.md)
  item is per-item and follows the item's lifetime.
- A ref to an element in an [`:if`](./conditional-rendering.md) branch is only
  meaningful while the branch is shown; when the branch is hidden the node is
  removed.

## Related

- [Lifecycle](./lifecycle.md) — when the element is live.
- [Event handling](./event-handling.md) — often you can avoid a ref by handling
  events declaratively.
- [Components](../components/) — component instance handles.
