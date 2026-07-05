// emits.d.ts — types for src/emits.mjs
// Child → parent events (c-emits).

import type { Context } from "./core.js";

/** eventPropName("save") → "onSave"; ("save-all") → "onSaveAll". The `@name`
 *  → `onName` mapping the codegen uses for component-tag event listeners. */
export function eventPropName(name: string): string;

/** registerEmits(c, props, declared?) — stash the child's props so `emit` can
 *  find `on<Name>` handlers; optionally record declared event names for lean
 *  (warn-only) validation. Called at the top of a child `setup`. Returns `c`. */
export function registerEmits<P = Record<string, unknown>>(
  c: Context,
  props: P,
  declared?: string[]
): Context;

/** emit(c, name, payload?) — invoke the parent's `on<Name>` handler if present.
 *  Returns true if a handler ran. Never marks the parent dirty by itself — the
 *  handler decides whether to mutate parent state. */
export function emit(c: Context, name: string, payload?: unknown): boolean;
