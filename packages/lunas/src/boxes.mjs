// boxes.mjs — per-variable reactive boxes, specialized by the compiler.
// See output-design.md §4 "Per-variable box specialization".
//
// ES2015 + Proxy (the runtime's compatibility floor). No BigInt.

import { markVar } from "./core.mjs";

// Marker used to unwrap a fine-grained `:for` element proxy back to its raw
// target. Reading `proxy[RAW]` returns the underlying object; on any non-proxy
// (or a plain value) it is `undefined`. Lets forBlock key its raw->handle map by
// stable raw identity even though `items()` yields element proxies.
export const RAW = Symbol("lunas.raw");

// rawOf(x) — the raw object behind a fine-grained element proxy, or x itself.
export function rawOf(x) {
  if (x !== null && typeof x === "object") {
    const r = x[RAW];
    if (r !== undefined) return r;
  }
  return x;
}

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
//
// Fine-grained `:for` tracking (opt-in): a deepBox used as a `:for` source can
// have a `forBlock` attach itself via `box.observeElems()`. Once observed, the
// box records — between flushes — whether the mutation was STRUCTURAL (a write
// on the array itself: `arr[i] = x`, `arr.length = n`, `push`/`splice`/…, or a
// whole-value reassign `box.v = x`) or a pure ELEMENT-FIELD write (a nested
// mutation of an object that is a direct element of the array, e.g.
// `arr[i].label = x`). Both still `markVar` so every dependent flushes; the
// forBlock consults `box._struct` / `box._elems` to patch just the touched
// items instead of running a full reconcile when nothing structural changed.
export function deepBox(c, i, v) {
  const notify = () => markVar(c, i);
  // Fine-grained state (null until a forBlock opts in via observeElems()).
  const self = {
    get v() {
      return px;
    },
    set v(x) {
      if (x !== v) {
        v = x;
        px = wrap(x);
        if (self._track) self._struct = true; // whole-value reassign is structural
        notify();
      }
    },
    // --- fine-grained :for support (all no-cost until observeElems runs) ---
    _track: false, // observing element-field vs structural changes
    _struct: false, // a structural change occurred since last clear
    _elems: null, // Set<rawElement> field-mutated since last clear
    observeElems() {
      if (!this._track) {
        this._track = true;
        this._elems = new Set();
        // Re-wrap so the root array's proxy uses the tracking handler and
        // remembers which nested objects are direct array elements.
        px = wrap(v);
      }
      return this;
    },
    _clear() {
      this._struct = false;
      if (this._elems) this._elems.clear();
    },
  };
  const onStruct = () => {
    if (self._track) self._struct = true;
    notify();
  };
  const onElem = (rawEl) => {
    if (self._elems) self._elems.add(rawEl);
    notify();
  };
  const wrap = makeWrap(notify, {
    isRoot: () => self._track,
    onStruct,
    onElem,
  });
  let px = wrap(v);
  return self;
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
export function makeWrap(notify, fine) {
  const cache = new WeakMap(); // raw object -> proxy
  // Fine-grained :for hooks (optional). When active (fine.isRoot() true), the
  // ROOT array's own mutations route to fine.onStruct(); each direct element of
  // the array, and the whole subtree beneath it, routes nested field writes to
  // fine.onElem(rawTopLevelElement). This lets a forBlock patch only the touched
  // items on a field-only update instead of running a full reconcile.
  const fineActive = () => fine != null && fine.isRoot();

  // Element-subtree handler: mutations attribute to `owner` (the direct array
  // element at the root of this subtree). Reads keep the same owner so deeper
  // objects (`arr[i].sub.field = x`) still mark `arr[i]`.
  const elemHandler = (owner) => ({
    get(t, k, r) {
      if (k === RAW) return t;
      const val = Reflect.get(t, k, r);
      return val !== null && typeof val === "object" ? wrapElem(val, owner) : val;
    },
    set(t, k, x, r) {
      const had = k in t;
      const old = t[k];
      const ok = Reflect.set(t, k, x, r);
      if (ok && (!had || old !== x)) fine.onElem(owner);
      return ok;
    },
    deleteProperty(t, k) {
      const had = k in t;
      const ok = Reflect.deleteProperty(t, k);
      if (ok && had) fine.onElem(owner);
      return ok;
    },
  });
  const wrapElem = (val, owner) => {
    if (val === null || typeof val !== "object") return val;
    if (isCollection(val)) return wrap(val); // collections keep coarse semantics
    let px = elemCache.get(val);
    if (!px) {
      px = new Proxy(val, elemHandler(owner));
      elemCache.set(val, px);
    }
    return px;
  };
  const elemCache = new WeakMap(); // raw element-subtree object -> elem proxy

  const handler = {
    get(t, k, r) {
      const val = Reflect.get(t, k, r);
      if (val === null || typeof val !== "object") return val;
      // Root array element read under fine-grained tracking: wrap so its nested
      // field writes attribute to that element (owner = the raw element itself).
      if (fineActive() && Array.isArray(t) && typeof k !== "symbol") {
        return wrapElem(val, val);
      }
      return wrap(val);
    },
    set(t, k, x, r) {
      const had = k in t;
      const old = t[k];
      const ok = Reflect.set(t, k, x, r);
      if (ok && (!had || old !== x)) {
        // A write on the root array itself (index assignment, length, push/…)
        // is structural. Deeper objects use elemHandler, so this handler's
        // target `t` under fine mode is only ever the root array.
        if (fineActive() && Array.isArray(t)) fine.onStruct();
        else notify();
      }
      return ok;
    },
    deleteProperty(t, k) {
      const had = k in t;
      const ok = Reflect.deleteProperty(t, k);
      if (ok && had) {
        if (fineActive() && Array.isArray(t)) fine.onStruct();
        else notify();
      }
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
