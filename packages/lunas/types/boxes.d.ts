// boxes.d.ts — types for src/boxes.mjs
// Per-variable reactive boxes, specialized by the compiler.

import type { Context } from "./core.js";

/** A reassign-only reactive cell backed by reactive index `i` in context `c`. */
export interface Box<T> {
  v: T;
}

/**
 * A deeply-mutated reactive cell. `.v` returns the RAW value (no Proxy); a deep
 * mutation is made reactive by an explicit invalidation the compiler injects
 * after the mutating statement.
 */
export interface DeepBox<T> {
  v: T;
  /** Signal a structural deep mutation (`arr.push`, `arr[i]=x`, `obj.k=v`, …). */
  touch(): T;
  /** Signal an element-field mutation of a direct array element `el`. */
  touchElem<E>(el: E): E;
}

/**
 * box(c, i, v) — reassign-only variable at reactive index i.
 * Lightest path: plain getter/setter, no Proxy. Same-value writes are no-ops.
 */
export function box<T>(c: Context, i: number, v: T): Box<T>;

/**
 * deepBox(c, i, v) — deeply-mutated variable (arr.push, obj.k = …, nested field
 * writes, Map/Set mutations).
 *
 * Proxy-free (Svelte-family model): `.v` returns the RAW value, so reads are as
 * cheap as a plain `box`. A deep mutation is made reactive by an explicit
 * invalidation the compiler injects right after the mutating statement:
 * `box.touch()` for a structural change (`push`/`splice`/`arr[i]=x`/`obj.k=v`/
 * `delete`/`length=`/Map-Set `set`/`add`/`delete`/`clear`) and
 * `box.touchElem(el)` for an element-field write (`arr[i].label = x`). Both mark
 * the variable dirty; `markVar` defers the flush to a microtask.
 */
export function deepBox<T>(c: Context, i: number, v: T): DeepBox<T>;

/**
 * prop(c, name, i, raw, def, deep) — adopt an `@input` prop as a reactive
 * variable at index i. The child reads it as a box (`.v`), so its own
 * template binds react when the prop changes. The seed is `raw` when the
 * parent passed a value, else the compiled default `def`. `deep` selects a
 * deepBox (the child deeply mutates the prop locally).
 */
export function prop<T>(
  c: Context,
  name: string,
  i: number,
  raw: T | (() => T) | undefined,
  def: T,
  deep?: boolean
): Box<T>;

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
