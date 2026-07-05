# Suspense

A **suspense boundary** shows a fallback while asynchronous children inside it
are still loading, then reveals the real content once every pending async
dependency has resolved. It's the coordination point for
[async / lazy components](#async-components) — instead of each lazy component
managing its own spinner, one boundary waits for the whole group.

## Example

```html
<script>
  import { asyncComponent } from "lunas";

  const Profile = asyncComponent(() => import("./Profile.lunas"));
  const Feed    = asyncComponent(() => import("./Feed.lunas"));
</script>

<Suspense>
  <template #default>
    <Profile />
    <Feed />
  </template>

  <template #fallback>
    <p>Loading…</p>
  </template>
</Suspense>
```

Both `Profile` and `Feed` begin loading immediately. The `Loading…` fallback is
shown until **both** modules have resolved and mounted, then the fallback is torn
down and the real content revealed in one swap.

## How the boundary coordinates

The compiler emits the runtime's
[`suspenseBlock`](../api/runtime.md):

```js
suspenseBlock(c, anchor, contentFactory, fallbackFactory);
```

- `contentFactory(c)` builds the content **immediately** — so async children
  start loading right away — but the content is kept hidden while anything is
  pending.
- Each async component under the boundary registers itself with the **nearest**
  boundary (published on the component context as `c._suspense` during the
  content build). It bumps the boundary's pending counter while its module loads
  and settles it once the module resolves (or fails).
- `fallbackFactory()` builds the fallback shown while the pending counter is
  above zero.
- When the counter reaches zero, the fallback is removed and the content
  revealed.

### No fallback flash on synchronous content

The reveal is batched via the runtime's `afterFlush`. If every async dependency
was already cached (so the subtree resolves synchronously during the build), the
pending counter hits zero before the first paint and the content is revealed
**directly — the fallback never mounts**. You only see the fallback when there is
genuinely async work to wait for.

## Nested boundaries

Boundaries nest, and each owns exactly its own subtree. An inner
`<Suspense>` saves the enclosing boundary, installs itself while building its
content, then restores the parent — so an async dependency always registers with
its **nearest** boundary and never leaks a pending count upward.

```html
<Suspense>
  <template #fallback><p>Loading page…</p></template>

  <Header />

  <!-- The Feed's own loading is scoped to this inner boundary; the outer
       boundary reveals as soon as Header (and this inner boundary's own
       placeholder) are ready. -->
  <Suspense>
    <template #fallback><p>Loading feed…</p></template>
    <Feed />
  </Suspense>
</Suspense>
```

## Async components

`asyncComponent(loader, opts)` turns a dynamic-import loader into a mountable
component factory:

```js
import { asyncComponent } from "lunas";

const Heavy = asyncComponent(() => import("./Heavy.lunas"), {
  loading: Spinner,   // shown while pending — only after `delay` ms
  error: ErrorCard,   // shown on rejection or timeout (gets `{ error }` in props)
  delay: 200,         // ms before showing `loading` (default 200; avoids a flash)
  timeout: 10000,     // ms after which a still-pending load is treated as an error
});
```

- The loader can return a dynamic `import()`, an ES-module namespace
  (`{ default }`), or a bare factory. The default export or bare factory is
  unwrapped automatically.
- The resolved module is **cached** after the first load, so later mounts build
  **synchronously** with no placeholder.
- `loading` is shown only after `delay` ms, so a fast load never flashes a
  spinner. `error` is shown on rejection or after `timeout`.

Async components work **with or without** a surrounding `<Suspense>`:

- **Inside** a boundary, they register their pending state with it and the
  boundary's fallback covers them collectively.
- **Standalone**, each one shows its own `loading` / `error` components per its
  own `delay` / `timeout`.

Mount async children through the runtime's `mountAsyncChild` (the compiler emits
this) — it threads the component context so the child can find the nearest
boundary, and its `unmount()` **cancels any in-flight load** so a late resolution
writes no DOM.

## Unmount safety

Every async mount owns a live token. Unmounting (including tearing down a
boundary that's still showing its fallback) flips that token, so a loader that
resolves *afterwards* writes nothing and settles no boundary. Destroying a
`suspenseBlock` cancels any pending async children in its (possibly still-hidden)
content.

## Gotchas

- **Content is built eagerly, shown lazily.** Async children start loading the
  moment the boundary is created, even though you don't see them until they're
  ready — that's what lets the boundary wait for the whole group in parallel.
- **`delay` on a standalone async component defaults to 200 ms.** Inside a
  suspense boundary the boundary's fallback is what you see, so the child's own
  `loading` typically isn't needed.
- **A rejected async component still settles its boundary.** Suspense reveals
  content once everything *settles* (resolved or errored); it does not block
  forever on a failed load. Handle the failure with the async component's
  `error` option.
- **Nested boundaries scope their waiting.** A slow child under an inner boundary
  won't hold up an outer boundary that has nothing else pending.

## See also

- [Keep-alive](./keep-alive.md) — cache resolved panes so they don't reload.
- [Routing](../scaling/routing.md) — lazy-load route components with
  `asyncComponent`.
- [Runtime API](../api/runtime.md) — `suspenseBlock`, `asyncComponent`,
  `mountAsyncChild`.
