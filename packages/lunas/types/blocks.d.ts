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

/**
 * ifChain(c, anchor, deps, which, makes) — one :if/:elseif/:else cascade at a
 * single permanent anchor. `which()` returns the index of the branch that
 * should be shown, or -1 for "no branch". Exactly one branch is alive at a
 * time; switching tears the old branch down and builds the new one.
 */
export function ifChain(
  c: Context,
  anchor: Node,
  deps: number[],
  which: () => number,
  makes: Array<() => BlockNodes>
): BlockHandle;

/** One item's builder in mount mode: returns the child root node(s) plus an
 * optional patch closure that updates the item's data cell on a keyed re-run. */
export interface MountResult {
  node: BlockNodes;
  patch?: (itemData: unknown, index: number) => void;
}

/** Options for forBlock. Exactly one construction mode is used per call:
 * - compiled: `html` + `wire` (bulk-innerHTML fast path for element items),
 * - mount: `mount` (`:for` over a component tag — one mountChild per item),
 * - make: `make` (generic per-item builder).
 * All other fields are optional. */
export interface ForBlockOpts<T = unknown> {
  /** Generic per-item builder; returns node or node array. */
  make?(itemData: T, key: unknown, index: number): BlockNodes;
  /** Compiled-mode item skeleton HTML (bulk innerHTML fast path). */
  html?: string;
  /** Compiled-mode per-item wiring; may return a patch closure. */
  wire?(root: Node, itemData: T, index: number): ((d: T, i: number) => void) | void;
  /** Mount-mode per-item builder (`:for` over a component). */
  mount?(itemData: T, key: unknown, index: number): MountResult;
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

/**
 * dynamicBlock(c, anchor, deps, factoryOf, props) — dynamic component
 * (`:is`). `factoryOf()` returns the current child factory, or a falsy value
 * for "render nothing". Whenever the factory identity changes, the old child
 * is unmounted and the new one is mounted at the same anchor via mountChild.
 */
export interface DynamicBlockHandle<P = Record<string, unknown>> {
  readonly handle: MountedChild | null;
  update(): void;
  setProp(name: string, value: unknown): void;
  destroy(): void;
}
export function dynamicBlock<P = Record<string, unknown>>(
  c: Context,
  anchor: Node,
  deps: number[],
  factoryOf: () => ChildFactory<P> | null | undefined | false,
  props?: P
): DynamicBlockHandle<P>;

/**
 * teleportBlock(c, anchor, targetOf, build) — teleport/portal. `build()`
 * returns the content node(s) (like an :if branch make()). `targetOf()`
 * resolves the mount target: a selector string or an Element. The content is
 * inserted into the target instead of inline at `anchor`.
 */
export interface TeleportHandle {
  nodes: Node[];
  destroy(): void;
}
export function teleportBlock(
  c: Context,
  anchor: Node,
  targetOf: () => string | Element | null | undefined,
  build: () => BlockNodes
): TeleportHandle;

/** onCleanup registrar passed to a parent-provided slot content factory. */
export type SlotOnCleanup = (fn: () => void) => void;

/** The parent-provided factory for a slot's content. */
export type SlotFactory<S = unknown> = (
  slotProps: S | undefined,
  onCleanup: SlotOnCleanup
) => BlockNodes | null | undefined;

/**
 * slotBlock(childCtx, anchor, factory, fallback, slotPropsOf) — render slot
 * content at a `<slot>` anchor inside a CHILD component. `factory` is the
 * parent-provided slot content factory (wired against the parent's
 * context); `fallback` is the child's own fallback, shown only when
 * `factory` is absent.
 */
export function slotBlock<S = unknown>(
  childCtx: Context,
  anchor: Node,
  factory: SlotFactory<S> | null | undefined,
  fallback: ((slotProps: S | undefined) => BlockNodes) | null | undefined,
  slotPropsOf?: () => S
): { nodes: Node[] };

/**
 * slotContent(parentCtx, build, slotProps, onCleanup) — build the PARENT
 * half of a slot factory. Opens a fresh scope on the parent context, runs
 * `build(slotProps)` to create and wire the content against the parent, and
 * registers the scope's teardown through `onCleanup`.
 */
export function slotContent<S = unknown>(
  parentCtx: Context,
  build: (slotProps: S | undefined) => BlockNodes,
  slotProps: S | undefined,
  onCleanup: SlotOnCleanup
): BlockNodes;
