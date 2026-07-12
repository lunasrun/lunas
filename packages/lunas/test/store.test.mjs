// store.test.mjs — module-level reactive stores (createStore/useStore/
// derivedStore).
// Run: node packages/lunas/test/store.test.mjs

import assert from "node:assert";
import {
  createContext,
  bind,
  beginScope,
  endScope,
  dropScope,
} from "../src/core.mjs";
import { createStore, useStore, derivedStore } from "../src/store.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("two independent contexts both re-render on a store write", async () => {
  const store = createStore({ count: 0 });
  const c1 = createContext(null);
  const c2 = createContext(null);
  useStore(c1, 0, store, "count");
  useStore(c2, 3, store, "count"); // different reactive index per component

  let seen1 = null;
  let seen2 = null;
  bind(c1, [0], () => {
    seen1 = store.get("count");
  });
  bind(c2, [3], () => {
    seen2 = store.get("count");
  });
  assert.strictEqual(seen1, 0);
  assert.strictEqual(seen2, 0);

  store.set("count", 5);
  await tick();
  assert.strictEqual(seen1, 5);
  assert.strictEqual(seen2, 5);
});

await test("same-value write is a no-op", async () => {
  const store = createStore({ x: "a" });
  const c = createContext(null);
  useStore(c, 0, store, "x");
  let runs = 0;
  bind(c, [0], () => runs++);
  store.set("x", "a");
  await tick();
  assert.strictEqual(runs, 1, "no flush for identical value");
});

await test("writing one field never touches a component that adopted another field", async () => {
  const store = createStore({ a: 1, b: 1 });
  const c = createContext(null);
  useStore(c, 0, store, "a");
  useStore(c, 1, store, "b");
  let aRuns = 0;
  let bRuns = 0;
  bind(c, [0], () => {
    aRuns++;
    void store.get("a");
  });
  bind(c, [1], () => {
    bRuns++;
    void store.get("b");
  });
  aRuns = 0;
  bRuns = 0;
  store.set("b", 2);
  await tick();
  assert.strictEqual(aRuns, 0, "field a's dependents untouched");
  assert.strictEqual(bRuns, 1);
});

await test("batching: multiple writes to different fields of one context -> one flush", async () => {
  const store = createStore({ a: 0, b: 0 });
  const c = createContext(null);
  useStore(c, 0, store, "a");
  useStore(c, 1, store, "b");
  let paints = 0;
  bind(c, [0, 1], () => {
    paints++;
    void store.get("a");
    void store.get("b");
  });
  paints = 0;
  store.set("a", 1);
  store.set("b", 2);
  store.set("a", 3);
  assert.strictEqual(paints, 0, "nothing painted synchronously");
  await tick();
  assert.strictEqual(paints, 1, "coalesced into a single flush pass");
});

await test("batching: multiple writes to the same field -> one flush", async () => {
  const store = createStore({ n: 0 });
  const c = createContext(null);
  useStore(c, 0, store, "n");
  let runs = 0;
  bind(c, [0], () => runs++);
  runs = 0;
  store.set("n", 1);
  store.set("n", 2);
  store.set("n", 3);
  await tick();
  assert.strictEqual(runs, 1);
  assert.strictEqual(store.get("n"), 3);
});

await test("deep mutation on a store array triggers dependents", async () => {
  const store = createStore({ items: [1, 2] });
  const c = createContext(null);
  useStore(c, 0, store, "items");
  let len = 0;
  bind(c, [0], () => {
    len = store.get("items").length;
  });
  assert.strictEqual(len, 2);
  store.get("items").push(3);
  await tick();
  assert.strictEqual(len, 3);
  assert.deepStrictEqual(Array.from(store.get("items")), [1, 2, 3]);
});

await test("deep mutation on a nested store object triggers dependents", async () => {
  const store = createStore({ user: { name: "a", tags: ["x"] } });
  const c = createContext(null);
  useStore(c, 0, store, "user");
  let name = null;
  bind(c, [0], () => {
    name = store.get("user").name;
  });
  store.get("user").name = "b";
  await tick();
  assert.strictEqual(name, "b");

  let tagCount = 0;
  bind(c, [0], () => {
    tagCount = store.get("user").tags.length;
  });
  store.get("user").tags.push("y");
  await tick();
  assert.strictEqual(tagCount, 2);
});

await test("outside subscribe/unsubscribe: plain-JS consumer sees writes", async () => {
  const store = createStore({ path: "/" });
  const seen = [];
  const unsubscribe = store.subscribe("path", (v) => seen.push(v));
  store.set("path", "/about");
  assert.deepStrictEqual(seen, ["/about"], "subscribe fires synchronously, no component needed");
  unsubscribe();
  store.set("path", "/contact");
  assert.deepStrictEqual(seen, ["/about"], "no callback after unsubscribe");
});

await test("outside subscribe does not require any adopting component", async () => {
  const store = createStore({ count: 0 });
  let last = -1;
  store.subscribe("count", (v) => {
    last = v;
  });
  store.set("count", 42);
  assert.strictEqual(last, 42);
  assert.strictEqual(store.get("count"), 42);
});

await test("scope-drop cleanup: component unmount stops notifications", async () => {
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

  runs = 0;
  store.set("n", 1);
  await tick();
  assert.strictEqual(runs, 1, "still notified while scope alive");

  dropScope(c, scope);
  store.set("n", 2);
  await tick();
  assert.strictEqual(runs, 1, "no notification after dropScope");
  assert.strictEqual(store.get("n"), 2, "store itself still updated");
});

await test("useStore's own detach() is idempotent and stops notifications", async () => {
  const store = createStore({ n: 0 });
  const c = createContext(null);
  const detach = useStore(c, 0, store, "n");
  let runs = 0;
  bind(c, [0], () => runs++);
  runs = 0;
  detach();
  detach(); // idempotent
  store.set("n", 1);
  await tick();
  assert.strictEqual(runs, 0, "detached component not notified");
});

await test("derivedStore: lazy, recomputes only after a dep changes", async () => {
  const store = createStore({ a: 2, b: 3 });
  let computes = 0;
  const sum = derivedStore(store, ["a", "b"], () => {
    computes++;
    return store.get("a") + store.get("b");
  });
  assert.strictEqual(computes, 0, "no read yet -> no compute");
  assert.strictEqual(sum.v, 5);
  assert.strictEqual(sum.v, 5, "memoized");
  assert.strictEqual(computes, 1);

  store.set("a", 10);
  assert.strictEqual(sum.v, 13);
  assert.strictEqual(computes, 2);
});

await test("derivedStore: adopted by a component like a plain field", async () => {
  const cart = createStore({ items: [{ price: 1 }, { price: 2 }] });
  const total = derivedStore(cart, ["items"], () =>
    cart.get("items").reduce((sum, it) => sum + it.price, 0)
  );
  const app = createStore({ total });
  const c = createContext(null);
  useStore(c, 0, app, "total");

  let seen = null;
  bind(c, [0], () => {
    seen = app.get("total");
  });
  assert.strictEqual(seen, 3);

  cart.get("items").push({ price: 5 });
  await tick();
  assert.strictEqual(seen, 8);
});

await test("derivedStore: subscribe sees the fresh recomputed value", async () => {
  const store = createStore({ n: 1 });
  const doubled = derivedStore(store, ["n"], () => store.get("n") * 2);
  const seen = [];
  doubled.subscribe((v) => seen.push(v));
  store.set("n", 4);
  assert.deepStrictEqual(seen, [8]);
});

console.log("store.test.mjs: all " + passed + " tests passed");
