# Server-side rendering (SSR) & hydration

> **Status — PLANNED / DEFERRED.** SSR + client hydration is a designed-for but
> **not-yet-implemented** codegen mode. It is tracked as `a-ssr` in the roadmap
> with status **`deferred`**. There is **no SSR API to call today.** This page
> documents the intended approach so you can understand where Lunas is headed and
> why the current architecture is built to make it a drop-in addition — not to
> describe something you can use now.

## Why Lunas is "SSR-ready" even though SSR isn't built

Lunas's core construction strategy already produces the exact artifact a server
needs, for two structural reasons:

1. **The component's static HTML string is what a server would emit.** A Lunas
   component is built by parsing a static HTML string with the browser's native
   parser and then wiring reactive bindings against the resulting DOM. That same
   static string is precisely what a server would send down the wire.

2. **Anchors are created at runtime, not embedded in the HTML.** Control-flow
   anchors (the text nodes that mark where `:if` / `:for` / child components go)
   are created by the runtime during wiring — they are **not** baked into the
   HTML string as comments or markers. The server HTML stays clean and
   comment-free, which keeps server output small and avoids a parse penalty on
   the client's initial document parse.

Because of these two properties, **hydration can reuse the CSR wiring path**
almost verbatim.

## The designed-for hydration approach

Client-side rendering today does, per component:

```
parse static HTML (innerHTML)  →  positional nav to nodes  →  create anchors  →  bind(...)
```

Hydration would do the same thing **minus the parse**:

```
(server DOM already exists)   →  positional nav to nodes  →  create anchors  →  bind(...)
```

That is: **skip `innerHTML`** (the server-rendered subtree is already in the
document), then run the *same* positional-navigation + anchor-creation + `bind`
steps against the server-rendered DOM. Because the wiring logic is identical, the
hydration codegen mode reuses the CSR runtime rather than duplicating it.

Two facts make this consistent:

- **Positional node navigation** (`refs(root, paths)` walking `childNodes`
  indices) works the same on a parsed-from-string tree and a server-rendered
  tree, so the same `refs` paths locate the same nodes.
- **The scope attribute is stable across builds** (see
  [scoped CSS](./scoped-css.md)), so a server and a client agree on the same
  `data-lunas-<hash>` id — server-rendered scoped styles line up with
  client-side wiring.

## What is intentionally not decided yet

Because SSR is deferred, the following are **not** specified and may change when
the mode is actually built:

- The **server-side render entry point** (how you turn a component + props into
  an HTML string on the server).
- The **hydration entry point** on the client (the analogue of today's `attach`
  that adopts an existing subtree instead of inserting one).
- **Streaming**, per-request data loading, and how [async
  components](../built-ins/suspense.md) / [suspense](../built-ins/suspense.md)
  boundaries resolve on the server.
- **Router** integration for server-side route resolution (though
  [`memoryHistory`](./routing.md) already provides the injectable,
  non-browser history seam the server path would build on).

## What you can rely on today

- **Nothing in the current architecture blocks SSR** — it's an additive codegen
  mode, not a rewrite.
- The **transition** built-in already
  [degrades gracefully](../built-ins/transition.md) outside a browser (no
  `requestAnimationFrame`), so it won't hang a non-DOM render.
- The **router**'s history access is behind an injectable adapter
  ([`memoryHistory`](./routing.md)), explicitly noted as a tests/SSR seam.

If you need SSR now, it isn't available — plan for a client-rendered app and
track the `a-ssr` roadmap item.

## See also

- `crates/lunas_compiler/docs/output-design.md` §9 — the authoritative
  SSR-readiness note this page is based on.
- [Scoped CSS](./scoped-css.md) — the stable scope id that lets server and client
  agree.
- [Routing](./routing.md) — `memoryHistory`, the non-browser history seam.
- [Tooling](./tooling.md) — the current (client-only) build workflow.
