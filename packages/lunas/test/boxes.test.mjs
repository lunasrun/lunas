// boxes.test.mjs — box / deepBox / shared tests.
// Run: node packages/lunas/test/boxes.test.mjs

import assert from "node:assert";
import { createContext, bind } from "../src/core.mjs";
import { box, deepBox, shared } from "../src/boxes.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("box get/set + reactivity", async () => {
  const c = createContext(null);
  const b = box(c, 0, 1);
  let seen = null;
  bind(c, [0], () => {
    seen = b.v;
  });
  assert.strictEqual(seen, 1);
  b.v = 5;
  assert.strictEqual(b.v, 5, "read-your-write synchronously");
  await tick();
  assert.strictEqual(seen, 5);
});

await test("box same-value write is a no-op", async () => {
  const c = createContext(null);
  const b = box(c, 0, "x");
  let runs = 0;
  bind(c, [0], () => runs++);
  b.v = "x";
  await tick();
  assert.strictEqual(runs, 1, "no flush for identical value");
});

await test("deepBox: push triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2]);
  let len = 0;
  bind(c, [0], () => {
    len = d.v.length;
  });
  assert.strictEqual(len, 2);
  d.v.push(3);
  d.touch(); // compiler-injected invalidation after a structural mutation
  await tick();
  assert.strictEqual(len, 3);
  assert.deepStrictEqual(Array.from(d.v), [1, 2, 3]);
});

await test("deepBox: splice triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2, 3, 4]);
  let snapshot = null;
  bind(c, [0], () => {
    snapshot = Array.from(d.v);
  });
  d.v.splice(1, 2);
  d.touch();
  await tick();
  assert.deepStrictEqual(snapshot, [1, 4]);
});

await test("deepBox: nested object set triggers (via injected touch)", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { user: { name: "a" }, tags: ["x"] });
  let name = null;
  bind(c, [0], () => {
    name = d.v.user.name;
  });
  assert.strictEqual(name, "a");
  d.v.user.name = "b"; // nested field write on the raw value
  d.touch(); // compiler injects this after the mutation
  await tick();
  assert.strictEqual(name, "b");
  let tagCount = 0;
  bind(c, [0], () => {
    tagCount = d.v.tags.length;
  });
  d.v.tags.push("y"); // nested array mutation
  d.touch();
  await tick();
  assert.strictEqual(tagCount, 2);
});

await test("deepBox: delete triggers; whole-value replace triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1, b: 2 });
  let keys = null;
  bind(c, [0], () => {
    keys = Object.keys(d.v).join(",");
  });
  delete d.v.b;
  d.touch();
  await tick();
  assert.strictEqual(keys, "a");
  d.v = { z: 9 }; // whole-value reassign: the setter marks, no touch needed
  await tick();
  assert.strictEqual(keys, "z");
});

await test("deepBox: nested value identity is the raw object, stable across reads", () => {
  const c = createContext(null);
  const inner = {};
  const d = deepBox(c, 0, { user: inner });
  assert.strictEqual(d.v.user, d.v.user, "raw value, no per-read wrapper");
  assert.strictEqual(d.v.user, inner, "reads return the raw object, not a proxy");
});

await test("shared: two components both flushed", async () => {
  const c1 = createContext(null);
  const c2 = createContext(null);
  const s = shared(10);
  s.attach(c1, 0);
  s.attach(c2, 3); // different reactive index per component
  let v1 = null;
  let v2 = null;
  bind(c1, [0], () => {
    v1 = s.v;
  });
  bind(c2, [3], () => {
    v2 = s.v;
  });
  s.v = 11;
  await tick();
  assert.strictEqual(v1, 11);
  assert.strictEqual(v2, 11);
});

await test("shared: detach stops one component, other still updates", async () => {
  const c1 = createContext(null);
  const c2 = createContext(null);
  const s = shared(0);
  s.attach(c1, 0);
  s.attach(c2, 0);
  let v1 = null;
  let v2 = null;
  bind(c1, [0], () => {
    v1 = s.v;
  });
  bind(c2, [0], () => {
    v2 = s.v;
  });
  s.detach(c1);
  s.v = 7;
  await tick();
  assert.strictEqual(v1, 0, "detached component not notified");
  assert.strictEqual(v2, 7);
});

await test("shared: same-value write is a no-op", async () => {
  const c1 = createContext(null);
  const s = shared("k");
  s.attach(c1, 0);
  let runs = 0;
  bind(c1, [0], () => runs++);
  s.v = "k";
  await tick();
  assert.strictEqual(runs, 1);
});

console.log("boxes.test.mjs: all " + passed + " tests passed");
