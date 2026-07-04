// watch.d.ts — types for src/watch.mjs
// User-facing watchers over the adjacency model.

import type { Context } from "./core.js";

/** Stops (unbinds) the watcher/effect it was returned from. */
export type StopHandle = () => void;

/** Options accepted by watch(). */
export interface WatchOpts {
  /** Also invoke the callback once at registration time (default: false). */
  immediate?: boolean;
}

/**
 * watch(c, deps, cb, opts?) — run `cb` after any of `deps` changes. By
 * default the first (synchronous) run is suppressed, so the callback fires
 * only on subsequent changes. Pass { immediate: true } to also invoke it
 * once at registration time. The watcher is collected by the current scope
 * (torn down by dropScope) in addition to the returned stop().
 */
export function watch(
  c: Context,
  deps: number[],
  cb: () => void,
  opts?: WatchOpts
): StopHandle;

/**
 * watchEffect(c, deps, fn) — run `fn` immediately and again after any of
 * `deps` changes (no distinction between the initial and later runs).
 */
export function watchEffect(
  c: Context,
  deps: number[],
  fn: () => void
): StopHandle;
