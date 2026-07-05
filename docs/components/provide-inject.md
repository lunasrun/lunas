# Provide / inject — dependency injection down the tree

Props and events pass data between a parent and its *direct* child. When a value
needs to reach a **deep descendant** — a theme, a current user, a service handle
— threading it through every intermediate component ("prop drilling") is
tedious. `provide` / `inject` let an ancestor publish a value that any descendant
can pull, no matter how deep, without the components in between knowing about it.

## The basics

An ancestor calls `provide(c, key, value)`; any descendant calls
`inject(c, key, default?)` to read it:

```lunas
<!-- App.lunas (ancestor) -->
@use Page from "./Page.lunas"
html:
    <Page/>
script:
    provide("theme", "dark")
```

```lunas
<!-- DeepButton.lunas (some descendant, any depth below App) -->
html:
    <button class="btn ${theme}">click</button>
script:
    let theme = inject("theme", "light")   // "dark" — resolved from App
```

- In `.lunas` script the component context `c` is implicit — you write
  `provide("theme", "dark")` and `inject("theme", "light")`.
- The descendant does not need to know *which* ancestor provided the value, or
  how many levels up it is.

## How resolution works

`inject` walks the **parent-context chain** — the `childCtx.parent` links that
`mountChild` sets when it mounts a child. It returns the value from the **nearest
ancestor** that provided the key (self included), stopping at the first match.

```
App        provide("theme", "dark")
 └─ Layout
     └─ Panel
         └─ DeepButton   inject("theme")  → walks up → finds App's "dark"
```

The walk is O(depth). Only children mounted via `mountChild` are linked, so a
**root** component's `parent` is `null` and `inject` on a root falls straight
through to its default.

## Keys — strings or Symbols

A key may be a **string** or a **Symbol**:

```lunas
script:
    provide("theme", "dark")            // string key
```

```lunas
script:
    const StoreKey = Symbol("store")
    provide(StoreKey, { count: 1 })     // Symbol key
```

String and Symbol keys are distinct — a string `"store"` will **not** match a
`Symbol("store")`. Symbols avoid accidental collisions between unrelated
providers that happen to pick the same string name; export a shared `Symbol` from
a module when several components must agree on the same injection key.

## Defaults

`inject(c, key, default)` returns `default` when no ancestor provides the key:

```lunas
let color = inject("accent", "#0af")   // "#0af" if nobody provided "accent"
```

Omit the default and an unprovided key yields `undefined`:

```lunas
let color = inject("accent")           // undefined if unprovided
```

The default is a plain value (matching the common case). If you need a lazily
constructed default, pass a thunk and call it yourself:

```lunas
let svc = inject("service") || makeService()
```

### Provided-`undefined` vs. absent

An ancestor can deliberately provide `undefined`. To distinguish "provided the
value `undefined`" from "never provided", use `hasInjection`:

```lunas
if (hasInjection(c, "maybe")) {
    // some ancestor provided "maybe" (its value may itself be undefined)
} else {
    // no ancestor provides "maybe" at all
}
```

`hasInjection` walks the same chain and returns a boolean.

## Nearest wins (shadowing)

If several ancestors provide the **same key**, the **nearest** one shadows the
others for a given descendant:

```
App     provide("k", "root")
 └─ Mid   provide("k", "mid")   ← shadows App's "root"
     └─ Leaf   inject("k")  → "mid"
```

A component deeper than `Mid` sees `"mid"`; a component between `App` and `Mid`
(if any) sees `"root"`. This lets a subtree override an inherited value locally
without affecting the rest of the tree.

## How it compiles

`provide` / `inject` are direct runtime calls; there is no special template
syntax. They compile to:

```js
// provide("theme", "dark")
provide(c, "theme", "dark");   // registers on c._provides (a Map)

// inject("theme", "light")
const theme = inject(c, "theme", "light");   // walks c.parent chain, nearest wins
```

`provide` stores the key on the component's own `_provides` Map (a later
`provide` of the same key on the same component overwrites it). `inject` walks
`c.parent` links — the chain `mountChild` builds — to the nearest provider.

## Reactivity note

`provide` registers a value; it is not itself a reactive channel. If you provide
a plain value and later replace it, existing injectors that already read it hold
the old value. To share **reactive** state broadly, provide a reactive container
(a box, a store, or an object you mutate) rather than a bare snapshot — the
descendants then read through that live container. For app-wide reactive state
outside any single component, prefer a **store** (`createStore` / `useStore`);
see [../scaling/](../scaling/).

## When to use provide/inject vs. props

- **Props** — direct, explicit parent → child value. Best for a component's
  declared interface. Prefer props when the relationship is one level deep or the
  value is genuinely part of the child's API.
- **provide/inject** — cross-cutting concerns that many descendants need (theme,
  locale, current user, a service handle) where threading props through every
  level would be noise. Use sparingly; it creates an implicit dependency that
  isn't visible in a component's tag.

## Gotchas

- **Only `mountChild`-linked descendants can inject.** A component mounted
  outside the tree (standalone root) has `parent = null` and only sees its
  defaults.
- **String and Symbol keys never collide.** Pick one convention per key and stick
  to it; export shared `Symbol`s for keys that multiple modules must agree on.
- **Nearest provider wins.** A closer ancestor's `provide` shadows a farther one
  for that subtree.
- **A bare provided value is a snapshot.** Provide a reactive container if
  descendants must see later changes.

## Related

- [props.md](./props.md) — the explicit, one-level-deep alternative.
- [events.md](./events.md) — child → parent communication.
- [../scaling/](../scaling/) — module-level stores for app-wide reactive state.
- [../api/](../api/) — `provide`, `inject`, `hasInjection` runtime helpers.
