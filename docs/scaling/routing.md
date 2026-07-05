# Routing

Lunas ships a client-side router as part of its runtime. A router is
conceptually a **store whose one reactive field is the current route** — so a
component adopts the current route the same way it adopts any other
[store](./state-management.md) field, and plain JS (tests, devtools, the outlet)
can subscribe without a component context.

## Defining a router

```js
import { createRouter } from "lunas";
import Home from "./pages/Home.lunas";
import User from "./pages/User.lunas";
import NotFound from "./pages/NotFound.lunas";

export const router = createRouter([
  { path: "/",            component: Home },
  { path: "/users/:id",   component: User },
  { path: "*",            component: NotFound }, // catch-all → 404
]);
```

A route is `{ path, component, ...extra }`. Any extra keys (a `name`, `meta`,
etc.) ride along on the route definition and are available on the matched route.

## Route matching

`createRouter` pre-compiles each path into segments. A segment is one of:

| Segment | Meaning | Example |
|---|---|---|
| **static** | must equal the incoming segment | `/users` |
| **param** (`:name`) | captures exactly one segment | `/users/:id` |
| **catch-all** (`*` or `*name`) | captures the rest (0+ segments) | `/files/*path` |

### Precedence

When several routes match, they're **ranked**: static beats param beats
catch-all, and earlier segments dominate later ones. Ties fall back to
declaration order (first wins). So given:

```js
{ path: "/users/new" },   // static — wins for /users/new
{ path: "/users/:id" },   // param  — wins for /users/42
{ path: "*" },            // catch-all — 404 for everything else
```

`/users/new` matches the static route even though `/users/:id` also *could*
match, because static outranks param.

- A catch-all captures every remaining segment as a single `/`-joined string
  under its name (default `rest`): `*path` matching `/a/b/c` gives
  `params.path === "a/b/c"`.
- Trailing slashes are normalized away: `/a/b/` and `/a/b` match identically.

### The reactive route object

The current route is `{ path, params, query, matched }`:

- `path` — normalized pathname (leading slash, no trailing slash, query
  stripped).
- `params` — captured `:param` / `*catch` values.
- `query` — the parsed `?a=1&b=2` string as a plain object (last value wins for
  repeated keys; `+` decodes to space).
- `matched` — the matched route definition, or `null` on a total miss (declare a
  `*` route to always have a 404 slot).

## Navigating

```js
router.push("/users/42");         // navigate, push a new history entry
router.push("/search?q=lunas");   // query strings are parsed into route.query
router.replace("/login");         // navigate, replace the current entry
router.back();                    // go back one entry (like the browser button)
router.forward();                 // go forward one entry
```

`push` and `replace` return a `Promise<boolean>` that resolves to whether the
navigation **committed** (a guard may cancel it — see below). `back` / `forward`
are guard-free, mirroring the browser buttons.

## Navigation guards

`beforeEach(to, from)` runs before every `push`/`replace`. Return `false` (sync
or via a resolved Promise) to **cancel** the navigation; anything else commits
it:

```js
export const router = createRouter(routes, {
  beforeEach(to, from) {
    if (to.matched?.meta?.requiresAuth && !isLoggedIn()) {
      router.replace("/login");
      return false; // cancel the original navigation
    }
    return true;
  },
});
```

The guard may be async (return a `Promise<boolean>`) — useful for awaiting an
auth check. When a guard cancels, the store is left untouched (no route change).

> Guards run on `push`/`replace` only, not on `back`/`forward` — browser
> back/forward land already committed to history and are resolved into the store
> directly.

## Rendering the matched route: `routerOutlet`

An **outlet** mounts the matched route's component and swaps it whenever
navigation changes *which route matches*:

```html
<!-- app template -->
<nav> … </nav>
<router-outlet />
```

which compiles to a `routerOutlet(c, anchor, router)` call. Key behavior:

- The matched route's captured **params are passed as props** to the component.
  So `/users/:id` mounts `User` with `{ id }`.
- The outlet **re-mounts only when the matched route definition changes**, not on
  every param tweak. Navigating `/users/1 → /users/2` keeps the same `User`
  instance mounted; its own props reactivity handles the `id` change. Navigating
  `/users/2 → /` swaps to `Home`.
- Pass `options.props(route)` to override/augment the prop mapping (e.g. to
  inject the router itself or the parsed query).

## Links: `routerLink`

`<a>` links wire up to client-side navigation instead of a full page load:

```html
<a :href="/users/42">View user</a>
```

compiles to `routerLink(aEl, router, "/users/42")` alongside setting the static
`href` (kept for SSR / no-JS / middle-click). On a plain left click it calls
`preventDefault()` + `router.push(path)`. **Modified clicks**
(ctrl/meta/shift/alt) and non-primary buttons fall through to the browser, so
"open in new tab" still works. Pass `{ replace: true }` to use `replace` instead
of `push`.

## Reading the current route in a component

A component that reads `router.current` in its template or handlers adopts the
route at a reactive index — the compiler emits `router.adopt(c, i)` (sugar over
`useStore(c, i, router.store, "route")`). From then on, route changes re-render
the parts of that component that read the route:

```html
<script>
  import { router } from "./router.mjs";
</script>

<p>Current path: {router.current.path}</p>
<p :if="router.current.params.id">User #{router.current.params.id}</p>
```

Outside a component (plain JS — tests, devtools), subscribe directly:

```js
const stop = router.subscribe((route) => {
  console.log("navigated to", route.path);
});
// …later
stop();
```

## `@useRouting` / `@useAutoRouting`

The router runtime is the codegen target for the `@useRouting` /
`@useAutoRouting` directives. `@useAutoRouting` expands a placeholder into an
outlet wired to the app router (the `<router-outlet/>` shape above); `@useRouting`
gives you the router instance to drive manually. Both compile down to
`createRouter` + `routerOutlet` + `router.adopt`, so everything documented here
applies regardless of which directive you use.

## Testing with `memoryHistory`

History access is behind an injectable adapter, so you can drive a router in a
test (or any non-browser environment) with an in-memory stand-in:

```js
import { createRouter, memoryHistory } from "lunas";

const router = createRouter(routes, {
  history: memoryHistory("/users/1"),
});

await router.push("/users/2");
router.current.params.id; // "2"
router.back();            // fires the store update, like popstate
```

`memoryHistory(initial)` keeps its own back-stack. `back()`/`forward()` invoke
the router's resolve (the popstate analogue); `push`/`replace` do **not** fire
the listener (mirroring the browser, where `pushState` emits no `popstate` — the
router resolves after its own programmatic navigation). The browser default is
`historyAdapter(window)`, used automatically when you omit `options.history`.

## Gotchas

- **`route.matched` is a proxy view**, not `===` the original route object (it's
  read through the store's deep-mutation proxy). Compare by a stable key like
  `route.matched.path`, not by identity — this is exactly how the outlet decides
  whether to re-mount.
- **Params are always strings.** `/users/:id` gives `params.id === "42"`, not the
  number `42`. Parse it yourself if you need a number.
- **Declare a `*` route** if you want a real 404 page; without one, an unmatched
  path leaves `matched === null` and the outlet renders nothing.
- **Guards don't run on back/forward** — those follow browser history directly.
- Call `router.destroy()` to detach the history listener on teardown / HMR.

## See also

- [State management](./state-management.md) — the store the router is built on.
- [Keep-alive](../built-ins/keep-alive.md) — wrap the outlet to preserve page
  state across navigation.
- [Suspense](../built-ins/suspense.md) — lazy-load route components.
- [Runtime API](../api/runtime.md) — `createRouter`, `routerOutlet`,
  `routerLink`, `memoryHistory`.
