// router.d.ts — types for src/router.mjs
// Client-side router runtime: route table + matching, history integration,
// store-backed reactive current route, outlet, links, and navigation guards.

import type { Context } from "./core.js";
import type { Store } from "./store.js";

/** Parsed query string: last-value-wins for repeated keys. */
export type Query = Record<string, string>;

/** Captured route params (`:name` segments and `*name` catch-all). */
export type Params = Record<string, string>;

/** A route definition in the route table. */
export interface Route<C = unknown> {
  /** Path pattern: static segments, `:param`s, and a trailing `*`/`*name`. */
  path: string;
  /** The component factory to mount when this route matches (outlet target). */
  component?: C;
  /** Arbitrary extra data carried on the route (name, meta, …). */
  [extra: string]: unknown;
}

/** The reactive current-route value held in the router's store. */
export interface RouteState<C = unknown> {
  /** Normalized pathname (leading slash, no trailing slash, query stripped). */
  path: string;
  /** Captured params from the matched route. */
  params: Params;
  /** Parsed query string. */
  query: Query;
  /** The matched route definition, or null on a total miss (no catch-all). */
  matched: Route<C> | null;
}

/** Unsubscribe / detach function. */
export type Unsubscribe = () => void;

/**
 * A history adapter: the injectable seam between the router and the browser
 * History API. `memoryHistory()` and `historyAdapter()` both satisfy it.
 */
export interface HistoryAdapter {
  /** Current location as `pathname + search`. */
  readonly location: string;
  /** Push a new history entry (does NOT invoke `listen`'s callback). */
  push(path: string): void;
  /** Replace the current history entry (does NOT invoke `listen`'s callback). */
  replace(path: string): void;
  /** Go `delta` entries in history; invokes `listen`'s callback on a move. */
  go(delta: number): void;
  /** Subscribe to popstate-style changes; returns an unsubscribe. */
  listen(fn: (path: string) => void): Unsubscribe;
}

/**
 * memoryHistory(initial) — an in-memory History-API stand-in for tests/SSR.
 * Keeps its own back-stack; `listen`'s callback fires on back()/forward()
 * (the popstate analogue) but not on push()/replace().
 */
export function memoryHistory(initial?: string): HistoryAdapter;

/**
 * historyAdapter(win) — the default adapter over the browser History API.
 * `win` defaults to the global `window`. Throws if no window is available
 * (pass a memoryHistory() via options.history for non-browser environments).
 */
export function historyAdapter(win?: Window): HistoryAdapter;

/** Options for createRouter. */
export interface RouterOptions<C = unknown> {
  /** History adapter; defaults to historyAdapter() over `window`. */
  history?: HistoryAdapter;
  /**
   * Navigation guard run before every push/replace. Returning `false` (sync or
   * via a resolved Promise) cancels the navigation; anything else commits it.
   */
  beforeEach?: (
    to: RouteState<C>,
    from: RouteState<C>
  ) => boolean | Promise<boolean>;
}

/** A router instance created by createRouter. */
export interface Router<C = unknown> {
  /** The backing store; components adopt the route field through it. */
  readonly store: Store<{ route: RouteState<C> }>;
  /** The current reactive route object. */
  readonly current: RouteState<C>;
  /**
   * Navigate, pushing a new history entry. Resolves to whether the navigation
   * committed (a beforeEach guard may cancel it).
   */
  push(path: string): Promise<boolean>;
  /** Navigate, replacing the current history entry. */
  replace(path: string): Promise<boolean>;
  /** Go back one history entry (guard-free, like the browser button). */
  back(): void;
  /** Go forward one history entry. */
  forward(): void;
  /**
   * Plain-JS subscription to route changes; `fn(route)` runs synchronously on
   * every committed navigation. Returns an unsubscribe.
   */
  subscribe(fn: (route: RouteState<C>) => void): Unsubscribe;
  /**
   * adopt(c, i) — sugar over `useStore(c, i, router.store, "route")`: adopt the
   * current route at component context `c`'s reactive index `i`. Returns a
   * detach() (idempotent, scope-aware). Compiler-facing hook for a component
   * that reads `router.current`.
   */
  adopt(c: Context, i: number): Unsubscribe;
  /** Detach the history listener (for teardown/HMR). */
  destroy(): void;
}

/**
 * createRouter(routes, options) — build a router over `routes`. The current
 * route lives in a store field named "route", so components adopt it via
 * `router.adopt(c, i)` (or `useStore`) and plain consumers via
 * `router.subscribe(fn)`. Matching ranks static > param > catch-all.
 */
export function createRouter<C = unknown>(
  routes: Route<C>[],
  options?: RouterOptions<C>
): Router<C>;

/** A handle for whole-outlet teardown. */
export interface OutletHandle {
  destroy(): void;
}

/** Options for routerOutlet. */
export interface OutletOptions<C = unknown> {
  /**
   * Map the current route to the props passed to the mounted component.
   * Defaults to the matched route's captured params.
   */
  props?: (route: RouteState<C>) => Record<string, unknown>;
}

/**
 * routerOutlet(c, anchor, router, options) — mount the matched route's
 * component at the text `anchor` (mountChild semantics), swapping it out
 * whenever navigation changes which route matches. Params are passed as props.
 * Re-mounts only when the matched route definition changes, not on every param
 * tweak. Returns a handle with destroy().
 */
export function routerOutlet<C = unknown>(
  c: Context,
  anchor: Node,
  router: Router<C>,
  options?: OutletOptions<C>
): OutletHandle;

/** Options for routerLink. */
export interface LinkOptions {
  /** Use router.replace instead of router.push. */
  replace?: boolean;
}

/**
 * routerLink(el, router, path, options) — wire `el`'s click to a client-side
 * navigation (preventDefault + router.push, or replace when options.replace).
 * Modified/aux clicks fall through to the browser. Returns an unbind().
 */
export function routerLink(
  el: Element,
  router: Router,
  path: string,
  options?: LinkOptions
): Unsubscribe;

/** parseQuery(p) — the `?a=1&b=2` portion of `p` as a plain object. */
export function parseQuery(p: string): Query;

/** normalizePath(p) — canonical pathname (leading slash, no trailing, no query). */
export function normalizePath(p: string): string;
