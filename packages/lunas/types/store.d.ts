// store.d.ts — types for src/store.mjs
// Module-level reactive state living outside any component.

import type { Context } from "./core.js";

/**
 * A store field's shape: attach/detach for component adoption, subscribe for
 * plain-JS consumers. `derivedStore()` results are also field-shaped and can
 * be placed under a createStore() key.
 */
export interface StoreField<T> {
  readonly v: T;
  attach(c: Context, i: number): void;
  detach(c: Context): void;
  subscribe(fn: (value: T) => void): () => void;
}

/** Unsubscribe function returned by store.subscribe() / useStore(). */
export type Unsubscribe = () => void;

/**
 * A module-level store created by createStore(). Field values may be read
 * and written by key from anywhere — component handlers or plain module
 * code — independent of any single component's reactive index.
 */
export interface Store<T extends Record<string, any> = Record<string, any>> {
  /** Current value of `key` (through the deep-mutation proxy for objects/arrays). */
  get<K extends keyof T>(key: K): T[K];
  /**
   * Write `key`, notifying every component that adopted it (batched per the
   * normal microtask flush) and every subscribe() listener. Same-value
   * writes are no-ops. Throws if `key` holds a derived (read-only) value.
   */
  set<K extends keyof T>(key: K, v: T[K]): void;
  /**
   * Outside-component subscription for plain-JS consumers (router, devtools,
   * tests). `fn(value)` runs synchronously on every write to `key`. Returns
   * an unsubscribe function.
   */
  subscribe<K extends keyof T>(
    key: K,
    fn: (value: T[K]) => void
  ): Unsubscribe;
  /** Internal: raw field accessor used by useStore/derivedStore. */
  _field(key: keyof T): StoreField<any>;
}

/**
 * createStore(initial) — create a module-level store from a plain object of
 * named initial values. Each key becomes an independent field (its own
 * subscriber list): a write to one field never notifies components that
 * only adopted another field. A value that is already field-shaped (e.g. the
 * result of derivedStore()) is kept as-is instead of being wrapped.
 */
export function createStore<T extends Record<string, any>>(initial: T): Store<T>;

/**
 * useStore(c, i, store, key) — adopt store field `key` at component context
 * `c`'s reactive index `i`. Writes to `key` mark index `i` dirty in `c`
 * (batched per the normal microtask flush), exactly like a compiler-emitted
 * `shared(...).attach(c, i)` call but sourced from a module-level store.
 *
 * Returns a detach() that undoes the adoption (idempotent). When called
 * while `c.scope` is open, the adoption is also torn down automatically by
 * `dropScope(c, scope)` — the same lifecycle a plain `bind` gets.
 */
export function useStore<T extends Record<string, any>, K extends keyof T>(
  c: Context,
  i: number,
  store: Store<T>,
  key: K
): Unsubscribe;

/**
 * derivedStore(store, deps, fn) — a read-only value derived from one or more
 * fields of `store`, lazily recomputed and memoized (like `computed`), but
 * living at module scope. `deps` is the list of store keys it reads.
 * Returns a field-shaped handle: place it under a createStore() key to let a
 * component `useStore` it, or `subscribe` to it directly from plain JS.
 */
export function derivedStore<T extends Record<string, any>, R>(
  store: Store<T>,
  deps: (keyof T)[],
  fn: () => R
): StoreField<R> & { stop(): void };
