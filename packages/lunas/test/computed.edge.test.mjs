// computed.edge.test.mjs — lazy derived values: chained computed, diamond
// deps, multi-dep coalescing, laziness across never-read paths, read-inside-
// bind tracking edge cases beyond computed.test.mjs.
// Run: node packages/lunas/test/computed.edge.test.mjs

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

await test("computed never read: fn never runs, even across multiple dep changes", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let computes = 0;
  computed(c, 1, [0], () => {
    computes++;
    return a.v;
  });
  a.v = 2;
  await tick();
  a.v = 3;
  await tick();
  assert.strictEqual(computes, 0, "no read anywhere -> fn body never executes");
});

await test("chained computed: computed-of-computed recomputes exactly once per upstream change", async () => {
  const c = createContext(null);
  const a = box(c, 0, 2);
  let doubleRuns = 0;
  let quadRuns = 0;
  const double = computed(c, 1, [0], () => {
    doubleRuns++;
    return a.v * 2;
  });
  const quad = computed(c, 2, [1], () => {
    quadRuns++;
    return double.v * 2;
  });
  assert.strictEqual(quad.v, 8);
  assert.strictEqual(doubleRuns, 1);
  assert.strictEqual(quadRuns, 1);
  a.v = 3;
  await tick();
  assert.strictEqual(quad.v, 12, "chain recomputed with fresh upstream value");
  assert.strictEqual(doubleRuns, 2, "double recomputed exactly once");
  assert.strictEqual(quadRuns, 2, "quad recomputed exactly once");
});

await test("chained computed: reading only the outer one still recomputes the inner exactly once", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let innerRuns = 0;
  const inner = computed(c, 1, [0], () => {
    innerRuns++;
    return a.v + 1;
  });
  const outer = computed(c, 2, [1], () => inner.v * 10);
  assert.strictEqual(outer.v, 20);
  a.v = 5;
  await tick();
  assert.strictEqual(outer.v, 60, "reading outer pulls a fresh inner value");
  assert.strictEqual(innerRuns, 2, "inner recomputed once per read-through, not per dep-change alone");
});

await test("diamond dependency: two computeds derive from one base, a third combines both, each recomputes once", async () => {
  const c = createContext(null);
  const base = box(c, 0, 1);
  let leftRuns = 0;
  let rightRuns = 0;
  let combineRuns = 0;
  const left = computed(c, 1, [0], () => {
    leftRuns++;
    return base.v + 1;
  });
  const right = computed(c, 2, [0], () => {
    rightRuns++;
    return base.v * 10;
  });
  const combined = computed(c, 3, [1, 2], () => {
    combineRuns++;
    return left.v + right.v;
  });
  assert.strictEqual(combined.v, 1 + 1 + 1 * 10);
  assert.strictEqual(leftRuns, 1);
  assert.strictEqual(rightRuns, 1);
  assert.strictEqual(combineRuns, 1);
  base.v = 2;
  await tick();
  assert.strictEqual(combined.v, 2 + 1 + 2 * 10);
  assert.strictEqual(leftRuns, 2, "left recomputed exactly once despite two paths to base");
  assert.strictEqual(rightRuns, 2, "right recomputed exactly once");
  assert.strictEqual(combineRuns, 2, "combine recomputed exactly once, not twice for two upstream changes");
});

await test("multiple deps changed in the same tick -> exactly one recompute", async () => {
  const c = createContext(null);
  const x = box(c, 0, 1);
  const y = box(c, 1, 1);
  let sumRuns = 0;
  const sum = computed(c, 2, [0, 1], () => {
    sumRuns++;
    return x.v + y.v;
  });
  assert.strictEqual(sum.v, 2);
  assert.strictEqual(sumRuns, 1);
  x.v = 5;
  y.v = 5;
  await tick();
  assert.strictEqual(sum.v, 10, "both writes reflected");
  assert.strictEqual(sumRuns, 2, "exactly one recompute for the whole coalesced flush, not two");
});

await test("dep changes but the computed is never read again before a further change: still exactly one recompute on next read", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let computes = 0;
  const d = computed(c, 1, [0], () => {
    computes++;
    return a.v;
  });
  assert.strictEqual(d.v, 1);
  a.v = 2;
  await tick(); // stale, not read
  a.v = 3;
  await tick(); // stale again, still not read -- only one flag flip, no compounding
  assert.strictEqual(computes, 1, "no recompute happened without a read");
  assert.strictEqual(d.v, 3, "first read after two skipped changes sees the latest value");
  assert.strictEqual(computes, 2, "exactly one recompute for the catch-up read");
});

await test("reading a computed inside a bind subscribes the bind via the computed's own index (not its upstream deps)", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const d = computed(c, 1, [0], () => a.v * 3);
  let bindRuns = 0;
  // The bind only declares index 1 (the computed), matching what the
  // compiler emits for a template part reading a derived value -- it does
  // NOT need to know about index 0 directly.
  bind(c, [1], () => {
    bindRuns++;
    void d.v;
  });
  assert.strictEqual(bindRuns, 1);
  a.v = 2; // upstream write marks index 1 dirty via the computed's internal bind
  await tick();
  assert.strictEqual(bindRuns, 2, "bind re-ran because the computed's own index was marked");
});

await test("computed with an empty deps list never recomputes after the first read (no upstream can mark it stale)", () => {
  const c = createContext(null);
  let computes = 0;
  const d = computed(c, 0, [], () => {
    computes++;
    return 42;
  });
  assert.strictEqual(d.v, 42);
  assert.strictEqual(d.v, 42);
  assert.strictEqual(computes, 1, "memoized forever: nothing can invalidate it");
});

await test("computed fn can throw: laziness means the throw happens at read time, not registration", () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const d = computed(c, 1, [0], () => {
    if (a.v === 0) throw new Error("boom");
    return a.v;
  });
  assert.throws(() => d.v, /boom/);
});

await test("computed fn recovers on next read after a dep change, even if the previous read threw", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const d = computed(c, 1, [0], () => {
    if (a.v === 0) throw new Error("boom");
    return a.v * 2;
  });
  assert.throws(() => d.v);
  a.v = 5;
  await tick();
  assert.strictEqual(d.v, 10, "fresh read after the dep changed recomputes cleanly");
});

await test("two independent computed instances over the same box do not share memoized state", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runsX = 0;
  let runsY = 0;
  const x = computed(c, 1, [0], () => {
    runsX++;
    return a.v + 100;
  });
  const y = computed(c, 2, [0], () => {
    runsY++;
    return a.v + 200;
  });
  assert.strictEqual(x.v, 101);
  assert.strictEqual(y.v, 201);
  assert.strictEqual(runsX, 1);
  assert.strictEqual(runsY, 1);
  a.v = 2;
  await tick();
  // Read only x this time.
  assert.strictEqual(x.v, 102);
  assert.strictEqual(runsX, 2);
  assert.strictEqual(runsY, 1, "y untouched: laziness is per-computed, no shared read");
});

console.log("computed.edge.test.mjs: all " + passed + " tests passed");
