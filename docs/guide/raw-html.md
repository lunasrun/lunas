# Raw HTML

`:html` sets an element's inner HTML from an expression. Use it to render
pre-formatted or server-provided markup that you can't express with the template.

```lunas
html:
    <article :html="markup"></article>
    <button @click="fill()">fill</button>

script:
    let markup = "<b>bold</b>"
    function fill() { markup = "<i>italic</i>" }
```

The value is inserted as HTML (both initially and reactively when it changes).

## How it compiles

`:html="markup"` compiles to an `innerHTML` assignment wrapped in a reactive
bind:

```js
bind(c, [/* deps of markup */], () => { e0.innerHTML = markup.v; });
```

## XSS caveat — read this

**The value is inserted verbatim.** Any markup — including `<script>`, event-
handler attributes, and `<img onerror=…>` — is parsed and can execute. **Never
bind untrusted input** (user content, URL parameters, API responses you don't
control) through `:html`. Sanitize it first, or render it as text with
[`${...}`](./template-syntax.md), which escapes automatically.

```lunas
<!-- SAFE: text is escaped -->
<p>${userInput}</p>

<!-- DANGEROUS: only for markup you fully trust -->
<article :html="trustedMarkup"></article>
```

If you must render user-supplied HTML, run it through a sanitizer (e.g. a
DOMPurify-style library) in script before binding it.

## Children are overwritten

`:html` **replaces** the element's children. Putting both static children and
`:html` on the same element is a conflict — the compiler emits a warning, and the
`:html` value wins (the static children are overwritten at runtime):

```lunas
<!-- Warns: the "keep me" content is overwritten by markup -->
<div :html="markup">keep me</div>
```

Give the raw-HTML value its own dedicated element with no static children.

## Notes

- Reserved bound names `:innerHtml` and `:textContent` are **rejected** by the
  compiler — `:html` is the supported way to set inner HTML.
- Setting HTML re-parses the fragment on each change, replacing prior nodes; any
  DOM state inside the previous markup (focus, listeners you attached manually) is
  discarded. Prefer declarative template constructs where you can.

## Related

- [Template syntax](./template-syntax.md) — `${...}` for escaped text.
- [Reactivity fundamentals](./reactivity-fundamentals.md) — how the bind updates.
