# Event handling

An `@`-prefixed attribute wires a DOM event listener. The event name is whatever
follows the `@` (`@click`, `@input`, `@keydown`, …), and the value is a
JavaScript handler.

## Method vs inline handlers

```lunas
html:
    <button @click="inc()">+1</button>
    <input @keydown="onKey" />

script:
    let count = 0
    function inc() { count = count + 1 }
    function onKey(e) { if (e.key == "Enter") inc() }
```

Two forms:

- **Inline call** — `@click="inc()"`. The value is an expression evaluated when
  the event fires. It compiles to `on(e0, "click", () => { inc(); })`.
- **Bare reference** — `@keydown="onKey"`. The named function is used as the
  listener directly, so it receives the DOM event as its argument.

Use a bare reference when your handler needs the event object; use an inline call
when you want to pass specific arguments.

## Arguments

Because the inline form is just an expression, you can pass any arguments —
including loop variables and data:

```lunas
html:
    <ul>
        <li :for="item of items" :key="item.id">
            ${item.label}
            <button @click="remove(item.id)">×</button>
        </li>
    </ul>

script:
    let items = [{ id: 1, label: "a" }]
    function remove(id) { items = items.filter(x => x.id !== id) }
```

Here `item.id` is captured per-item, so each button removes its own row.

To also use the DOM event alongside arguments, reference it explicitly in the
inline expression (e.g. `@input="onType($event)"` where your handler takes the
event), or use a bare reference and read `e.target` inside.

## How handler writes trigger updates

Handlers drive reactivity simply by **mutating reactive state**. When a handler
reassigns or deeply mutates a reactive variable, the box setter marks that
variable dirty; the affected [bindings](./reactivity-fundamentals.md) are
enqueued and run on the next microtask flush.

```lunas
script:
    let count = 0
    let log = []
    function inc() {
        count = count + 1     // marks count's index dirty
        log.push(count)       // marks log's index dirty (deepBox)
        // both updates flush together on the next microtask
    }
```

You don't declare what a handler changes — the compiler analyzes the handler (and
functions it calls) to know its write set, and the box setters do the enqueuing
at runtime. Multiple writes in one handler coalesce into a single DOM update
pass.

## Component events

`@name` on a **component tag** is different from a DOM event: it listens for an
event the child *emits*, not a native DOM event. `@save="onSave($event)"` becomes
an `onSave` handler passed to the child, which raises it with `emit`. Names are
camel-cased (`@save-all` → `onSaveAll`). The handler runs in the parent. See
[components](../components/) for child-to-parent events.

## Related

- [Reactivity fundamentals](./reactivity-fundamentals.md) — how writes schedule
  updates.
- [Forms & two-way binding](./forms-and-two-way.md) — `::value` combines a bind
  with an input listener.
- [Template syntax](./template-syntax.md).
