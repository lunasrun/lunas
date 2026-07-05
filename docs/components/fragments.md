# Fragments — multi-root components

A component doesn't have to have a single wrapper element. When a template has
**more than one top-level node**, Lunas compiles it as a **fragment**: the
component renders its several roots directly, with **no wrapper element** added
around them. This keeps your DOM clean — no stray `<div>` inserted just to
satisfy a "single root" rule.

## The basics

Write however many top-level nodes you need in the `html:` section:

```lunas
html:
    <h1 @click="rename()">${title}</h1>
    <p :if="show">visible ${title}</p>
    <footer>end</footer>

script:
    let title = "T"
    let show  = true
    function rename() {
        title = "T2"
        show  = !show
    }
```

This component has **three** top-level nodes (`<h1>`, `<p>`, `<footer>`). There is
no enclosing element — mounted into a parent, it contributes those three siblings
directly, not a wrapper containing them.

Contrast a **single-root** component, which has exactly one top-level element:

```lunas
html:
    <div class="counter">
        <span>${value}</span>
        <button @click="inc()">+</button>
    </div>
```

The compiler picks single-root vs. fragment automatically based on how many
top-level nodes the template has — you don't declare it.

## How it compiles

A single-root component compiles to a `component(tag, attrs, HTML, setup)`
factory whose root is the wrapper element. A multi-root component compiles to a
`fragment(attrs, HTML, setup)` factory instead:

```js
// single-root:
export default component("div", { class: "counter" }, HTML, setup);

// multi-root (fragment): NO wrapper tag
export default fragment({}, HTML, setup);
```

At runtime, `fragment`:

1. bulk-parses the static skeleton via `innerHTML` into a **throwaway host**;
2. runs `setup` while the nodes are still attached to that host (so positional
   `refs` navigate the parsed tree, exactly like a single-root component);
3. **snapshots the host's child nodes** (after setup, so any top-level anchors
   created during wiring travel with the group), detaches them from the host, and
   returns them as an **Array of nodes** carrying the component context on
   `__lunasCtx`.

That Array *is* the mountable unit. Because it carries `__lunasCtx`, a parent's
`mountChild` drives its props exactly as it would for a single-root child — a
fragment child is used identically to any other child:

```lunas
@use MultiRoot from "./MultiRoot.lunas"
html:
    <MultiRoot :title="heading"/>
```

## Mounting and teardown

The block helpers (`mountChild`, `ifBlock`, `forBlock`, …) already treat a node
group as a unit — they iterate the whole array to insert, move, or remove it. So a
fragment:

- **mounts** by inserting *all* its nodes before the anchor;
- **moves** as a unit (e.g. reordered inside a `:for`);
- **unmounts** by removing *every* node of the group and firing its `onDestroy`
  once.

No wrapper means no single element to move — the runtime tracks the node set
instead. This is transparent to you: a fragment child behaves like any child from
the outside.

> Under the hood, the context uses the **first** node as its `root` for
> positional refs against the whole child list. You don't interact with this
> directly; it's why `refs`, `:ref`, and event wiring work the same as in a
> single-root component.

## Use with control flow

Control-flow branches and list items can themselves be multi-root — the same
node-group machinery applies:

- **`:if` with a multi-node branch** — when several top-level nodes live inside a
  branch, the branch is tracked as a group (or delimited by a start/end anchor
  pair) so toggling the condition inserts/removes the whole group.
- **`:for` items with several nodes** — a multi-node item is likewise tracked as a
  group, so reordering/removing moves/removes all of the item's nodes together.

The compiler picks the cheap single-node path when a branch or item is
single-root, and the group path when it's multi-root — you just write the markup.

A fragment component *inside* an `:if` or `:for` composes naturally: its node
group is nested inside the block's node group, and teardown recurses.

## When to reach for a fragment

- **Table rows / list items** that must be direct children of a `<table>` /
  `<ul>` — a wrapper `<div>` would be invalid HTML there. A fragment component can
  return `<tr>`-level or `<li>`-level siblings directly.
- **Layout siblings** where an extra wrapper would break flex/grid layout.
- **Anywhere the wrapper element would be semantically meaningless.** If you'd
  otherwise add a `<div>` "just because", a fragment avoids it.

Prefer a single root when there *is* a natural container element — it's the
fastest common path (one element, one attach). Reach for a fragment when the
wrapper would be noise or invalid.

## Gotchas

- **Fragment vs. single-root is automatic.** It's decided by the number of
  top-level nodes in the template, not a flag. Two top-level nodes → fragment.
- **`attrs` on a fragment are ignored.** A fragment has no single root to attach
  attributes to; the `fragment(attrs, …)` signature accepts `attrs` for parity
  but does not apply them. Put attributes on the individual roots instead.
- **A fragment child is used like any child.** `@use` it, mount it, pass props and
  slots — nothing special at the call site.
- **The group moves/removes together.** You can't detach one root of a fragment
  independently; the runtime treats the set as one unit.

## Related

- [registration.md](./registration.md) — `@use` and how a component's default
  export (a `fragment(...)` factory here) is imported.
- [props.md](./props.md) — props work identically on a fragment child.
- [../built-ins/](../built-ins/) — `:if` / `:for` control flow that hosts
  multi-root branches and items.
- [../api/](../api/) — the `fragment` and `component` runtime factories.
