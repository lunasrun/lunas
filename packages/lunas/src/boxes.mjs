// boxes.mjs — per-variable reactive boxes, specialized by the compiler.
// See output-design.md §4 "Per-variable box specialization".
//
// ES2015+. No Proxy, no BigInt. Deep mutation is made reactive by explicit
// compiler-injected invalidation (`box.touch()` / `box.touchElem(el)`) emitted
// right after each mutating statement — the Svelte-family model — instead of a
// runtime Proxy intercepting every nested get/set. Reads return the RAW value,
// so the hot path pays nothing.

import { markVar } from "./core.mjs";

// box(c, i, v) — reassign-only variable at reactive index i.
// Lightest path: plain getter/setter, no Proxy. Same-value writes are no-ops.
export function box(c, i, v) {
  return {
    get v() {
      return v;
    },
    set v(x) {
      if (x !== v) {
        v = x;
        markVar(c, i);
      }
    },
  };
}

// deepBox(c, i, v) — deeply-mutated variable (arr.push, obj.k = …, nested
// field writes, Map/Set mutations).
//
// Proxy-free (Svelte-family model): `.v` returns the RAW value, so reads are as
// cheap as a plain `box`. Deep mutation is made reactive by an explicit
// invalidation call the compiler injects immediately after each mutating
// statement:
//   - `box.touch()`       — a STRUCTURAL deep mutation (`arr.push(x)`,
//                           `arr[i] = y`, `arr.length = n`, `obj.k = v`,
//                           `delete obj.k`, Map/Set `set/add/delete/clear`, or a
//                           whole-value reassign — the setter marks that one).
//   - `box.touchElem(el)` — an ELEMENT-FIELD mutation of a direct array element
//                           (`arr[i].label = x`): `el` is the mutated element.
// Both call `markVar`, so every dependent flushes. `markVar` defers the flush to
// a microtask, so it does not matter that the touch runs before the mutation
// completes within the same synchronous statement.
//
// Fine-grained `:for` tracking (opt-in): a deepBox used as a `:for` source has
// a `forBlock` attach via `box.observeElems()`. Once observed, the box records —
// between flushes — whether a STRUCTURAL change occurred (`touch()` / reassign)
// or only ELEMENT-FIELD writes (`touchElem(el)`), so the forBlock can patch just
// the touched items instead of a full reconcile. This is the same `_struct` /
// `_elems` contract the Proxy version exposed; only the source of the signal
// changed (explicit calls, not trap interception).
export function deepBox(c, i, v) {
  const notify = () => markVar(c, i);
  const self = {
    get v() {
      return v;
    },
    set v(x) {
      if (x !== v) {
        v = x;
        if (self._track) self._struct = true; // whole-value reassign is structural
        notify();
      }
    },
    // touch() — a structural deep mutation happened on the current value.
    // Returns the raw value so a compiler-injected `(box.touch(), expr)` prefix
    // never disturbs the surrounding expression's own value.
    touch() {
      if (self._track) self._struct = true;
      notify();
      return v;
    },
    // touchElem(el) — a field of the direct array element `el` was mutated.
    // Records `el` for fine-grained patching (when observed) and marks dirty.
    // Returns `el` so it composes in a comma-prefixed injection.
    touchElem(el) {
      if (self._elems) self._elems.add(el);
      notify();
      return el;
    },
    // --- fine-grained :for support (all no-cost until observeElems runs) ---
    _track: false, // observing element-field vs structural changes
    _struct: false, // a structural change occurred since last clear
    _elems: null, // Set<rawElement> field-mutated since last clear
    observeElems() {
      if (!this._track) {
        this._track = true;
        this._elems = new Set();
      }
      return this;
    },
    _clear() {
      this._struct = false;
      if (this._elems) this._elems.clear();
    },
    // The RAW current value. forBlock iterates this for keying/patching.
    _raw() {
      return v;
    },
  };
  return self;
}

// prop(c, i, raw, def) — adopt an `@input` prop as a reactive variable at
// index i (output-design.md §6). The child reads it as a box (`.v`), so its
// own template binds react when the prop changes. The seed is `raw` when the
// parent passed a value, else the compiled default `def`. A getter-valued
// `raw` (the parent passes `() => expr` for a reactive prop) is invoked once
// to seed; the parent keeps it live by pushing new values through the
// mountChild handle's setProp, which writes `child._props[name].v`.
//
// The box is registered under `name` in `c._props` so a parent's mountChild
// can find and drive it. `deep` selects a deepBox (the child deeply mutates
// the prop locally) — parent-driven whole-value replacement still works.
export function prop(c, name, i, raw, def, deep) {
  const seed = raw === undefined ? def : typeof raw === "function" ? raw() : raw;
  const b = deep ? deepBox(c, i, seed) : box(c, i, seed);
  (c._props || (c._props = {}))[name] = b;
  return b;
}

// shared(v) — a value shared across components (prop passed down and
// mutated). Each dependent component attaches with its own reactive index;
// a write marks the variable dirty in EVERY attached component.
export function shared(v) {
  const subs = []; // [{ c, i }]
  const notify = () => {
    for (const s of subs) markVar(s.c, s.i);
  };
  return {
    get v() {
      return v;
    },
    set v(x) {
      if (x !== v) {
        v = x;
        notify();
      }
    },
    attach(c, i) {
      subs.push({ c, i });
    },
    detach(c) {
      for (let k = subs.length - 1; k >= 0; k--) {
        if (subs[k].c === c) subs.splice(k, 1);
      }
    },
  };
}
