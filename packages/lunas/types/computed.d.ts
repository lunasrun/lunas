// computed.d.ts — types for src/computed.mjs
// Lazily-evaluated derived values in the adjacency model.

import type { Context } from "./core.js";

/** A read-only box-shaped handle whose `.v` getter yields the memoized result. */
export interface Computed<T> {
  readonly v: T;
}

/**
 * computed(c, i, deps, fn) — derived value at reactive index `i` reading the
 * upstream reactive indices in `deps`. The compute fn runs only when the
 * value is actually read AND an upstream dep has changed since the last
 * computation (lazy + memoized).
 */
export function computed<T>(
  c: Context,
  i: number,
  deps: number[],
  fn: () => T
): Computed<T>;
