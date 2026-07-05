// boxes.d.ts — types for src/boxes.mjs
// Per-variable reactive boxes, specialized by the compiler.

import type { Context } from "./core.js";

/** A reassign-only reactive cell backed by reactive index `i` in context `c`. */
export interface Box<T> {
  v: T;
}

/**
 * box(c, i, v) — reassign-only variable at reactive index i.
 * Lightest path: plain getter/setter, no Proxy. Same-value writes are no-ops.
 */
export function box<T>(c: Context, i: number, v: T): Box<T>;

/**
 * deepBox(c, i, v) — deeply-mutated variable (arr.push, obj.k = …).
 * Reads through `.v` return a Proxy that marks the variable dirty on any
 * nested set/delete. Nested objects are wrapped lazily on property access;
 * wrappers are cached per underlying object so identity is stable.
 *
 * Map/Set (and WeakMap/WeakSet) are collection-aware: accessors and methods
 * run against the real collection so native internal slots accept the
 * receiver, and mutating operations (Map `set`/`delete`/`clear`, Set
 * `add`/`delete`/`clear`) mark the variable dirty. Values stored inside a
 * collection are NOT deeply wrapped — reassign an entry to make a nested
 * change reactive. WeakMap/WeakSet do not throw but are not deeply reactive.
 */
export function deepBox<T>(c: Context, i: number, v: T): Box<T>;

/**
 * A value shared across components (prop passed down and mutated). Each
 * dependent component attaches with its own reactive index; a write marks
 * the variable dirty in every attached component.
 */
export interface Shared<T> {
  v: T;
  /** Attach a dependent context/reactive-index pair to this shared value. */
  attach(c: Context, i: number): void;
  /** Detach every attachment belonging to context `c`. */
  detach(c: Context): void;
}

/** shared(v) — create a value shared across components. */
export function shared<T>(v: T): Shared<T>;
