# Template syntax

A Lunas template is HTML with a small binding overlay. Bindings live in one of
two places: **`${...}` interpolations** inside text and attribute values, or
**prefixed attribute keys** (`:`, `::`, `@`, plus the control-flow attributes
`:if` / `:elseif` / `:else` / `:for`).

## Text interpolation

`${ expr }` embeds a JavaScript expression in text:

```lunas
html:
    <p>Count: ${count}</p>
    <button>${interval == null ? "Start" : "Stop"}</button>

script:
    let count = 0
    let interval = null
```

- The expression is arbitrary JavaScript: member access, ternaries, calls,
  string literals, and object literals all work. The scanner is brace- and
  string-balanced, so `${ {a:1}.a }` and `${ f("}") }` terminate correctly.
- Multiple interpolations per text run are allowed; each becomes its own binding,
  and static text between them is preserved verbatim.
- An interpolation whose expression reads **no reactive variable** is assigned
  once at build time — no update binding is created for it.

### How it compiles

A reactive text run compiles to a single dynamic text node updated by a `bind`.
`<p>count: ${count}!</p>` becomes a static `<p></p>` plus:

```js
bind(c, [0], () => { t0.data = `count: ${count.v}!`; });
```

The literal parts (`count: ` and `!`) live in the dynamic text node, not the
static HTML, so the whole run is one node updated as one template literal.

## Attribute bindings

A `:`-prefixed attribute means "the value is a JavaScript expression, bind it
reactively":

```lunas
html:
    <h1 :title="label">${greeting}</h1>
    <div :class="isActive ? 'on' : 'off'"></div>

script:
    let label = "the heading"
    let greeting = "hello"
    let isActive = true
```

The reserved names `:innerHtml` and `:textContent` are rejected — use
[`:html`](./raw-html.md) for raw markup.

### Property vs attribute, and boolean attributes

The compiler picks the fastest write per attribute:

- **Known DOM properties** are set as properties. `:value="name"` compiles to
  `e0.value = name.v` rather than `setAttribute`.
- **Boolean attributes** are set as truthy properties:
  `:disabled="locked"` compiles to `e0.disabled = !!(locked.v)`.
- **Everything else** uses `setAttribute`: `:title="label"` compiles to
  `e0.setAttribute("title", label.v)`.

Each of these is wrapped in a `bind` on the expression's reactive dependencies,
so it re-runs only when a dependency changes.

## Interpolation inside static attributes

A plain (unprefixed) attribute value may itself contain `${...}`:

```lunas
html:
    <div class="tag ${flavor} end"></div>
    <section class="card ${tone}"></section>
```

This compiles to a reactive attribute write built from a template literal:

```js
bind(c, [0], () => { e0.setAttribute("class", `tag ${flavor.v} end`); });
```

Interpolation is recognized in attribute **values** only (not in attribute
names). For richer class/style logic, prefer [`:class` / `:style`](./class-and-style.md).

## Event handlers

An `@`-prefixed attribute wires a DOM event listener. See
[Event handling](./event-handling.md) for the full story.

```lunas
html:
    <button @click="inc()">bump</button>
    <input @keydown="onKey" />

script:
    let n = 0
    function inc() { n++ }
    function onKey(e) { if (e.key == "Enter") { /* … */ } }
```

`@click="inc()"` compiles to `on(e0, "click", () => { inc(); })`; a bare
reference like `@keydown="onKey"` is used as the handler directly.

## Two-way binding

A `::`-prefixed attribute is sugar for a value binding **plus** a write-back
listener. `::value="name"` is `:value="name"` combined with an `input` listener
that writes `name = el.value`. See [Forms & two-way binding](./forms-and-two-way.md).

## Expressions allowed

Anywhere a binding takes an expression (`${...}`, `:attr`, `@event`, `:if`,
etc.), you may use any JavaScript expression: identifiers, member access, calls,
ternaries, logical/comparison operators, template and object/array literals.
Reactive dependencies are detected by the compiler — a reactive `let` you *read*
inside an expression becomes a dependency of that binding, so the binding re-runs
when it changes.

## Directive summary

| Form | Example | Value is | Notes |
|---|---|---|---|
| Interpolation | `${count}` | JS expression | in text & attribute values |
| Attribute bind | `:class="x?'a':'b'"` | JS expression | `:innerHtml`/`:textContent` rejected |
| Two-way | `::value="title"` | JS lvalue | `:value` + `@input` write-back |
| Event | `@click="toggle()"` | JS handler/expression | |
| Raw HTML | `:html="markup"` | JS expression | overwrites children; [XSS caveat](./raw-html.md) |
| Ref | `:ref="field"` | binding name | [template refs](./template-refs.md) |
| If chain | `:if` / `:elseif` / `:else` | JS condition (`:else` none) | [conditional rendering](./conditional-rendering.md) |
| For | `:for="x of xs"` | for-loop header | [list rendering](./list-rendering.md) |

## Related

- [Reactivity fundamentals](./reactivity-fundamentals.md)
- [Class and style bindings](./class-and-style.md)
- [Conditional rendering](./conditional-rendering.md) ·
  [List rendering](./list-rendering.md)
