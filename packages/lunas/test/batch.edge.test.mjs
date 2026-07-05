// batch.edge.test.mjs — batch/nextTick edge cases beyond batch.test.mjs:
// return value passthrough, batch+pending-microtask interplay, batch
// exceptions still flush, empty batch, three-level nesting, nextTick
// scheduled from inside a batch, sequential (non-nested) batches,
// independent contexts, batchDepth bookkeeping.
// Run: node packages/lunas/test/batch.edge.test.mjs

import assert from "node:assert";
import { createContext, bind } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { nextTick, batch } from "../src/batch.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("batch(c, fn) returns fn's return value", () => {
  const c = createContext(null);
  const result = batch(c, () => 42);
  assert.strictEqual(result, 42);
});

await test("batch with zero writes is a harmless no-op (fn still runs, no flush error)", () => {
  const c = createContext(null);
  let ran = false;
  assert.doesNotThrow(() => {
    batch(c, () => {
      ran = true;
    });
  });
  assert.strictEqual(ran, true);
});

await test("batch that throws still flushes synchronously (finally block runs)", () => {
  const c = createContext(null);
  const d = box(c, 0, 0);
  let paints = 0;
  bind(c, [0], () => paints++);
  paints = 0;
  assert.throws(() => {
    batch(c, () => {
      d.v = 1;
      throw new Error("fail");
    });
  }, /fail/);
  assert.strictEqual(paints, 1, "the write before the throw is still flushed via finally");
  assert.strictEqual(d.v, 1);
});

await test("batchDepth: absent before first use, resets to 0 after the outermost batch completes", () => {
  const c = createContext(null);
  assert.strictEqual(c.batchDepth, undefined, "field does not exist until first batch() call");
  batch(c, () => {});
  assert.strictEqual(c.batchDepth, 0, "back to 0 after outermost batch, ready for reuse");
});

await test("three-level nested batch: only the outermost call triggers a flush", () => {
  const c = createContext(null);
  const b = box(c, 0, 0);
  let paints = 0;
  bind(c, [0], () => paints++);
  paints = 0;
  batch(c, () => {
    b.v = 1;
    batch(c, () => {
      b.v = 2;
      batch(c, () => {
        b.v = 3;
      });
      assert.strictEqual(paints, 0, "innermost batch exit does not flush");
    });
    assert.strictEqual(paints, 0, "mid-level batch exit does not flush either");
  });
  assert.strictEqual(paints, 1, "single flush only when the outermost batch call returns");
  assert.strictEqual(b.v, 3, "final value from the deepest write");
});

await test("sequential (non-nested) batches on the same context each flush independently", () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  let paints = 0;
  bind(c, [0], () => paints++);
  paints = 0;
  batch(c, () => {
    a.v = 1;
  });
  assert.strictEqual(paints, 1, "first batch flushed on its own exit");
  batch(c, () => {
    a.v = 2;
  });
  assert.strictEqual(paints, 2, "second (unrelated) batch flushed independently");
});

await test("batch interacts correctly with an already-pending microtask flush from a write made before it", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  let paints = 0;
  bind(c, [0], () => paints++);
  paints = 0;
  a.v = 1; // schedules a microtask flush that hasn't run yet
  batch(c, () => {
    a.v = 2; // synchronous batch flush happens at batch() exit, consuming the queue
  });
  assert.strictEqual(paints, 1, "batch's synchronous flush already drained the queue");
  await tick();
  assert.strictEqual(paints, 1, "the earlier-scheduled microtask flush finds an empty queue: harmless no-op");
});

await test("nextTick scheduled from inside a batch resolves after the batch's synchronous flush has landed", async () => {
  const c = createContext(null);
  const d = box(c, 0, 0);
  let painted = -1;
  bind(c, [0], () => {
    painted = d.v;
  });
  let ntResolved = false;
  batch(c, () => {
    d.v = 9;
    nextTick(c).then(() => {
      ntResolved = true;
    });
    assert.strictEqual(painted, 0, "write not yet visible mid-batch: flush happens at batch exit");
  });
  assert.strictEqual(painted, 9, "flushed synchronously by the time batch() returns");
  assert.strictEqual(ntResolved, false, "nextTick's promise callback is still a pending microtask continuation");
  await tick();
  assert.strictEqual(ntResolved, true);
});

await test("batches on two different contexts nest independently even when interleaved (c2's batch closes inside c1's)", () => {
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
      batch(c2, () => {
        a2.v = 2;
      });
      assert.strictEqual(p2, 0, "c2's inner nested batch hasn't closed yet");
    });
    assert.strictEqual(p2, 1, "c2 flushed fully on its own outermost batch exit, independent of c1 still being open");
    assert.strictEqual(p1, 0, "c1 not yet flushed: still inside its own batch");
  });
  assert.strictEqual(p1, 1, "c1 flushed once its own outermost batch exits");
});

await test("multiple nextTick calls with no pending writes each still resolve, in call order", async () => {
  const c = createContext(null);
  const order = [];
  const p1 = nextTick(c).then(() => order.push("a"));
  const p2 = nextTick(c).then(() => order.push("b"));
  const p3 = nextTick(c).then(() => order.push("c"));
  await Promise.all([p1, p2, p3]);
  assert.deepStrictEqual(order, ["a", "b", "c"]);
});

await test("nextTick does not itself trigger any bind reruns beyond what was already pending", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  let runs = 0;
  bind(c, [0], () => runs++);
  runs = 0;
  await nextTick(c); // nothing was written; should not cause any bind to run
  assert.strictEqual(runs, 0, "nextTick alone never marks any var dirty");
  void a;
});

console.log("batch.edge.test.mjs: all " + passed + " tests passed");
