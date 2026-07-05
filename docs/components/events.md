# Events — child → parent with `emit`

Props flow **down** (parent → child); events flow **up** (child → parent). A
child raises a named event with an optional payload by calling `emit`, and the
parent listens by attaching an `@name` handler to the child's tag. Under the
hood, `@name` on a component tag compiles to an `on<Name>` prop, and `emit`
looks that prop up and calls it.

## Raising an event from the child

In the child's `script`, call `emit(c, "<name>", payload)`:

```lunas
@input label:string = "Save"

html:
    <button @click="save()">${label}</button>

script:
    function save() {
        emit("saved", { at: Date.now() })
    }
```

- The first argument is the child's component context (`c`); in `.lunas` script
  it is available implicitly — you write `emit("saved", payload)`.
- `"saved"` is the event name (a plain identifier; kebab-case is allowed, see
  [Naming](#naming-the-name--onname-convention)).
- The second argument is an optional single **payload**.

## Listening in the parent

Attach an `@name` handler to the child's tag, exactly like a native DOM event:

```lunas
@use SaveButton from "./SaveButton.lunas"

html:
    <SaveButton label="Save now" @saved="onSaved($event)"/>

script:
    function onSaved(e) {
        console.log("child saved at", e.at)
    }
```

- `@saved="onSaved($event)"` runs `onSaved` in the **parent's** scope when the
  child emits `"saved"`.
- `$event` is the payload the child passed to `emit`.

### How it compiles — `@name` → `onName`

`@name` on a **component tag** does not become a DOM listener. It becomes an
`on<Name>` entry on the child's `mountChild` props object:

```js
// <SaveButton @saved="onSaved($event)"/>
mountChild(c, anchor, SaveButton, {
  label: "Save now",
  onSaved: ($event) => onSaved($event),   // @saved → onSaved
});
```

At the top of the child's `setup`, `registerEmits` stashes the props so `emit`
can find handlers; then `emit(c, "saved", payload)` looks up `onSaved` and calls
it:

```js
// child setup
registerEmits(c, props /*, ["saved"] optional declared list */);
// …later…
emit(c, "saved", { at: Date.now() });   // → props.onSaved({ at: … })
```

`eventPropName` is the exact mapping the compiler uses: `"save"` → `"onSave"`.

> **Component tag vs. DOM element.** `@click` on a `<button>` is a DOM listener;
> `@saved` on a `<SaveButton/>` is an emit handler. The compiler distinguishes
> them by whether the tag is a `@use`d component or a native element.

## Naming — the `@name` → `onName` convention

Event names are camel-cased with an `on` prefix:

| Emitted name | Handler prop | Parent attribute |
|---|---|---|
| `save` | `onSave` | `@save` |
| `close` | `onClose` | `@close` |
| `save-all` | `onSaveAll` | `@save-all` |
| `update-model-value` | `onUpdateModelValue` | `@update-model-value` |

Kebab-case segments are joined and capitalized: `save-all` → `onSaveAll`. Pick a
convention (kebab in templates is idiomatic) and the mapping is mechanical.

## Payloads

`emit` takes **one** payload argument. To send multiple values, pass an object:

```lunas
<!-- child -->
function submit() {
    emit("submit", { name: name, email: email })
}
```

```lunas
<!-- parent -->
<Form @submit="save($event)"/>
script:
    function save(data) { post(data.name, data.email) }
```

The parent receives that object as `$event`. There is no implicit argument
spreading — one payload, one parameter.

## `emit` does not mark the parent dirty

This is the key semantic to internalize:

> `emit` invokes the parent's `on<Name>` handler, but it does **not** by itself
> mark the parent's reactive state dirty. The handler decides.

If the handler mutates parent reactive state, the parent's box setters mark the
parent dirty and it flushes as usual:

```lunas
<!-- parent -->
@use Counter from "./Counter.lunas"
html:
    <Counter @changed="onChanged($event)"/>
    <p>total: ${total}</p>
script:
    let total = 0
    function onChanged(n) { total = total + n }  // box setter → parent flushes
```

Here the child's `emit("changed", 1)` calls `onChanged(1)`, which writes `total`
— *that* write is what re-renders `${total}`. An event whose handler touches no
reactive state produces no re-render. The two contexts stay independent: a child
event marks only the child; the parent updates only if its handler chooses to
mutate parent state.

## No parent, no listener → no-op

`emit` is safe to call even when nobody is listening:

- If the parent passed **no** matching `on<Name>` handler, `emit` is a no-op.
- If the child has **no parent** at all (mounted standalone), `emit` is a no-op.

In both cases `emit` returns `false`; when a handler ran it returns `true`. So a
reusable component can `emit("changed", …)` unconditionally without checking
whether anyone subscribed.

## Optional validation

`registerEmits` accepts an optional array of declared event names. Emitting a
name **not** in that list logs a `console.warn` (it does **not** throw) and
still runs any handler that happens to be attached:

```js
registerEmits(c, props, ["save", "close"]);
emit(c, "delete", …);   // warns: emitted undeclared event "delete"; handler still runs if present
```

This is a lean, warn-only guard — useful in development, never fatal.

## Gotchas

- **`@saved` on a native element is a DOM listener, not an emit.** Emit handlers
  only apply to `@use`d component tags. `@click` on `<button>` binds the native
  click event.
- **`emit` alone won't re-render the parent.** Make the handler mutate parent
  state if you want a re-render. (This is a feature: side-effect-only events
  don't force layout work.)
- **One payload only.** Bundle multiple values into an object.
- **Names are case/segment sensitive in the mapping.** `@save-all` maps to
  `onSaveAll`; the child must `emit(c, "save-all", …)` (kebab) — the runtime
  camel-cases it for the lookup.

## Related

- [props.md](./props.md) — the downward (parent → child) direction.
- [registration.md](./registration.md) — `@use` (a tag with `@name` must be a
  `@use`d component).
- [provide-inject.md](./provide-inject.md) — for cross-cutting communication
  that shouldn't thread through every level.
- [../api/](../api/) — `emit`, `registerEmits`, `eventPropName`.
