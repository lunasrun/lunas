# Props — `@input`

A component declares the inputs it accepts with `@input`. Props are Lunas's
one-way channel from parent to child: the parent supplies values (static or
reactive), and the child reads them like any other reactive variable. When a
reactive prop's source changes in the parent, the child re-renders the parts
that read it — automatically, with no wiring on your side.

## Declaring a prop

`@input` lines go at the top of the child's `.lunas` file, alongside `@use`:

```lunas
@input start:number = 0

html:
    <div class="counter">
        <span class="value">${value}</span>
        <button @click="inc()">+</button>
    </div>

script:
    let value = start
    function inc() { value = value + 1 }
```

The grammar is:

```
@input <name>:<type>[?] [= <default>]
```

| Part | Meaning |
|---|---|
| `<name>` | the prop name — the attribute the parent passes (`:start="…"`). |
| `<type>` | the declared type (`string`, `number`, `boolean`, a custom type, …). Used for typing/tooling; see [Typing](#typing). |
| `?` | optional marker — the prop may be omitted with no default. |
| `= <default>` | a default expression, used when the parent omits the prop. |

Examples:

```lunas
@input tone:string = "plain"    <!-- optional via default -->
@input text:string = "?"        <!-- optional via default -->
@input label:string             <!-- required: no default, no ? -->
@input title:string?            <!-- optional, no default (undefined if omitted) -->
```

A declared prop name is in scope in the `script:` block as a value you can read
(`let value = start`), and in the template as an interpolation (`${text}`).

## Passing props from the parent

Reference the child by its `@use` tag and pass attributes:

```lunas
@use Counter from "./Counter.lunas"
@use Badge   from "./Badge.lunas"

html:
    <Counter :start="seed"/>       <!-- reactive: bound to `seed` -->
    <Badge text="static label"/>   <!-- static: a literal string -->
    <Badge :text="user"/>          <!-- reactive: bound to `user` -->

script:
    let seed = 10
    let user = "ada"
```

There are two flavours of prop passing:

### Static props

A plain attribute (`text="static label"`) — or an interpolation-free value — is a
**static prop**: a constant passed once at construction. A valueless flag
attribute (`disabled`) passes `true`.

```lunas
<Badge text="hello"/>   <!-- text = "hello" -->
<Badge/>                <!-- text = "?" (the child's default) -->
```

### Reactive props

A bound attribute (`:start="seed"`, `:text="user"`) — or a static value
containing an interpolation — is a **reactive prop**. The child seeds from it at
construction, *and* the parent keeps it live: whenever the bound expression's
dependencies change, the new value is pushed into the child.

```lunas
<Counter :start="seed"/>
```

If the parent later does `seed = 20`, the child's `start` prop updates and any
child template part that reads it re-renders.

### How it compiles

A child mounts through the `mountChild` runtime helper. Reactive props are passed
as **getters** in the initial props object (so the child seeds correctly at
construction), and each reactive prop gets a parent-side `bind` that pushes new
values via `setProp`:

```js
// <Counter :start="seed"/>  with a static "static" prop for illustration
const ch0 = mountChild(c, anchor, Counter, {
  start: () => seed,      // reactive prop → getter
  static: "x",            // static prop → plain value
});
bind(c, [/* deps of seed */], () => ch0.setProp("start", seed)); // keeps it live
```

- The **getter** seeds the child's reactive prop box at construction.
- The **`bind`** re-runs on the source expression's compile-time deps and calls
  `ch0.setProp("start", seed)`. The bind's initial run re-seeds the same value,
  which the box no-ops on (an equal write does nothing).
- **Static props** are plain values in the initial object — no driving `bind` is
  emitted.

Inside the child, each `@input` prop is adopted as a **reactive box** with the
`prop` helper at the top of `setup`:

```js
const start = prop(c, "start", i, props.start, /* default */ 0);
```

`prop` seeds the box from `props.start` (calling it if the parent passed a
getter, else using the value), or from the default when the prop is omitted, and
registers the box under `c._props["start"]` so the parent's `setProp` can drive
it.

## One-way data flow

Data flows **parent → child only**. `setProp` writes the child's own prop box —
`child._props[name].v` — which marks the **child** dirty and flushes the child.
It never touches the parent's reactive state.

Symmetrically, the child mutating a prop locally (`value = value + 1` above, or
mutating a prop directly) changes only the child's copy and does **not**
propagate back to the parent. The two component contexts are fully independent:

> A parent prop push marks only the child; a child event marks only the child.
> Parent and child never cross-contaminate reactive state.

If a child needs to send information upward, use an **event** ([events.md](./events.md))
rather than mutating a prop. This one-way discipline is what keeps updates
predictable.

## Reactivity of props (parent change → child updates)

Because a reactive prop is driven by a parent-side `bind`, the flow is:

1. Parent state changes (`seed = 20`).
2. The parent's driving bind re-runs and calls `ch0.setProp("start", 20)`.
3. `setProp` writes the child's `start` box → the **child** is marked dirty.
4. The child flushes; every child template part that reads `start` re-runs.

```lunas
<!-- Parent -->
@use Badge from "./Badge.lunas"
html:
    <Badge :text="user"/>
    <button @click="user = 'grace'">rename</button>
script:
    let user = "ada"
```

Clicking the button sets `user = "grace"`; the `Badge`'s `${text}` re-renders to
`[grace]` on the next flush. You wire nothing extra — declaring `:text` as a
bound attribute is enough.

### Deep mutation of an object/array prop

By default a prop box is a plain reassign-only `box`. If the child deeply mutates
the prop value locally (e.g. `arr.push(…)`, `obj.k = …`), the compiler selects a
`deepBox` for that prop (the `deep` flag on `prop`), so deep mutations mark the
child dirty. Parent-driven whole-value replacement still works either way.

## Typing

The `:type` in `@input name:type` documents the prop's type for tooling and
readability. Use the framework's type names (`string`, `number`, `boolean`) or
your own type identifiers. The `?` marker and the presence/absence of a
`= default` together describe optionality:

| Declaration | Required? | Value when omitted |
|---|---|---|
| `@input x:number` | required | — |
| `@input x:number?` | optional | `undefined` |
| `@input x:number = 0` | optional | `0` |

## Gotchas

- **A static prop is never reactive.** `text="user"` passes the literal string
  `"user"`, not the value of the `user` variable — you almost certainly meant
  `:text="user"`. The `:` prefix is what makes an attribute bound.
- **Mutating a prop in the child does not update the parent.** It only updates
  the child's local copy. Emit an event instead.
- **Equal writes are free.** The prop box no-ops when the new value equals the
  old one (`x !== v` check), so re-pushing the same value costs nothing.
- **Passing a getter yourself isn't the API.** You write `:prop="expr"`; the
  compiler turns it into a getter + driving bind. Don't pass a function unless the
  prop's *type* is a function.

## Related

- [registration.md](./registration.md) — `@use` (a prop-passing tag must be
  `@use`d first).
- [events.md](./events.md) — the child → parent direction (the counterpart to
  props).
- [slots.md](./slots.md) — passing *content* (not just values) to a child.
- [../guide/reactivity](../guide/) — how reactive boxes and flushing work.
- [../api/](../api/) — the `prop` / `mountChild` runtime helpers.
