# Introduction

Lunas is a single-file-component web framework: a `.lunas` file bundles an
`html:` template, an optional `style:` block, and a `script:` block, and a
**Rust compiler** turns it into plain JavaScript that targets a **tiny,
dependency-free runtime**.

```lunas
html:
    <main>
        <h1>${title}</h1>
        <button @click="inc()">Count: ${count}</button>
    </main>

script:
    let title = "Hello Lunas"
    let count = 0
    function inc() { count = count + 1 }
```

If you have used Svelte or Vue single-file components, this will feel familiar.
The differences are in how it compiles and how it stays fast.

## What Lunas is

Lunas has two parts:

- **A compiler** (Rust). It parses your component, works out exactly which
  template parts depend on which reactive variables, and emits a small,
  purpose-built JS module — no framework interpreter shipped to the browser.
- **A runtime** (`lunas` on npm). A dependency-free ES2015 module. Its whole
  reactive core is a handful of functions; only the ones your component uses are
  imported, and it tree-shakes cleanly.

There is **no virtual DOM**, no runtime dependency-tracking graph, and no
per-node effect objects.

## Why it is fast

Two compounding sources of speed, both settled by cross-hardware benchmarks (see
the [API reference](../api/) for the design contract):

1. **Static DOM is built in bulk by the browser's native HTML parser.** The
   compiler emits the static skeleton of your component as one contiguous
   `innerHTML` string. Dynamic seams are excluded from that string and
   represented as lightweight runtime anchors, so a mostly-static component is
   essentially a single native parse. The static HTML is kept whitespace-free and
   **comment-free** on purpose — a comment node drops Chromium's parser off its
   fast path.
2. **Dependencies are resolved at compile time.** Because the compiler already
   knows which variables each dynamic part reads, runtime reactivity setup is
   nearly free: just registering precomputed dependency indices. A write enqueues
   exactly the affected update functions; a microtask flush runs only those. No
   graph is discovered at runtime.

The headline win is *fast initial render* (construction offloaded to the native
parser) plus *near-zero reactivity setup*. Updates are targeted node mutations —
`innerHTML` is never used to update, so DOM state, listeners, focus, and
selection are preserved.

## How it compares

Lunas sits in the **compiler + small runtime** family alongside Svelte, and
shares Vue 3's template ergonomics (`:attr`, `@event`, `:if`/`:for`, slots,
`provide`/`inject`). The distinguishing choices:

| | Lunas | Svelte 5 | Vue 3 |
|---|---|---|---|
| Reactivity | compile-time dependency dispatch (numbered vars) | signals/runes | signals + proxy tracking |
| DOM construction | bulk `innerHTML` of a static skeleton | incremental DOM ops | virtual DOM |
| Runtime tracking | none (deps known at compile time) | signal graph | proxy graph |
| Reactive authoring | mutate a top-level `let` | `$state` / runes | `ref` / `reactive` |
| Compatibility floor | ES2015 (no Proxy) | modern | modern |

What makes a variable reactive in Lunas is simply **mutating a top-level `let`**
in your script. You write ordinary JavaScript; the compiler wires the
reactivity. See [Reactivity fundamentals](./reactivity-fundamentals.md).

## Feature overview

Everything below is documented in this guide:

- [Template syntax](./template-syntax.md) — `${}` interpolation, `:attr`
  bindings, `@event` handlers.
- [Reactivity fundamentals](./reactivity-fundamentals.md) — what makes state
  reactive, the flush/microtask model.
- [Computed values](./computed.md) — lazy, memoized derived state.
- [Class and style bindings](./class-and-style.md) — `:class` / `:style`.
- [Conditional rendering](./conditional-rendering.md) — `:if` / `:elseif` /
  `:else`.
- [List rendering](./list-rendering.md) — `:for` with keyed reconciliation.
- [Event handling](./event-handling.md) — `@event` handlers and arguments.
- [Forms & two-way binding](./forms-and-two-way.md) — `::value` / `::checked`.
- [Watchers](./watchers.md) — `watch` / `watchEffect`.
- [Template refs](./template-refs.md) — `:ref` for elements and components.
- [Lifecycle](./lifecycle.md) — `onMount` / `onDestroy` / `onUpdate`.
- [Raw HTML](./raw-html.md) — `:html` and its XSS caveat.

Beyond the essentials, Lunas also ships [components & props](../components/),
[built-ins](../built-ins/) (slots, teleport, keep-alive, transitions, dynamic
components), and [scaling](../scaling/) features (stores, router, provide/inject,
async components). See the [API reference](../api/) for the runtime surface.

Ready to build something? Head to the [Quick start](./quick-start.md).
