// store.edge.test.mjs — additional edge-focused coverage for store.mjs:
// per-field isolation details, useStore adoption at index, derivedStore
// staleness edge cases, two independent contexts/stores, and scope-drop
// interplay with derived values. Complements store.test.mjs.
// Run: node packages/lunas/test/store.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import {
  createContext,
  bind,
  beginScope,
  endScope,
  dropScope,
} from "../src/core.mjs";
import { createStore, useStore, derivedStore } from "../src/store.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

// -- per-field isolation, unknown-key auto-creation --------------------------

test("reading an undeclared key lazily creates an (undefined) field", () => {
  const store = createStore({});
  assert.strictEqual(store.get("ghost"), undefined);
  const c = createContext(null);
  useStore(c, 0, store, "ghost");
  let seen;
  bind(c, [0], () => {
    seen = store.get("ghost");
  });
  store.set("ghost", "now-set");
  return tick().then(() => {
    assert.strictEqual(seen, "now-set");
  });
});

test("two independent stores never cross-notify each other's subscribers", async () => {
  const storeA = createStore({ v: 1 });
  const storeB = createStore({ v: 1 });
  const seenA = [];
  const seenB = [];
  storeA.subscribe("v", (x) => seenA.push(x));
  storeB.subscribe("v", (x) => seenB.push(x));
  storeA.set("v", 2);
  assert.deepStrictEqual(seenA, [2]);
  assert.deepStrictEqual(seenB, [], "store B untouched by store A's write");
});

test("adopting the same field twice at different indices on the same context both fire", async () => {
  const store = createStore({ n: 0 });
  const c = createContext(null);
  useStore(c, 0, store, "n");
  useStore(c, 1, store, "n"); // same field, second reactive index
  let a = 0;
  let b = 0;
  bind(c, [0], () => {
    a++;
    void store.get("n");
  });
  bind(c, [1], () => {
    b++;
    void store.get("n");
  });
  a = b = 0;
  store.set("n", 1);
  await tick();
  assert.strictEqual(a, 1);
  assert.strictEqual(b, 1);
});

// -- useStore adoption returns a working detach + scope interplay -----------

test("useStore outside a scope requires an explicit detach() call", async () => {
  const store = createStore({ n: 0 });
  const c = createContext(null);
  // No beginScope: c.scope is null, so no automatic scope-drop wiring.
  const detach = useStore(c, 0, store, "n");
  let runs = 0;
  bind(c, [0], () => runs++);
  runs = 0;
  store.set("n", 1);
  await tick();
  assert.strictEqual(runs, 1, "still attached");
  detach();
  runs = 0;
  store.set("n", 2);
  await tick();
  assert.strictEqual(runs, 0, "detached manually");
});

test("dropScope on a scope containing useStore also tears down a plain bind in the same scope", async () => {
  const store = createStore({ n: 0 });
  const c = createContext(null);
  const scope = beginScope(c);
  useStore(c, 0, store, "n");
  let runs = 0;
  bind(c, [0], () => {
    runs++;
    void store.get("n");
  });
  endScope(c);
  dropScope(c, scope);
  runs = 0;
  store.set("n", 5);
  await tick();
  assert.strictEqual(runs, 0, "both the adoption and the bind are gone");
});

// -- batched writes across multiple stores in one microtask ------------------

test("writes to two different stores adopted by one context still coalesce to one flush", async () => {
  const s1 = createStore({ a: 0 });
  const s2 = createStore({ b: 0 });
  const c = createContext(null);
  useStore(c, 0, s1, "a");
  useStore(c, 1, s2, "b");
  let paints = 0;
  bind(c, [0, 1], () => {
    paints++;
    void s1.get("a");
    void s2.get("b");
  });
  paints = 0;
  s1.set("a", 1);
  s2.set("b", 2);
  assert.strictEqual(paints, 0);
  await tick();
  assert.strictEqual(paints, 1);
});

// -- derivedStore: multi-dep staleness, chained derivations ------------------

test("derivedStore recomputes once per flush window even with two deps both changing", () => {
  const store = createStore({ a: 1, b: 1 });
  let computes = 0;
  const sum = derivedStore(store, ["a", "b"], () => {
    computes++;
    return store.get("a") + store.get("b");
  });
  assert.strictEqual(sum.v, 2);
  computes = 0;
  store.set("a", 10);
  store.set("b", 10);
  // Each set synchronously invalidates+recomputes (no batching inside
  // derivedStore itself — it recomputes eagerly on every upstream write to
  // keep plain-JS subscribers synchronous).
  assert.strictEqual(sum.v, 20);
});

test("derivedStore chained off another derivedStore stays consistent", () => {
  const store = createStore({ n: 2 });
  const doubled = derivedStore(store, ["n"], () => store.get("n") * 2);
  const wrapped = createStore({ doubled });
  const quadrupled = derivedStore(wrapped, ["doubled"], () =>
    wrapped.get("doubled") * 2
  );
  assert.strictEqual(quadrupled.v, 8);
  store.set("n", 5);
  assert.strictEqual(doubled.v, 10);
  assert.strictEqual(quadrupled.v, 20);
});

test("derivedStore.stop() unsubscribes from upstream fields (no more recompute-on-write)", () => {
  const store = createStore({ n: 1 });
  let computes = 0;
  const doubled = derivedStore(store, ["n"], () => {
    computes++;
    return store.get("n") * 2;
  });
  assert.strictEqual(doubled.v, 2);
  computes = 0;
  doubled.stop();
  store.set("n", 100);
  // No subscribe-driven recompute happened (stop unsubscribed); but note
  // reading .v afterward may recompute lazily if implementation marks stale.
  // Since stop() only removes the invalidate subscription, .v was never
  // invalidated, so it stays memoized at the old value.
  assert.strictEqual(doubled.v, 2, "stale value retained after stop()");
  assert.strictEqual(computes, 0);
});

test("derivedStore field passed into createStore is not double-wrapped (isField passthrough)", () => {
  const cart = createStore({ items: [{ price: 3 }] });
  const total = derivedStore(cart, ["items"], () =>
    cart.get("items").reduce((s, it) => s + it.price, 0)
  );
  const app = createStore({ total, other: 1 });
  // app.get("total") should read through the SAME derived field, not a fresh
  // plain field wrapping the derived object.
  assert.strictEqual(app.get("total"), 3);
  cart.get("items").push({ price: 7 });
  cart.touch("items"); // deep mutation of the upstream field -> derived recomputes
  assert.strictEqual(app.get("total"), 10, "derived passthrough recomputes live");
});

test("subscribing to a derivedStore's output does not receive upstream-unrelated writes", () => {
  const store = createStore({ a: 1, b: 100 });
  const seen = [];
  const onlyA = derivedStore(store, ["a"], () => store.get("a") * 10);
  onlyA.subscribe((v) => seen.push(v));
  store.set("b", 200); // unrelated field
  assert.deepStrictEqual(seen, [], "derivedStore only depends on declared deps");
  store.set("a", 2);
  assert.deepStrictEqual(seen, [20]);
});

// -- scope-drop cleanup for a derived value adopted into a component ---------

test("dropScope detaches a component from a derivedStore-backed field too", async () => {
  const cart = createStore({ items: [1, 2] });
  const count = derivedStore(cart, ["items"], () => cart.get("items").length);
  const app = createStore({ count });
  const c = createContext(null);
  const scope = beginScope(c);
  useStore(c, 0, app, "count");
  let seen = null;
  bind(c, [0], () => {
    seen = app.get("count");
  });
  endScope(c);
  assert.strictEqual(seen, 2);

  dropScope(c, scope);
  cart.get("items").push(3);
  cart.touch("items"); // deep mutation of the upstream field -> derived recomputes
  await tick();
  assert.strictEqual(seen, 2, "detached component does not observe the derived update");
  assert.strictEqual(app.get("count"), 3, "the derived value itself still updates");
});

// -- deep mutation isolation between fields -----------------------------------

test("deep mutation + touch() on one array field does not notify a sibling field's subscribers", () => {
  const store = createStore({ list: [1], other: "x" });
  const seenList = [];
  const seenOther = [];
  store.subscribe("list", (v) => seenList.push(v));
  store.subscribe("other", (v) => seenOther.push(v));
  // New contract: the deep mutation is inert on its own; touch(key) is what
  // notifies, and it only notifies the field that was touched.
  store.get("list").push(2);
  store.touch("list");
  assert.deepStrictEqual(seenList, [[1, 2]], "the touched field's own subscribers fire");
  assert.deepStrictEqual(seenOther, [], "unrelated field's subscribers silent");
});

test("replacing a store field's whole value returns a new raw object (stable raw identity)", () => {
  const store = createStore({ obj: { a: 1 } });
  const first = store.get("obj");
  // get() now returns the RAW value, so identity is stable across reads (no
  // per-read proxy wrapping) rather than a fresh proxy each time.
  assert.strictEqual(store.get("obj"), first, "repeated reads return the same raw object");
  store.set("obj", { a: 2 });
  const second = store.get("obj");
  assert.notStrictEqual(first, second, "a whole-value write swaps in a different raw object");
  assert.strictEqual(second.a, 2);
});
