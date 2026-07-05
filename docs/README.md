# Lunas Documentation

**Lunas** is a single-file-component web front-end framework with a Vue-parity
feature set. A `.lunas` file bundles an `html:` template, a `style:` block, and a
`script:` block (TypeScript or JavaScript); the Lunas compiler (written in Rust)
turns it into plain JavaScript plus a tiny, dependency-free runtime.

Lunas is fast at initial render because the static DOM is built by the browser's
native parser in bulk (`innerHTML`), dynamic parts are represented as lightweight
runtime anchors, and reactive dependencies are resolved **at compile time** — no
virtual DOM, no runtime dependency graph. The [architecture overview](./architecture.md)
explains how the pieces fit together end to end.

> **Status.** The runtime and the compiler front end are in active development on
> the `rewrite/beta-11` branch. **Server-side rendering (SSR) and hydration are
> planned but deferred** — the compiled output is designed for them (the static
> HTML string is exactly what a server would emit, and anchors are created at
> runtime so hydration can reuse the client wiring path), but the SSR codegen mode
> is not implemented yet.

---

## Guide

Conceptual, tutorial-style pages — start here.

- [Introduction](./guide/introduction.md)
- [Quick start](./guide/quick-start.md)
- [Template syntax](./guide/template-syntax.md)
- [Reactivity fundamentals](./guide/reactivity-fundamentals.md)
- [Computed values](./guide/computed.md)
- [Class and style bindings](./guide/class-and-style.md)
- [Conditional rendering](./guide/conditional-rendering.md)
- [List rendering](./guide/list-rendering.md)
- [Event handling](./guide/event-handling.md)
- [Forms and two-way binding](./guide/forms-and-two-way.md)
- [Watchers](./guide/watchers.md)
- [Template refs](./guide/template-refs.md)
- [Lifecycle](./guide/lifecycle.md)
- [Raw HTML](./guide/raw-html.md)

## Components

- [Registration](./components/registration.md)
- [Props](./components/props.md)
- [Events](./components/events.md)
- [Slots](./components/slots.md)
- [Provide / inject](./components/provide-inject.md)
- [Dynamic components](./components/dynamic-components.md)
- [Async components](./components/async-components.md)
- [Fragments](./components/fragments.md)

## Built-ins

- [Teleport](./built-ins/teleport.md)
- [Transition](./built-ins/transition.md)
- [Keep-alive](./built-ins/keep-alive.md)
- [Suspense](./built-ins/suspense.md)

## Scaling up

- [Routing](./scaling/routing.md)
- [State management](./scaling/state-management.md)
- [Scoped CSS](./scaling/scoped-css.md)
- [SSR](./scaling/ssr.md) *(planned / deferred)*
- [Tooling](./scaling/tooling.md)

## API reference

Per-symbol reference for the runtime's public surface (signatures, parameters,
returns, examples, notes). These are the primitives the compiler emits calls into.

- [Reactivity](./api/reactivity.md) — `box`, `deepBox`, `shared`, `computed`,
  `watch`, `watchEffect`, `batch`, `nextTick`, `afterFlush`, and the low-level
  core (`bind`, `markVar`, `flush`, scopes).
- [Component](./api/component.md) — `component`, `refs`, `on`, `normClass`,
  `normStyle`, `mountChild`.
- [Blocks & control flow](./api/blocks-and-control-flow.md) — anchors, `ifBlock`,
  `ifChain`, `forBlock`, `dynamicBlock`, `teleportBlock`, `slotBlock`,
  `slotContent`.
- [Router](./api/router.md) — `createRouter`, `routerOutlet`, `routerLink`,
  `memoryHistory`, guards, and the route shape.
- [Store](./api/store.md) — `createStore`, `useStore`, `subscribe`,
  `derivedStore`.
- [Async & suspense](./api/async.md) — `asyncComponent`, `mountAsyncChild`,
  `suspenseBlock`.
- [Lifecycle, events, DI, transitions & keep-alive](./api/lifecycle.md) —
  `onMount`/`onDestroy`/`onUpdate`/`onActivated`/`onDeactivated`, `attach`,
  `emit`/`eventPropName`, `provide`/`inject`/`hasInjection`, `withTransition`,
  `keepAlive`.

## Architecture

- [Architecture overview](./architecture.md) — the compile pipeline, the runtime
  model, and the benchmark-locked decisions behind them.
