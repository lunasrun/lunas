# Teleport

`<teleport>` renders a chunk of a component's template into a **different place in
the DOM** — typically outside the component's own subtree — while keeping it part
of the component logically. Its reactivity, event handlers, and teardown all
still belong to the component that declared it.

The classic use case is a modal, toast, or tooltip: you want the markup to live
next to the component that owns its state, but you need it mounted on
`document.body` (or some other container) so it escapes `overflow: hidden`,
`transform`, or `z-index` stacking contexts of its ancestors.

## Example

```html
<script>
  let open = false;
</script>

<button @click="open = true">Open dialog</button>

<teleport to="body">
  <div :if="open" class="modal-backdrop" @click="open = false">
    <div class="modal" @click="/* stop */">
      <h2>Hello from a teleport</h2>
      <button @click="open = false">Close</button>
    </div>
  </div>
</teleport>
```

Even though the `.modal-backdrop` markup is written inside the component, at
runtime it is appended into `<body>`. The `open` variable, the `@click`
handlers, and the `:if` block all keep working exactly as if the content were
inline.

## `to` — the target

`to` resolves to the mount target. It accepts either:

- a **CSS selector string** (`to="body"`, `to="#overlay-root"`,
  `to=".portals"`) — resolved with `document.querySelector(...)`; the **first**
  match wins, or
- an **Element** (a `:to="someEl"` binding whose value is a live DOM node).

```html
<!-- selector string -->
<teleport to="#toast-root"> … </teleport>

<!-- an element reference -->
<teleport :to="containerEl"> … </teleport>
```

If the target cannot be resolved (no element matches the selector, or the value
is `null`), the content is simply **not inserted** — nothing is thrown. The
component still mounts fine; the teleported content just has nowhere to go.

## How it works

Under the hood the compiler emits a call to the runtime's
[`teleportBlock`](../api/runtime.md):

```js
teleportBlock(c, anchor, targetOf, build);
```

- `build()` produces the content nodes (a single root or a multi-root group),
  wired against **this** component's reactive context.
- `targetOf()` resolves the target (selector string → `querySelector`, or an
  Element).
- The content is `appendChild`-ed into the target instead of inserted inline at
  the anchor.
- A permanent empty **text anchor** still marks the original inline slot, so the
  surrounding layout is undisturbed and teardown order stays deterministic.

The teleported content's reactive bindings are collected into a **scope** homed
where the block was created. That means destroying the block (or the owning
component) tears down every binding inside the teleported content — **no leaks**,
exactly like an [`:if`](../guide/control-flow.md) branch.

## Teardown

When the owning component unmounts (or the surrounding block is removed), the
teleport's `destroy()`:

1. removes every teleported node from the target, and
2. drops the content's reactive scope, so no late writes land after unmount.

You never manage this yourself — the runtime cleans up teleported DOM even
though it lives outside the component's own subtree.

## Gotchas

- **The target must already exist** when the teleport mounts. A selector like
  `to="#overlay-root"` only works if that element is in the document at mount
  time. For app-level portals, put the container in your `index.html`.
- **Selector resolution is one-shot at mount.** The runtime resolves the target
  when the content is built. If you need the target to change reactively, drive
  it with an element binding (`:to`) rather than expecting a string selector to
  re-query.
- **Styling still comes from where the markup is written**, not where it lands.
  Component-[scoped CSS](../scaling/scoped-css.md) attributes are stamped based
  on the declaring component, so scoped rules keep applying to teleported
  content.
- Content that is conditionally rendered inside a teleport (like the `:if` in the
  example) toggles normally — the teleport is the *destination*, the `:if` still
  controls *whether* anything is there.

## See also

- [Transition](./transition.md) — animate a teleported modal's enter/leave.
- [Keep-alive](./keep-alive.md) and [Suspense](./suspense.md) — the other
  control-flow built-ins.
- [Control flow](../guide/control-flow.md) — `:if` / `:for` basics used above.
- [Runtime API](../api/runtime.md) — the `teleportBlock` primitive.
