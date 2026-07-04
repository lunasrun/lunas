// blocks.d.ts — types for src/blocks.mjs
// Control-flow blocks anchored at permanent text nodes.

import type { Context } from "./core.js";
import type { ForSeed, KeyOf, PatchItem, WarnFn } from "./for_diff.js";

/** A branch's DOM output: a single root node, or a multi-root node array. */
export type BlockNodes = Node | Node[];

/** A handle returned by ifBlock/forBlock/mountChild for whole-block teardown. */
export interface BlockHandle {
  destroy(): void;
}

/**
 * ifBlock(c, anchor, deps, cond, make)
 * `make()` returns a node (single-root branch) or an array of nodes
 * (multi-root branch — the compiler knows which and emits accordingly).
 * The branch is inserted before the permanent anchor when cond() becomes
 * truthy and removed (with scope teardown) when it becomes falsy.
 */
export function ifBlock(
  c: Context,
  anchor: Node,
  deps: number[],
  cond: () => boolean,
  make: () => BlockNodes
): BlockHandle;

/** Options for forBlock; all fields besides `make` are optional. */
export interface ForBlockOpts<T = unknown> {
  /** Build one item; returns node or node array. */
  make(itemData: T, key: unknown): BlockNodes;
  /** Compiled :key extractor (optional; falls back to item identity/index). */
  keyOf?: KeyOf<T>;
  /** Update an existing item's scope in place (optional). */
  patch?: PatchItem<T>;
  /** Duplicate-key warning hook (optional). */
  onWarn?: WarnFn;
  /** Seed from a bulk innerHTML initial render (optional); when present the
   * initial reconcile is skipped because the items are already mounted. */
  seed?: ForSeed<T>;
}

/**
 * forBlock(c, anchor, deps, items, opts)
 * `items` is a closure returning the current array (read lazily at flush
 * time). Updates go through the keyed LIS reconciler; innerHTML is never
 * used here.
 */
export function forBlock<T = unknown>(
  c: Context,
  anchor: Node,
  deps: number[],
  items: () => T[],
  opts: ForBlockOpts<T>
): BlockHandle;

/** A factory that builds one component instance given props. */
export type ChildFactory<P = Record<string, unknown>> = (props?: P) => Node;

/** Handle for a mounted child component. */
export interface MountedChild {
  root: Node;
  unmount(): void;
}

/**
 * mountChild(c, anchor, childFactory, props) — instantiate a child
 * component and insert its root before the anchor.
 */
export function mountChild<P = Record<string, unknown>>(
  c: Context,
  anchor: Node,
  childFactory: ChildFactory<P>,
  props?: P
): MountedChild;
