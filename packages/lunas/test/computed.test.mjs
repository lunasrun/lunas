// computed.test.mjs — lazy derived values in the adjacency model.
// Run: node packages/lunas/test/computed.test.mjs

import assert from "node:assert";
import { createContext, bind } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { computed } from "../src/computed.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("computed is lazy: fn does not run until read", () => {
  const c = createContext(null);
  const a = box(c, 0, 2);
  let computes = 0;
  computed(c, 1, [0], () => {
    computes++;
    return a.v * 10;
  });
  assert.strictEqual(computes, 0, "no read yet -> no compute");
});

await test("computed recomputes once on read, then memoizes", () => {
  const c = createContext(null);
  const a = box(c, 0, 2);
  let computes = 0;
  const d = computed(c, 1, [0], () => {
    computes++;
    return a.v * 10;
  });
  assert.strictEqual(d.v, 20);
  assert.strictEqual(d.v, 20, "second read is memoized");
  assert.strictEqual(computes, 1, "computed exactly once for two reads");
});

await test("computed recomputes only after a dep changes", async () => {
  const c = createContext(null);
  const a = box(c, 0, 2);
  let computes = 0;
  const d = computed(c, 1, [0], () => {
    computes++;
    return a.v + 1;
  });
  assert.strictEqual(d.v, 3);
  assert.strictEqual(computes, 1);
  a.v = 5; // dep changed -> stale, but not recomputed yet (lazy)
  assert.strictEqual(computes, 1, "no eager recompute on write");
  await tick();
  assert.strictEqual(d.v, 6, "recomputes on next read after change");
  assert.strictEqual(computes, 2);
});

await test("computed participates in adjacency: reading it in a bind tracks it", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const d = computed(c, 1, [0], () => a.v * 2);
  const seen = [];
  // The bind declares the computed's index (1) in its deps — exactly what the
  // compiler emits for a part that reads a derived value.
  bind(c, [1], () => seen.push(d.v));
  assert.deepStrictEqual(seen, [2], "initial run pulls fresh value");
  a.v = 4;
  await tick();
  assert.deepStrictEqual(seen, [2, 8], "downstream bind re-ran with new value");
});

await test("unchanged dep -> downstream bind does not re-run via computed", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const other = box(c, 2, "x");
  const d = computed(c, 1, [0], () => a.v * 2);
  let runs = 0;
  bind(c, [1], () => {
    runs++;
    void d.v;
  });
  assert.strictEqual(runs, 1);
  other.v = "y"; // unrelated var
  await tick();
  assert.strictEqual(runs, 1, "computed's dependents untouched");
});

await test("computed over multiple deps", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const b = box(c, 1, 2);
  let computes = 0;
  const sum = computed(c, 2, [0, 1], () => {
    computes++;
    return a.v + b.v;
  });
  assert.strictEqual(sum.v, 3);
  b.v = 10;
  await tick();
  assert.strictEqual(sum.v, 11);
  assert.strictEqual(computes, 2);
});

console.log("computed.test.mjs: all " + passed + " tests passed");
