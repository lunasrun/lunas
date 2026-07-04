// for_diff.d.ts — types for src/for_diff.mjs
// The Lunas keyed :for reconciler (update phase). Host-abstracted so it can
// run without a DOM: the caller supplies a `host` implementing the minimal
// node-placement interface, plus factory and patch callbacks.

/** A reconciled key: primitives compare by value, objects by identity. */
export type Key = string | number | boolean | object;

/** Extract the key for item `itemData` at position `i` in the new array. */
export type KeyOf<T = unknown> = (itemData: T, i: number) => Key;

/** Update an existing mounted item's reactive scope with new data. */
export type PatchItem<T = unknown> = (node: unknown, itemData: T) => void;

/** Duplicate-key warning hook. */
export type WarnFn = (message: string) => void;

/**
 * The minimal node-placement interface the DOM binding provides at runtime.
 * `node` in insertBefore/remove is whatever makeItem returned (a real DOM
 * node, or a multi-root handle understood by the caller).
 */
export interface ReconcileHost<N = unknown> {
  /**
   * Move/insert `node` so it sits immediately before `refNode` in the
   * parent. `refNode === null` means "append at the end" (before the
   * permanent `:for` anchor, in the real runtime).
   */
  insertBefore(node: N, refNode: N | null): void;
  /** Detach `node` from the parent. */
  remove(node: N): void;
}

/** Build the DOM (or handle) for a new item and return its host node. */
export type MakeItem<T = unknown, N = unknown> = (
  itemData: T,
  key: Key
) => N;

/**
 * The mutable state the reconciler threads across updates, in current DOM
 * order: keys/nodes/data are parallel arrays over the mounted items.
 */
export interface ForState<T = unknown, N = unknown> {
  keys: Key[];
  nodes: N[];
  data: T[];
}

/** Seed data for a bulk-innerHTML initial render (see forBlock's opts.seed). */
export interface ForSeed<T = unknown, N = unknown> {
  keys: Key[];
  handles: N[];
  data: T[];
}

/** Options accepted by reconcile(); all fields optional. */
export interface ReconcileOpts<T = unknown, N = unknown> {
  keyOf?: KeyOf<T>;
  patchItem?: PatchItem<T>;
  onWarn?: WarnFn;
}

/** createForState() — an empty ForState to seed or reconcile against. */
export function createForState<T = unknown, N = unknown>(): ForState<T, N>;

/**
 * seedForState(state, keys, nodes, data) — record the starting order after
 * an initial bulk-innerHTML render, so the first reconcile() diffs against
 * reality instead of assuming an empty list.
 */
export function seedForState<T = unknown, N = unknown>(
  state: ForState<T, N>,
  keys: Key[],
  nodes: N[],
  data: T[]
): void;

/**
 * reconcile(state, host, items, makeItem, opts) — diff `state`'s previous
 * mounted order against the new `items` and mutate `host` so its children
 * end up exactly in the new key order, using prefix/suffix trimming, a
 * key->index map, and a longest-increasing-subsequence pass to minimize
 * node moves. Mutates `state` in place to reflect the new order.
 */
export function reconcile<T = unknown, N = unknown>(
  state: ForState<T, N>,
  host: ReconcileHost<N>,
  items: T[],
  makeItem: MakeItem<T, N>,
  opts?: ReconcileOpts<T, N>
): void;

/**
 * longestIncreasingSubsequence(arr) — positions (ascending) into `arr`
 * forming a longest strictly-increasing run of its non-NADA (-1) values.
 * O(n log n) patience-sorting with parent links.
 */
export function longestIncreasingSubsequence(arr: number[]): number[];
