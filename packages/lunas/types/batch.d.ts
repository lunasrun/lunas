// batch.d.ts — types for src/batch.mjs
// Update coalescing and nextTick.

import type { Context } from "./core.js";

/**
 * nextTick(c) — a Promise resolved after the next flush completes (i.e.
 * after the DOM update pass). If nothing is pending the flush is still
 * scheduled, so `await nextTick(c)` always lands after the current tick's
 * update pass.
 */
export function nextTick(c: Context): Promise<void>;

/**
 * batch(c, fn) — run fn (which may write many boxes) and flush
 * synchronously afterward, collapsing the whole group into one update pass
 * that has completed by the time batch() returns. Nested batches on the
 * same context flush only at the outermost call. Returns whatever `fn`
 * returns.
 */
export function batch<T>(c: Context, fn: () => T): T;
