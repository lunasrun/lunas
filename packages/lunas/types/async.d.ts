// async.d.ts — types for src/async.mjs
// Async / lazy components + suspense boundaries.

import type { Context } from "./core.js";
import type { ChildFactory, MountedChild } from "./blocks.js";

/** An imported module, an `{ default }` ES-module namespace, or a bare factory. */
export type AsyncModule<P = Record<string, unknown>> =
  | ChildFactory<P>
  | { default: ChildFactory<P> };

/** A loader for a lazy component: `() => import("./Heavy.mjs")` or similar. */
export type AsyncLoader<P = Record<string, unknown>> = () =>
  | Promise<AsyncModule<P>>
  | AsyncModule<P>;

/** Options for {@link asyncComponent}. */
export interface AsyncComponentOptions<P = Record<string, unknown>> {
  /** Component shown while pending — only after `delay` ms, to avoid a flash. */
  loading?: ChildFactory<P>;
  /** Component shown on load rejection or timeout. Receives `{ error }` in props. */
  error?: ChildFactory<P & { error?: unknown }>;
  /** Milliseconds to wait before showing `loading` (default 200). */
  delay?: number;
  /** Milliseconds after which a still-pending load is treated as an error. */
  timeout?: number;
}

/**
 * asyncComponent(loader, opts) — wrap a lazy module loader into a mountable
 * child factory. The module is resolved on first mount (default export or bare
 * factory both accepted) and cached, so later mounts build synchronously.
 * Mount it with {@link mountAsyncChild} (threads the context for suspense).
 */
export function asyncComponent<P = Record<string, unknown>>(
  loader: AsyncLoader<P>,
  opts?: AsyncComponentOptions<P>
): ChildFactory<P>;

/**
 * mountAsyncChild(c, anchor, asyncFactory, props) — mount an async component
 * factory (from {@link asyncComponent}) at a text anchor. Same contract as
 * `mountChild`, but threads the component context so the async component can
 * register with the nearest {@link suspenseBlock}; its `unmount()` cancels any
 * in-flight load so a late resolution writes no DOM.
 */
export function mountAsyncChild<P = Record<string, unknown>>(
  c: Context,
  anchor: Node,
  asyncFactory: ChildFactory<P>,
  props?: P
): MountedChild;

/** Handle returned by {@link suspenseBlock}. */
export interface SuspenseHandle {
  /** Whether the boundary has revealed its content (all async deps settled). */
  isSettled(): boolean;
  /** Tear the boundary down, cancelling any pending async children. */
  destroy(): void;
}

/**
 * suspenseBlock(c, anchor, contentFactory, fallbackFactory) — a boundary at a
 * text anchor. It builds the content immediately (so async children begin
 * loading) but shows `fallback` until every async dep registered under it
 * resolves, then reveals the content (batched via afterFlush — a fully
 * synchronous subtree never flashes the fallback). Nested boundaries handle
 * their own subtree; each async child registers with its nearest boundary.
 */
export function suspenseBlock(
  c: Context,
  anchor: Node,
  contentFactory: (c: Context) => Node | Node[],
  fallbackFactory?: () => Node | Node[]
): SuspenseHandle;
