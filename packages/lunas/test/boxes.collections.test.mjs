// boxes.collections.test.mjs — deepBox reactivity for native Map/Set (and
// no-throw handling of WeakMap/WeakSet). deepBox is proxy-free: `.v` returns the
// raw collection, so accessors/methods run natively (no incompatible-receiver
// hazard), and a membership mutation (set/add/delete/clear) is made reactive by
// the compiler-injected `.touch()` that follows it — asserted here explicitly.
// Run: node packages/lunas/test/boxes.collections.test.mjs

import assert from "node:assert";
import { createContext, bind } from "../src/core.mjs";
import { deepBox } from "../src/boxes.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// ---------------------------------------------------------------------------
// Map: reads work, mutations are reactive.
// ---------------------------------------------------------------------------

await test("Map: size/get/has/keys/values/entries/forEach read without throwing", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1], ["b", 2]]));
  const m = d.v;
  assert.strictEqual(m.size, 2);
  assert.strictEqual(m.get("a"), 1);
  assert.strictEqual(m.has("b"), true);
  assert.strictEqual(m.has("z"), false);
  assert.deepStrictEqual([...m.keys()], ["a", "b"]);
  assert.deepStrictEqual([...m.values()], [1, 2]);
  assert.deepStrictEqual([...m.entries()], [["a", 1], ["b", 2]]);
  const seen = [];
  m.forEach((v, k) => seen.push(k + "=" + v));
  assert.deepStrictEqual(seen, ["a=1", "b=2"]);
});

await test("Map: iteration (for..of and spread) works through the proxy", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1]]));
  const out = [];
  for (const [k, v] of d.v) out.push(k + v);
  assert.deepStrictEqual(out, ["a1"]);
  assert.deepStrictEqual([...d.v], [["a", 1]]);
});

await test("Map: set() is reactive — a bind reading .size re-runs", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map());
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.size));
  assert.deepStrictEqual(sizes, [0], "initial bind run");
  d.v.set("a", 1);
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [0, 1], "set + touch marks the var dirty");
});

await test("Map: set() to an existing key still notifies (value may have changed)", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1]]));
  const values = [];
  bind(c, [0], () => values.push(d.v.get("a")));
  d.v.set("a", 2);
  d.touch();
  await tick();
  assert.deepStrictEqual(values, [1, 2], "re-set of a key re-runs the bind");
});

await test("Map: delete() is reactive", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1]]));
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.size));
  const removed = d.v.delete("a");
  d.touch();
  assert.strictEqual(removed, true, "delete returns the native boolean");
  await tick();
  assert.deepStrictEqual(sizes, [1, 0]);
});

await test("Map: clear() is reactive", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1], ["b", 2]]));
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.size));
  d.v.clear();
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [2, 0]);
});

await test("Map: chained set() returns the Map (native contract preserved)", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map());
  const ret = d.v.set("a", 1);
  // `.v` is the raw Map, so native Map.set returns the map itself.
  assert.strictEqual(ret.get("a"), 1);
  assert.strictEqual(ret.size, 1);
});

// ---------------------------------------------------------------------------
// Set: reads work, mutations are reactive.
// ---------------------------------------------------------------------------

await test("Set: size/has/values/entries/forEach read without throwing", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Set([1, 2, 3]));
  const s = d.v;
  assert.strictEqual(s.size, 3);
  assert.strictEqual(s.has(2), true);
  assert.strictEqual(s.has(9), false);
  assert.deepStrictEqual([...s.values()], [1, 2, 3]);
  assert.deepStrictEqual([...s], [1, 2, 3]);
  const seen = [];
  s.forEach((v) => seen.push(v));
  assert.deepStrictEqual(seen, [1, 2, 3]);
});

await test("Set: add() is reactive", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Set());
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.size));
  d.v.add(1);
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [0, 1]);
});

await test("Set: delete() and clear() are reactive", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Set([1, 2]));
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.size));
  d.v.delete(1);
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [2, 1]);
  d.v.clear();
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [2, 1, 0]);
});

await test("Set: add() returns the Set (native contract preserved)", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Set());
  const ret = d.v.add(1);
  assert.strictEqual(ret.has(1), true);
});

// ---------------------------------------------------------------------------
// Collections nested inside a deepBox'd object.
// ---------------------------------------------------------------------------

await test("Map nested in a deepBox'd object: reads work and mutations mark dirty", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { tags: new Map([["x", 1]]) });
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.tags.size));
  assert.deepStrictEqual(sizes, [1], "nested Map .size reads without throwing");
  d.v.tags.set("y", 2);
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [1, 2], "nested Map mutation + touch marks the outer var");
});

await test("Set nested in a deepBox'd object: mutations mark dirty", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { ids: new Set([1]) });
  const sizes = [];
  bind(c, [0], () => sizes.push(d.v.ids.size));
  d.v.ids.add(2);
  d.touch();
  await tick();
  assert.deepStrictEqual(sizes, [1, 2]);
});

await test("nested Map identity is stable across property reads (raw)", () => {
  const c = createContext(null);
  const m = new Map();
  const d = deepBox(c, 0, { m });
  assert.strictEqual(d.v.m, d.v.m, "same underlying Map");
  assert.strictEqual(d.v.m, m, "the raw Map itself, not a proxy");
});

// ---------------------------------------------------------------------------
// WeakMap/WeakSet: not deeply reactive, but must not throw.
// ---------------------------------------------------------------------------

await test("WeakMap: get/set/has/delete work without throwing (not deeply reactive)", () => {
  const c = createContext(null);
  const key = {};
  const d = deepBox(c, 0, new WeakMap([[key, 1]]));
  const wm = d.v;
  assert.strictEqual(wm.get(key), 1);
  assert.strictEqual(wm.has(key), true);
  wm.set(key, 2);
  assert.strictEqual(wm.get(key), 2);
  assert.strictEqual(wm.delete(key), true);
  assert.strictEqual(wm.has(key), false);
});

await test("WeakSet: add/has/delete work without throwing", () => {
  const c = createContext(null);
  const a = {};
  const d = deepBox(c, 0, new WeakSet([a]));
  const ws = d.v;
  assert.strictEqual(ws.has(a), true);
  const b = {};
  ws.add(b);
  assert.strictEqual(ws.has(b), true);
  assert.strictEqual(ws.delete(a), true);
  assert.strictEqual(ws.has(a), false);
});

await test("WeakMap mutation still notifies (collection-level, best-effort)", async () => {
  // WeakMap has no size/iteration, but membership mutations still mark the var
  // so a bind reading `.has(key)` can react. This is best-effort — WeakMaps are
  // documented as not deeply reactive, but the mutators do notify.
  const c = createContext(null);
  const key = {};
  const d = deepBox(c, 0, new WeakMap());
  const seen = [];
  bind(c, [0], () => seen.push(d.v.has(key)));
  d.v.set(key, 1);
  d.touch();
  await tick();
  assert.deepStrictEqual(seen, [false, true]);
});

// ---------------------------------------------------------------------------
// Regression: object/array deepBox behavior is unchanged.
// ---------------------------------------------------------------------------

await test("regression: object property set through deepBox still marks dirty", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  const seen = [];
  bind(c, [0], () => seen.push(d.v.a));
  d.v.a = 2;
  d.touch();
  await tick();
  assert.deepStrictEqual(seen, [1, 2]);
});

await test("regression: array push/splice through deepBox still marks dirty", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2]);
  const lens = [];
  bind(c, [0], () => lens.push(d.v.length));
  d.v.push(3);
  d.touch();
  await tick();
  assert.deepStrictEqual(lens, [2, 3]);
  d.v.splice(0, 1);
  d.touch();
  await tick();
  assert.deepStrictEqual(lens, [2, 3, 2]);
});

await test("regression: nested object identity stays stable (raw)", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { user: { name: "a" } });
  assert.strictEqual(d.v.user, d.v.user);
});

console.log("boxes.collections.test.mjs: all " + passed + " tests passed");
