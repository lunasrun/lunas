// provide.d.ts — types for src/provide.mjs
// Dependency injection down the component tree (c-provide).

import type { Context } from "./core.js";

/** A provide/inject key: a string or a Symbol. */
export type InjectionKey = string | symbol;

/** provide(c, key, value) — register `key → value` on this component's context.
 *  Returns `value`. */
export function provide<T>(c: Context, key: InjectionKey, value: T): T;

/** inject(c, key, def?) — resolve `key` from the nearest ancestor that provided
 *  it (self included), else return `def` (default `undefined`). */
export function inject<T = unknown>(
  c: Context,
  key: InjectionKey,
  def?: T
): T | undefined;

/** hasInjection(c, key) — whether any ancestor (or self) provides `key`. Lets a
 *  caller distinguish "provided undefined" from "not provided". */
export function hasInjection(c: Context, key: InjectionKey): boolean;
