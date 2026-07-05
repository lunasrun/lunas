// boxes.mjs — per-variable reactive boxes, specialized by the compiler.
// See output-design.md §4 "Per-variable box specialization".
//
// ES2015 + Proxy (the runtime's compatibility floor). No BigInt.

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

// deepBox(c, i, v) — deeply-mutated variable (arr.push, obj.k = …).
// Reads through .v return a Proxy that marks the variable dirty on any
// nested set/delete. Nested objects are wrapped lazily on property access;
// wrappers are cached per underlying object so identity is stable.
// Mutating array methods (push/splice/…) work through the set trap because
// they run with the proxy as `this`.
// Map/Set (and WeakMap/WeakSet) values are collection-aware: reads and methods
// run against the real collection (native internal slots reject a foreign
// receiver), and mutating ops (Map set/delete/clear, Set add/delete/clear)
// mark the variable dirty. Values stored inside a collection are not deeply
// wrapped — reassign an entry to make a change reactive.
export function deepBox(c, i, v) {
  const notify = () => markVar(c, i);
  const wrap = makeWrap(notify);
  let px = wrap(v);
  return {
    get v() {
      return px;
    },
    set v(x) {
      if (x !== v) {
        v = x;
        px = wrap(x);
        notify();
      }
    },
  };
}

// makeWrap(notify) — build a lazy, cached deep-Proxy wrapper that calls
// `notify` on any nested set/delete. Exported so other modules that need the
// same "deeply-mutated value" semantics (e.g. store.mjs's per-field deep
// mutation support) don't have to reimplement the Proxy handler.
//
// Collections (Map/Set) get a dedicated get-trap path: their accessors and
// methods have internal slots ([[MapData]]/[[SetData]]) that reject a foreign
// receiver, so we must run them against the REAL target rather than the proxy.
// Mutating collection methods (Map set/delete/clear, Set add/delete/clear) are
// wrapped to perform the op then `notify()`, so bindings that read the
// collection (`.size`, `.get`, iteration, `.has`, …) re-run. Reads never mark.
//
// Values stored inside a Map/Set are NOT deeply wrapped: only collection-level
// membership mutations are reactive. Mutating an object retrieved from a
// collection (`map.get(k).field = …`) does not mark the box — reassign the
// entry (`map.set(k, next)`) to trigger reactivity. Keeping values raw avoids
// proxy-identity hazards with `has`/`get`/key lookups and keeps semantics
// honest and simple.
export function makeWrap(notify) {
  const cache = new WeakMap(); // raw object -> proxy
  const handler = {
    get(t, k, r) {
      const val = Reflect.get(t, k, r);
      return val !== null && typeof val === "object" ? wrap(val) : val;
    },
    set(t, k, x, r) {
      const had = k in t;
      const old = t[k];
      const ok = Reflect.set(t, k, x, r);
      if (ok && (!had || old !== x)) notify();
      return ok;
    },
    deleteProperty(t, k) {
      const had = k in t;
      const ok = Reflect.deleteProperty(t, k);
      if (ok && had) notify();
      return ok;
    },
  };
  // Collection handler: bind everything to the real target so native internal
  // slots accept the receiver; wrap mutators to notify after the op.
  const collectionHandler = {
    get(t, k) {
      const val = Reflect.get(t, k, t);
      if (typeof val !== "function") return val;
      if (MUTATORS.has(k) && MUTATORS.get(k)(t)) {
        return function (...args) {
          const ret = val.apply(t, args);
          notify();
          return ret;
        };
      }
      // Non-mutating method (get/has/forEach/keys/values/entries/…): bind so
      // the native internal-slot check sees the real collection as receiver.
      return val.bind(t);
    },
  };
  const wrap = (val) => {
    if (val === null || typeof val !== "object") return val;
    let px = cache.get(val);
    if (!px) {
      px = new Proxy(val, isCollection(val) ? collectionHandler : handler);
      cache.set(val, px);
    }
    return px;
  };
  return wrap;
}

// Mutating collection method names -> predicate that reports whether the method
// is truly mutating on THIS target. `delete` and `clear` are shared names
// across Map/Set (both mutate); `set` mutates on Map but is a WeakSet non-op /
// absent elsewhere, and `add` mutates on Set/WeakSet. The predicate guards
// against, e.g., a `set` key on an unrelated wrapped object slipping through —
// though collectionHandler only ever wraps real Map/Set/WeakMap/WeakSet.
const MUTATORS = new Map([
  ["set", (t) => t instanceof Map || t instanceof WeakMap],
  ["add", (t) => t instanceof Set || t instanceof WeakSet],
  ["delete", () => true],
  ["clear", () => true],
]);

function isCollection(v) {
  return (
    v instanceof Map ||
    v instanceof Set ||
    v instanceof WeakMap ||
    v instanceof WeakSet
  );
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
