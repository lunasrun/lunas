# Router API

The client-side router runtime: a route table with matching, history
integration, a store-backed reactive current route, an outlet, links, and
navigation guards. Conceptually the router is a store whose single field
`"route"` is the current route — components adopt it exactly like any other store
field (see [store API](./store.md)).

See the [routing guide](../scaling/routing.md) for the conceptual walkthrough.

Import from the package root or the `lunas/router` subpath.

---

## `createRouter`

### Signature

```ts
function createRouter<C = unknown>(routes: Route<C>[], options?: RouterOptions<C>): Router<C>

interface Route<C> {
  path: string;              // static segments, ":param"s, trailing "*"/"*name"
  component?: C;             // factory to mount when matched (outlet target)
  [extra: string]: unknown;  // name, meta, …
}

interface RouterOptions<C> {
  history?: HistoryAdapter;  // defaults to historyAdapter() over window
  beforeEach?: (to: RouteState<C>, from: RouteState<C>) => boolean | Promise<boolean>;
}
```

### Description

Builds a router over `routes`. The current route lives in a store field named
`"route"`, so components adopt it via `router.adopt(c, i)` (or
[`useStore`](./store.md#usestore)) and plain consumers via `router.subscribe(fn)`.
Route matching ranks **static > param > catch-all**.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `routes` | `Route<C>[]` | The route table. |
| `options` | `RouterOptions<C>` | History adapter and `beforeEach` guard. |

### Returns

A `Router<C>` instance:

```ts
interface Router<C> {
  readonly store: Store<{ route: RouteState<C> }>;
  readonly current: RouteState<C>;
  push(path: string): Promise<boolean>;    // resolves to whether it committed
  replace(path: string): Promise<boolean>;
  back(): void;                            // guard-free, like the browser button
  forward(): void;
  subscribe(fn: (route: RouteState<C>) => void): Unsubscribe;
  adopt(c: Context, i: number): Unsubscribe; // sugar over useStore(c, i, store, "route")
  destroy(): void;                         // detach the history listener
}
```

### Example

```js
import { createRouter } from "lunas";
import Home from "./Home.mjs";
import User from "./User.mjs";

const router = createRouter([
  { path: "/", component: Home },
  { path: "/users/:id", component: User },
  { path: "*", component: NotFound },
]);
```

### Notes

- `push`/`replace` return a `Promise<boolean>` — a `beforeEach` guard returning
  `false` (sync or via a resolved promise) cancels the navigation.
- `back`/`forward` are guard-free (they mirror the browser buttons).

---

## The route shape (`RouteState`)

```ts
interface RouteState<C> {
  path: string;               // normalized pathname (leading slash, no trailing, no query)
  params: Record<string, string>; // captured ":name" segments and "*name" catch-all
  query: Record<string, string>;  // parsed query string (last-value-wins per key)
  matched: Route<C> | null;   // the matched route def, or null on a total miss
}
```

`router.current` is the live `RouteState`; components read it reactively after
`router.adopt(c, i)`.

---

## Navigation guards

`options.beforeEach(to, from)` runs before every `push`/`replace`. Returning
`false` — synchronously or via a resolved `Promise<boolean>` — cancels the
navigation; anything else commits it. `back()`/`forward()` bypass the guard.

```js
const router = createRouter(routes, {
  beforeEach: (to) => {
    if (to.matched?.meta?.auth && !isLoggedIn()) return false; // block
    return true;
  },
});
```

---

## `routerOutlet`

### Signature

```ts
function routerOutlet<C = unknown>(
  c: Context,
  anchor: Node,
  router: Router<C>,
  options?: OutletOptions<C>
): OutletHandle

interface OutletOptions<C> {
  props?: (route: RouteState<C>) => Record<string, unknown>;
}
interface OutletHandle { destroy(): void }
```

### Description

Mounts the matched route's `component` at the text `anchor` (with
[`mountChild`](./component.md#mountchild) semantics), swapping it out whenever
navigation changes **which route** matches. By default the matched route's
captured `params` are passed as props; override with `options.props`. Re-mounts
only when the matched route definition changes, **not** on every param tweak.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `c` | `Context` | The component context. |
| `anchor` | `Node` | The permanent text anchor for the outlet. |
| `router` | `Router<C>` | The router instance. |
| `options` | `OutletOptions<C>` | Optional route→props mapping. |

### Returns

`OutletHandle` — `{ destroy() }`.

### Example

```js
import { anchorBefore, routerOutlet } from "lunas";
const a = anchorBefore(placeholder);
routerOutlet(c, a, router);
```

---

## `routerLink`

### Signature

```ts
function routerLink(
  el: Element,
  router: Router,
  path: string,
  options?: LinkOptions
): Unsubscribe

interface LinkOptions { replace?: boolean }
```

### Description

Wires `el`'s click to a client-side navigation: `preventDefault` + `router.push`
(or `router.replace` when `options.replace`). Modified/aux clicks (Ctrl/Cmd/middle
button) fall through to the browser. Returns an `unbind()`.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `el` | `Element` | The link element. |
| `router` | `Router` | The router instance. |
| `path` | `string` | The destination path. |
| `options` | `LinkOptions` | `{ replace }`. |

### Returns

`Unsubscribe` — removes the click wiring.

### Example

```js
routerLink(anchorEl, router, "/users/1");
```

---

## `memoryHistory`

### Signature

```ts
function memoryHistory(initial?: string): HistoryAdapter
```

### Description

An in-memory History-API stand-in for tests and non-browser environments. Keeps
its own back-stack; `listen`'s callback fires on `back()`/`forward()` (the
popstate analogue) but **not** on `push()`/`replace()`. Pass it via
`options.history` to run a router without a `window`.

### Parameters

| Name | Type | Description |
| --- | --- | --- |
| `initial` | `string` | The initial location (default `"/"`). |

### Returns

A `HistoryAdapter`:

```ts
interface HistoryAdapter {
  readonly location: string;          // "pathname + search"
  push(path: string): void;
  replace(path: string): void;
  go(delta: number): void;
  listen(fn: (path: string) => void): Unsubscribe;
}
```

### Example

```js
import { createRouter, memoryHistory } from "lunas";
const router = createRouter(routes, { history: memoryHistory("/") });
```

### Notes

- The default adapter over the browser History API is `historyAdapter(win?)`
  (`win` defaults to global `window`; throws if no window is available). The
  router uses it automatically when no `history` option is given.
