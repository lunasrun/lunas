// store.mjs — module-level reactive state living outside any component.
// See output-design.md §4 "Shared across components" and §5 (store block).
//
// A store is the module-scope generalization of boxes.mjs's `shared`: instead
// of one value shared by components that hold a reference to it (props), a
// store is created once at module load and imported by however many
// components want it — same adjacency-dispatch contract, no auto-tracking,
// no runtime dependency discovery. Each store field is independently
// subscribable; a write to one field only marks dirty the components that
// adopted *that* field.
//
// ES2015 + Proxy (the runtime's compatibility floor). No BigInt.

import { markVar } from "./core.mjs";
import { makeWrap } from "./boxes.mjs";

// isField(x) — true for anything shaped like a storeField/derivedStore
// output (attach/detach/subscribe). Lets createStore accept a derivedStore
// result under a key without double-wrapping it.
function isField(x) {
  return (
    x !== null &&
    typeof x === "object" &&
    typeof x.attach === "function" &&
    typeof x.detach === "function" &&
    typeof x.subscribe === "function"
  );
}

// storeField(v) — one named slot of a store. Shaped like boxes.mjs's
// `shared`: `.v` get/set, `attach(c, i)` / `detach(c)` for component adoption,
// PLUS deep-mutation support (object/array values are lazily Proxy-wrapped,
// same cached-wrapper strategy as deepBox) and a `subscribe(fn)` for plain-JS
// consumers that are not component contexts at all (router, devtools, tests).
//
// Returns [field, notify]: `notify(value?)` is kept internal to this module
// (used directly by derivedStore to signal its own subscribers without a
// fake write) rather than exposed on the public field shape. When called
// with no argument it reports the field's own current `v` (the normal write
// path); derivedStore passes its freshly computed value explicitly since it
// never assigns through `field.v`.
function storeField(v) {
  const subs = []; // [{ c, i }] — attached component (context, reactive index) pairs
  const listeners = new Set(); // fn(value) — plain-JS subscribers

  const notify = (...value) => {
    for (const s of subs) markVar(s.c, s.i);
    for (const fn of listeners) fn(value.length ? value[0] : v);
  };
  const wrap = makeWrap(notify);
  let px = wrap(v);

  const field = {
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
    // attach(c, i) — adopt this field at component context `c`'s reactive
    // index `i`: from now on, writes to this field mark index `i` dirty in
    // `c` (batched per the normal microtask flush, like any other var).
    attach(c, i) {
      subs.push({ c, i });
    },
    // detach(c) — remove every attachment belonging to context `c` (called on
    // component/scope teardown so a torn-down component stops being notified).
    detach(c) {
      for (let k = subs.length - 1; k >= 0; k--) {
        if (subs[k].c === c) subs.splice(k, 1);
      }
    },
    subscribe(fn) {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
  };
  return [field, notify];
}

// createStore(initial) — create a module-level store from a plain object of
// named initial values. Each key becomes an independent field (its own subs
// list), so a write to one field never touches components that only adopted
// another field. A value that is already field-shaped (e.g. the result of
// derivedStore()) is kept as-is instead of being wrapped in a plain field, so
// derived values can be declared inline in the initial object.
export function createStore(initial) {
  const fields = new Map();
  for (const k in initial) {
    const v = initial[k];
    fields.set(k, isField(v) ? v : storeField(v)[0]);
  }

  function field(key) {
    let f = fields.get(key);
    if (!f) {
      f = storeField(undefined)[0];
      fields.set(key, f);
    }
    return f;
  }

  return {
    // get(key) — current value of `key` (through the deep-mutation proxy if
    // the value is an object/array). Safe to call from anywhere, component or
    // plain module code.
    get(key) {
      return field(key).v;
    },
    // set(key, v) — write `key`, notifying every component that adopted it
    // (batched per the normal microtask flush) and every plain-JS subscriber.
    // Same-value writes are no-ops, like box/deepBox/shared. Throws if `key`
    // holds a derived (read-only) value.
    set(key, v) {
      field(key).v = v;
    },
    // subscribe(key, fn) — outside-component subscription for plain-JS
    // consumers (router, devtools, tests). `fn(value)` runs synchronously on
    // every write to `key` (not batched — there is no component flush to ride
    // for a non-component listener). Returns an unsubscribe function.
    subscribe(key, fn) {
      return field(key).subscribe(fn);
    },
    // Internal: exposes the raw field handle for useStore/derivedStore. Not
    // part of the intended public surface (hence no doc-comment call-out in
    // README/types), but plain and unhidden since this module has no private
    // WeakMaps to enforce it — callers should prefer get/set/subscribe.
    _field: field,
  };
}

// useStore(c, i, store, key) — adopt store field `key` at component context
// `c`'s reactive index `i`. This is the mechanical shape the compiler emits
// for a component that reads (or reads+writes) a store field: exactly one
// call per (component, field) adoption, analogous to `shared(...).attach(c,
// i)` but sourced from a module-level store instead of a passed-down prop.
//
// Intended emitted shape (compiler-facing contract):
//
//   // module scope, compiled once from a `store {...}` declaration:
//   export const appStore = createStore({ count: 0, user: null });
//
//   // inside a component's setup(c, props), for each store field the
//   // component's template/handlers reference at reactive index i:
//   useStore(c, i, appStore, "count");
//   // template/handler code then reads/writes via:
//   appStore.get("count")                              // read (declares i)
//   appStore.set("count", appStore.get("count") + 1)   // write
//
// Returns a detach() that undoes the adoption; calling it more than once is
// a no-op. When `useStore` is called while `c.scope` is open (i.e. from
// inside a control-flow block's `make()`), the adoption also registers
// itself onto that scope, so `dropScope(c, scope)` calls detach()
// automatically when the block's content is torn down — the same lifecycle
// a plain `bind` gets. Outside a scope (e.g. adopted once in a component's
// top-level setup), the caller is expected to invoke the returned detach()
// from whatever future whole-component-unmount hook the runtime grows;
// until then, the store keeps a live reference for that component's lifetime.
export function useStore(c, i, store, key) {
  const f = store._field(key);
  f.attach(c, i);
  let live = true;
  const detach = () => {
    if (live) {
      live = false;
      f.detach(c);
    }
  };
  if (c.scope) {
    // A scope-drop record: dropScope() calls unbind(c, s) on every entry in
    // scope.subs, and unbind()'s only externally-visible effect is setting
    // `s.alive = false` (it also walks s.deps, harmless here since deps is
    // empty). Piggyback on that one write to run our own detach — no changes
    // to core.mjs's scope contract needed.
    c.scope.subs.push({
      deps: [],
      get alive() {
        return live;
      },
      set alive(_v) {
        detach();
      },
    });
  }
  return detach;
}

// derivedStore(store, deps, fn) — a read-only value derived from one or more
// fields of `store`, lazily recomputed and memoized exactly like computed.mjs's
// `computed`, but living at module scope instead of a component's reactive
// index. `deps` is the list of store keys it reads.
//
// Kept intentionally thin: it reuses storeField's attach/detach/subscribe
// machinery for its OWN output (so a derived value can be adopted into a
// component via `useStore(c, i, store, key)` once placed under a key — see
// createStore's field-shaped-value passthrough — or subscribed to directly
// from plain JS) plus computed.mjs's staleness policy for the upstream read
// (recompute deferred to the next `.v` read after any dep changes, never
// eager).
//
//   const cart = createStore({ items: [] });
//   const total = derivedStore(cart, ["items"], () =>
//     cart.get("items").reduce((sum, it) => sum + it.price, 0)
//   );
//   const app = createStore({ total }); // field-shaped value passes through
//   useStore(c, i, app, "total");       // component adopts the derived value
export function derivedStore(store, deps, fn) {
  let value;
  let stale = true;
  const [out, notifyOut] = storeField(undefined);
  const recompute = () => {
    value = fn();
    stale = false;
    return value;
  };
  const invalidate = () => {
    // Mark stale for component reads (kept lazy: a component's own bind
    // reads `.v` on its own schedule), but recompute right away to hand
    // plain-JS subscribers the fresh value synchronously, same as every
    // other subscribe() in this module (shared/store fields notify with the
    // value already updated, never a "please recompute yourself" signal).
    stale = true;
    notifyOut(recompute());
  };
  const unsubs = deps.map((k) => store._field(k).subscribe(invalidate));

  return {
    get v() {
      if (stale) recompute();
      return value;
    },
    attach(c, i) {
      out.attach(c, i);
    },
    detach(c) {
      out.detach(c);
    },
    subscribe(fn2) {
      return out.subscribe(fn2);
    },
    // stop() — unsubscribe from every upstream field. Rarely needed (derived
    // stores are normally module-scoped for the app's lifetime) but provided
    // for symmetry with watch/watchEffect's stop().
    stop() {
      for (const u of unsubs) u();
    },
  };
}
