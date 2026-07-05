// lifecycle.d.ts — types for src/lifecycle.mjs
// Component lifecycle hooks + the attach contract (§7).

import type { Context } from "./core.js";

/** onMount(c, fn) — run `fn` after this component's root attaches to a live
 *  tree. If already mounted, `fn` runs on the next microtask. */
export function onMount(c: Context, fn: () => void): void;

/** onDestroy(c, fn) — run `fn` when this component is torn down (fires once). */
export function onDestroy(c: Context, fn: () => void): void;

/** onUpdate(c, fn) — run `fn` after each flush of `c` that ran updates. */
export function onUpdate(c: Context, fn: () => void): void;

/** onActivated(c, fn) — keep-alive: run `fn` each time the component is
 *  (re)activated from the cache (fires on first activation too). */
export function onActivated(c: Context, fn: () => void): void;

/** onDeactivated(c, fn) — keep-alive: run `fn` each time the component is
 *  deactivated (cached, not destroyed). */
export function onDeactivated(c: Context, fn: () => void): void;

/** attach(root, host) — append a detached component root to a live host and
 *  fire the whole subtree's onMount callbacks. Returns `root`. */
export function attach<N extends Node>(root: N, host: Node): N;

/** isLive(node) — whether `node` is attached to a live tree (uses
 *  `Node.isConnected` with a walk-to-root fallback for shims). */
export function isLive(node: Node | null | undefined): boolean;
