// batch.test.mjs — coalescing and nextTick.
// Run: node packages/lunas/test/batch.test.mjs

import assert from "node:assert";
import { createContext, bind, markVar } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { nextTick, batch } from "../src/batch.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("N markVar on one var -> one flush pass", async () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0], () => runs++);
  markVar(c, 0);
  markVar(c, 0);
  markVar(c, 0);
  await tick();
  assert.strictEqual(runs, 2, "initial + exactly one flush");
});

await test("handler writing several vars -> single DOM update pass", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const b = box(c, 1, 0);
  let paints = 0;
  // One part reads both vars, like a template line interpolating both.
  bind(c, [0, 1], () => {
    paints++;
    void a.v;
    void b.v;
  });
  paints = 0;
  // A handler mutating multiple boxes:
  a.v = 1;
  b.v = 2;
  a.v = 3;
  assert.strictEqual(paints, 0, "nothing painted synchronously");
  await tick();
  assert.strictEqual(paints, 1, "coalesced into a single pass");
});

await test("nextTick resolves after the pending flush (DOM updated)", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  let painted = -1;
  bind(c, [0], () => {
    painted = a.v;
  });
  a.v = 42;
  // Before the tick the paint has not happened.
  assert.strictEqual(painted, 0);
  await nextTick(c);
  assert.strictEqual(painted, 42, "flush completed before nextTick resolved");
});

await test("nextTick with nothing pending still resolves this tick", async () => {
  const c = createContext(null);
  let resolved = false;
  const p = nextTick(c).then(() => {
    resolved = true;
  });
  assert.strictEqual(resolved, false);
  await p;
  assert.strictEqual(resolved, true);
});

await test("nextTick ordering: two calls resolve in order, after paint", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const order = [];
  bind(c, [0], () => order.push("paint:" + a.v));
  a.v = 1;
  const p1 = nextTick(c).then(() => order.push("t1"));
  const p2 = nextTick(c).then(() => order.push("t2"));
  await Promise.all([p1, p2]);
  assert.deepStrictEqual(order, ["paint:0", "paint:1", "t1", "t2"]);
});

await test("batch flushes synchronously at the outermost call", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const b = box(c, 1, 0);
  let paints = 0;
  bind(c, [0, 1], () => {
    paints++;
    void a.v;
    void b.v;
  });
  paints = 0;
  batch(c, () => {
    a.v = 1;
    b.v = 2;
  });
  assert.strictEqual(paints, 1, "applied synchronously, once, on batch exit");
  await tick();
  assert.strictEqual(paints, 1, "no extra microtask flush");
});

await test("nested batch flushes only at the outermost", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  let paints = 0;
  bind(c, [0], () => {
    paints++;
    void a.v;
  });
  paints = 0;
  batch(c, () => {
    a.v = 1;
    batch(c, () => {
      a.v = 2;
    });
    assert.strictEqual(paints, 0, "inner batch did not flush");
  });
  assert.strictEqual(paints, 1, "single flush at outer exit");
});

await test("batches on different contexts don't suppress each other", () => {
  const c1 = createContext(null);
  const c2 = createContext(null);
  const a1 = box(c1, 0, 0);
  const a2 = box(c2, 0, 0);
  let p1 = 0;
  let p2 = 0;
  bind(c1, [0], () => {
    p1++;
    void a1.v;
  });
  bind(c2, [0], () => {
    p2++;
    void a2.v;
  });
  p1 = 0;
  p2 = 0;
  batch(c1, () => {
    a1.v = 1;
    batch(c2, () => {
      a2.v = 1;
    });
    assert.strictEqual(p2, 1, "c2 flushed at its own batch exit");
  });
  assert.strictEqual(p1, 1, "c1 flushed at its own batch exit");
});

console.log("batch.test.mjs: all " + passed + " tests passed");
