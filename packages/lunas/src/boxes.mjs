// boxes.mjs — per-variable reactive boxes, specialized by the compiler.
// See output-design.md §4 "Per-variable box specialization".
//
// ES2015 + Proxy (the runtime's compatibility floor). No BigInt.

import { markVar } from "./core.mjs";

// Marker to unwrap a fine-grained `:for` element proxy back to its raw target:
// reading `proxy[RAW]` returns the underlying object. Kept for callers that hold
// an element proxy and need stable raw identity (the forBlock itself iterates
// the box's raw array directly and does not need it on the hot path).
export const RAW = Symbol("lunas.raw");

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
        if (self._track) {
          self._struct = true; // whole-value reassign is structural
          wrap.markRoot(x);
        }
        px = wrap(x);
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
        fineHooks.active = true;
        this._elems = new Set();
        // Register the current root array so its own mutations are structural,
        // then re-wrap so reads populate owner attribution.
        wrap.markRoot(v);
        px = wrap(v);
      }
      return this;
    },
    _clear() {
      this._struct = false;
      if (this._elems) this._elems.clear();
    },
    // The underlying RAW current value (the unwrapped array). forBlock iterates
    // this for keying/patching so the hot reconcile path never reads through the
    // element proxies; field-write detection still runs when USER code mutates
    // via `.v` (the proxy).
    _raw() {
      return v;
    },
  };
  const fineHooks = {
    active: false,
    onStruct() {
      self._struct = true;
      notify();
    },
    onElem(rawEl) {
      if (self._elems) self._elems.add(rawEl);
      notify();
    },
  };
  const wrap = makeWrap(notify, fineHooks);
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
  // Fine-grained :for tracking (optional, `fine` present). When active, the
  // ROOT array proxy's own mutations route to fine.onStruct() (structural), and
  // a nested field write attributes to the direct array element it lives under,
  // via fine.onElem(rawElement). Owner attribution uses a single WeakMap
  // (raw subtree object -> owning array element), populated on read, so the
  // hot get/set traps stay MONOMORPHIC (one shared handler, no per-element
  // handler allocation) — matching the non-fine cost as closely as possible.
  // `fine` is a live hooks object with a mutable `active` flag (false until a
  // forBlock calls observeElems). While inactive the traps behave exactly like
  // the non-fine path — one boolean read of overhead — so deepBoxes not used as
  // a fine `:for` source pay essentially nothing.
  const owners = fine ? new WeakMap() : null; // raw obj -> raw owning element
  const roots = fine ? new WeakSet() : null; // the diffed root array(s)

  const handler = {
    get(t, k, r) {
      if (fine && fine.active) {
        if (k === RAW) return t;
        const val = Reflect.get(t, k, r);
        if (val === null || typeof val !== "object") return val;
        if (roots.has(t)) {
          if (typeof k !== "symbol") owners.set(val, val); // direct element owns itself
        } else {
          const o = owners.get(t);
          if (o !== undefined && !owners.has(val)) owners.set(val, o);
        }
        return wrap(val);
      }
      const val = Reflect.get(t, k, r);
      return val !== null && typeof val === "object" ? wrap(val) : val;
    },
    set(t, k, x, r) {
      const had = k in t;
      const old = t[k];
      const ok = Reflect.set(t, k, x, r);
      if (ok && (!had || old !== x)) {
        if (fine && fine.active) {
          if (roots.has(t)) fine.onStruct();
          else {
            const o = owners.get(t);
            if (o !== undefined) fine.onElem(o);
            else notify();
          }
        } else notify();
      }
      return ok;
    },
    deleteProperty(t, k) {
      const had = k in t;
      const ok = Reflect.deleteProperty(t, k);
      if (ok && had) {
        if (fine && fine.active) {
          if (roots.has(t)) fine.onStruct();
          else {
            const o = owners.get(t);
            if (o !== undefined) fine.onElem(o);
            else notify();
          }
        } else notify();
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
  // markRoot(rawArray) — register the diffed root array so its own mutations are
  // structural. Called by the box for the current `.v` value (fine mode only).
  wrap.markRoot = (val) => {
    if (roots && val !== null && typeof val === "object") roots.add(val);
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
