# Dynamic components — `<component :is="expr">`

Sometimes *which* component to render is a runtime decision — a tab that swaps
between panels, a form that switches editors by field type. `<component
:is="expr">` renders whatever component factory `expr` currently evaluates to,
and **swaps** to a new one whenever `expr` changes. It is the Lunas equivalent of
Vue's `<component :is>`.

## The basics

`:is` takes an expression that evaluates to a component factory — usually a
`@use`d component held in a reactive variable:

```lunas
@use Panel  from "./Panel.lunas"
@use Notice from "./Notice.lunas"

html:
    <div>
        <component :is="view" :label="title"/>
        <button @click="swap()">swap</button>
    </div>

script:
    let view  = Panel        <!-- start with Panel -->
    let title = "hi"
    function swap() { view = Notice }   <!-- switch to Notice -->
```

- `view` holds a component factory (`Panel`, imported via `@use`).
- `<component :is="view">` renders `Panel`.
- Clicking **swap** sets `view = Notice`; the dynamic slot unmounts `Panel` and
  mounts `Notice` in its place.

## `@use` and dynamic components

A `<component :is>` can select *any* component you might switch to, but the
compiler cannot know statically which one. So when a `<component :is>` tag is
present in a template, **all** the file's `@use` factories are emitted as
imports — none are tree-shaken away — so every candidate is available at runtime:

```lunas
@use Panel  from "./Panel.lunas"   <!-- both emitted because a <component :is> -->
@use Notice from "./Notice.lunas"  <!-- exists in this template -->
```

You still declare each candidate with `@use`; that is how the factory identifiers
(`Panel`, `Notice`) come into scope for the `:is` expression to reference.

## Switching and remount-on-change

The slot re-mounts **when the factory identity changes** — i.e. when `:is`
evaluates to a *different* factory than before:

- `view = Notice` (was `Panel`) → unmount `Panel`, mount `Notice`.
- Setting `view` to the *same* factory it already holds → **no** remount.
- Setting `view` to a falsy value → render **nothing** (the old child is
  unmounted, no new one mounts).

Because switching is a genuine unmount + mount, the outgoing component's state is
**discarded** and the incoming component starts **fresh**. If you need to switch
between components while *preserving* their state (so returning to a component
restores its scroll position, form input, etc.), wrap the dynamic slot in
keep-alive instead of relying on `:is` alone; see [../built-ins/](../built-ins/).

## Passing props

Props on a `<component :is>` tag flow to whichever component is currently
mounted, exactly like a normal child mount:

```lunas
<component :is="view" :label="title"/>
```

- Reactive props (`:label="title"`) are forwarded and kept live: changing
  `title` pushes the new value into the current child.
- The same props object is reused across remounts and re-seeds the fresh child
  when `:is` switches — so after a swap, `Notice` receives the current `title`
  too.

Whether a given prop is *meaningful* depends on the mounted component: if `Panel`
declares `@input label` but `Notice` doesn't, the `:label` value is simply ignored
by `Notice`. Provide the union of props the candidates need.

## How it compiles

`<component :is="expr">` compiles to a `dynamicBlock` at a text anchor:

```js
// <component :is="view" :label="title"/>
const dyn = dynamicBlock(
  c,
  anchor,
  [/* deps of `view` */],
  () => view,                 // factoryOf: the current factory
  { label: () => title },     // props: reactive props as getters
);
bind(c, [/* deps of title */], () => dyn.setProp("label", title)); // keep props live
```

- `factoryOf()` returns the current factory. When it changes (its deps flush),
  `dynamicBlock` unmounts the old child and mounts the new one via `mountChild`
  at the same anchor.
- Props are forwarded through `setProp`, reused across remounts.
- The handle exposes `{ handle, update(), setProp(name, value), destroy() }`
  (`handle` is the current `mountChild` handle, or `null` when nothing is
  mounted).

## Gotchas

- **`:is` must evaluate to a component *factory*, not a string.** In Lunas you
  bind the factory itself (`let view = Panel`), not a tag name. Import each
  candidate with `@use` so its identifier is in scope.
- **Every `@use` is emitted when a `<component :is>` is present.** That is
  intentional — dead-code elimination can't see which factory `:is` will pick.
  Remove unused `@use` lines to keep the import set tight.
- **Switching resets state.** A swap unmounts and remounts; the outgoing
  component's reactive state is gone. Use keep-alive to preserve it.
- **A falsy `:is` renders nothing.** Setting `view` to `null`/`undefined`
  unmounts the current child and leaves the slot empty until `:is` becomes a
  factory again.

## Related

- [registration.md](./registration.md) — `@use` (candidates must be declared).
- [props.md](./props.md) — how forwarded props behave.
- [async-components.md](./async-components.md) — lazily-loaded components, which
  compose with dynamic switching.
- [../built-ins/](../built-ins/) — keep-alive for state-preserving switches.
- [../api/](../api/) — the `dynamicBlock` runtime helper.
