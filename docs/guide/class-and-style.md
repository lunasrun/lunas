# Class and style bindings

`:class` and `:style` are the ergonomic way to compute an element's classes and
inline styles. They accept strings, objects, and arrays, and they **merge with**
any static `class` / `style` attribute already on the element.

## `:class`

The bound value can be a string, an object, or an (arbitrarily nested) array:

```lunas
html:
    <div class="box" :class="{ active: on, big: huge }"></div>

script:
    let on = false
    let huge = true
```

### Accepted shapes

- **String** — used verbatim (trimmed): `:class="theme"`.
- **Object** — each key is included when its value is truthy:
  `:class="{ active: on, big: huge }"` → `active big` when both are truthy.
- **Array** — flattened recursively; entries may be strings or objects:
  `:class="['tag', { on: active }, extra]"`.
- **Falsy** (`null`, `false`) — contributes nothing.

### Merging with the static class

The static `class` attribute is always kept and prepended:

```lunas
<div class="box" :class="{ active: on }"></div>
```

When `on` is true the element's class becomes `box active`; when false it is just
`box`. The dynamic part is normalized (`normClass`) and joined onto the static
string, then the whole `class` attribute is written. If the merged result is
empty, the `class` attribute is removed.

## `:style`

The bound value can be a string, an object of camelCase properties, or an array:

```lunas
html:
    <div :style="{ color: hue, fontWeight: weight }"></div>

script:
    let hue = "red"
    let weight = "bold"
```

### Accepted shapes

- **String** — used verbatim (trimmed): `:style="cssText"`.
- **Object** — keys are CSS properties written in **camelCase**, converted to
  kebab-case: `{ fontWeight: "bold" }` → `font-weight: bold;`. Custom properties
  (`--x`) and already-kebab names pass through unchanged. Entries whose value is
  `null` or `false` are skipped.
- **Array** — merged left-to-right; later entries win.

### Merging with the static style

The static `style` attribute is kept and prepended (a trailing `;` is added if
missing), then merged with the normalized dynamic style. As with class, an empty
merged result removes the `style` attribute.

## How it compiles

`:class` and `:style` compile to `setClass` / `setStyle` calls that receive the
static value and the dynamic expression, wrapped in a `bind` on the expression's
dependencies:

```js
// <div class="box" :class="{ active: on, big: huge }">
bind(c, [/* deps of on, huge */], () => {
  setClass(e0, "box", { active: on.v, big: huge.v });
});
```

`setClass`/`setStyle` normalize the dynamic value, merge it with the static
string, and write the whole attribute — so the two never fight over the same
element.

## Notes

- Object keys for `:style` are **camelCase** (`fontWeight`), not kebab
  (`font-weight`); the runtime converts them.
- `:class` object keys are class names as-is; only their truthiness matters.
- These bindings are reactive: they re-run when any variable they read changes,
  exactly like other [attribute bindings](./template-syntax.md).

## Related

- [Template syntax](./template-syntax.md) — attribute bindings and static
  interpolation (`class="a ${x} b"`).
- [Reactivity fundamentals](./reactivity-fundamentals.md).
