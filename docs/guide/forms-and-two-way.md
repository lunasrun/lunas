# Forms and two-way binding

A `::`-prefixed attribute creates a **two-way binding**: it binds a value to the
element *and* writes user input back into your reactive state. `::value="name"`
is sugar for `:value="name"` plus an input listener that assigns
`name = el.value`.

## Text inputs — `::value`

```lunas
html:
    <input ::value="name">
    <p>Hello, ${name}</p>

script:
    let name = "ada"
```

Type in the field and `name` updates; assign `name` in script and the field
updates. The value of `::value` is an **lvalue** (an assignment target), not a
general expression — it must be something you can write back to.

### How it compiles

`::value="name"` emits both directions:

```js
bind(c, [0], () => { e0.value = name.v; });          // read side (property)
on(e0, "input", () => { name.v = e0.value; });        // write side (input event)
```

The read side runs on flush when `name` changes; the write side fires on every
`input` event.

## Checkboxes and radios — `::checked`

`::checked` binds a boolean and commits on the **`change`** event (checkbox/radio
semantics), not `input`:

```lunas
html:
    <input type="checkbox" ::checked="agree">
    <p>agree = ${agree}</p>

script:
    let agree = false
```

Compiles to:

```js
bind(c, [0], () => { e0.checked = !!(agree.v); });    // read side (boolean property)
on(e0, "change", () => { agree.v = e0.checked; });     // write side (change event)
```

**Radios:** bind each radio in a group to the same variable. Use `::checked`
against a boolean per option, or drive selection through `::value` plus the
radios' `value` attributes, depending on how you model the group.

## Select and other inputs

For a `<select>`, bind the selected value with `::value`; the write-back captures
the chosen option on input/change. The same `::name` mechanism generalizes: it
pairs a value binding with a write-back listener appropriate to the property.

```lunas
html:
    <select ::value="choice">
        <option value="a">A</option>
        <option value="b">B</option>
    </select>
```

## Combining with other directives

Two-way binding is just a value bind plus a listener, so you can add more
attributes and events freely:

```lunas
html:
    <input ::value="draft" @keydown="onKey" placeholder="What needs doing?" />

script:
    let draft = ""
    function onKey(e) { if (e.key == "Enter") add(draft) }
    function add(text) { /* … */ }
```

Here `::value` keeps `draft` in sync while a separate `@keydown` handler reacts to
Enter.

## Notes

- The bound target is an **lvalue**: `::value="user.name"` writes back to
  `user.name`. Deeply-mutated targets use a `deepBox`, so nested writes stay
  reactive.
- `::value` commits on `input` (every keystroke); `::checked` commits on `change`.
- If you need custom write-back logic, use a one-way [`:value`](./template-syntax.md)
  plus your own [`@input`/`@change`](./event-handling.md) handler instead.

## Related

- [Event handling](./event-handling.md).
- [Reactivity fundamentals](./reactivity-fundamentals.md).
- [Template syntax](./template-syntax.md) — one-way `:attr` bindings.
