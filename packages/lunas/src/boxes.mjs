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
  const wrap = (val) => {
    if (val === null || typeof val !== "object") return val;
    let px = cache.get(val);
    if (!px) {
      px = new Proxy(val, handler);
      cache.set(val, px);
    }
    return px;
  };
  return wrap;
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
