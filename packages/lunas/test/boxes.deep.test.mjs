// boxes.deep.test.mjs — box/deepBox/shared edge cases beyond boxes.test.mjs:
// identity no-ops, NaN semantics, deep array method coverage, nested-depth
// mutation, Proxy identity stability, Map/Set limitations, whole-value
// replacement, re-entrant writes.
// Run: node packages/lunas/test/boxes.deep.test.mjs

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

// ---------------------------------------------------------------------------
// box(): identity, NaN, undefined, rapid coalescing, re-entrant writes.
// ---------------------------------------------------------------------------

await test("box: identical object reference write is a no-op (=== check, not deep equal)", async () => {
  const c = createContext(null);
  const obj = { a: 1 };
  const b = box(c, 0, obj);
  let runs = 0;
  bind(c, [0], () => runs++);
  b.v = obj; // same reference
  await tick();
  assert.strictEqual(runs, 1, "no flush: x === v short-circuits in the setter");
});

await test("box: NaN -> NaN always triggers (NaN !== NaN, so the box's === guard never treats it as unchanged)", async () => {
  const c = createContext(null);
  const b = box(c, 0, NaN);
  let runs = 0;
  bind(c, [0], () => runs++);
  b.v = NaN;
  await tick();
  assert.strictEqual(runs, 2, "every NaN write flushes, even setting NaN to NaN");
  b.v = NaN;
  await tick();
  assert.strictEqual(runs, 3, "still triggers on a second identical NaN write");
});

await test("box: undefined -> concrete value transitions correctly", async () => {
  const c = createContext(null);
  const b = box(c, 0, undefined);
  let seen;
  bind(c, [0], () => {
    seen = b.v;
  });
  assert.strictEqual(seen, undefined);
  b.v = "x";
  await tick();
  assert.strictEqual(seen, "x");
});

await test("box: undefined -> undefined write is a no-op", async () => {
  const c = createContext(null);
  const b = box(c, 0, undefined);
  let runs = 0;
  bind(c, [0], () => runs++);
  b.v = undefined;
  await tick();
  assert.strictEqual(runs, 1, "undefined === undefined -> no flush");
});

await test("box: rapid multi-set coalescing observes only the final value", async () => {
  const c = createContext(null);
  const b = box(c, 0, 0);
  const seenEach = [];
  bind(c, [0], () => seenEach.push(b.v));
  seenEach.length = 0;
  b.v = 1;
  b.v = 2;
  b.v = 3;
  assert.deepStrictEqual(seenEach, [], "nothing runs synchronously");
  await tick();
  assert.deepStrictEqual(seenEach, [3], "single coalesced flush sees only the last write");
});

await test("box: set from inside a bind (re-entrant write to a different var) is delivered next flush", async () => {
  const c = createContext(null);
  const a = box(c, 0, 0);
  const other = box(c, 1, 100);
  const log = [];
  bind(c, [0], () => {
    log.push("a:" + a.v);
    if (a.v === 1) other.v = 999;
  });
  bind(c, [1], () => log.push("other:" + other.v));
  log.length = 0;
  a.v = 1;
  await tick();
  assert.deepStrictEqual(log, ["a:1", "other:999"], "write inside a bind schedules its own flush entry");
});

await test("box: set inside a bind for the SAME var re-queues itself rather than looping synchronously in one flush pass", async () => {
  // Note: bind(c, deps, fn) invokes fn() once immediately at registration,
  // before deps are wired into c.deps[] -- so any self-write guarded only by
  // the box's own value would already fire during that registration call
  // (deps not yet wired -> markVar is a same-tick no-op there). Use an
  // external "armed" flag so the self-write only happens on a run triggered
  // by a real external write, isolating the behavior under test: does a
  // write-to-self inside a bind's fn get delivered, and when?
  const c = createContext(null);
  const a = box(c, 0, 0);
  let runs = 0;
  let armed = false;
  bind(c, [0], () => {
    runs++;
    if (armed) {
      armed = false;
      a.v = a.v + 1; // one guarded self-write, schedules exactly one more run
    }
  });
  runs = 0;
  armed = true;
  a.v = 10;
  await tick();
  // flush()'s `for (const s of q)` iterates a snapshot array taken before
  // running any fn, so the self-write's markVar pushes into a *new*
  // c.queue rather than the one currently mid-iteration -- but that new
  // push still happens via a fresh queueMicrotask, which fully drains
  // (it's still a microtask) before our setTimeout-based tick() resolves.
  assert.strictEqual(runs, 2, "external write's run, then the self-write's own run, both land in this tick()");
  assert.strictEqual(a.v, 11, "self-write applied: 10 -> 11");
});

// ---------------------------------------------------------------------------
// deepBox(): nested mutation depths, array methods, delete, length
// truncation, whole-value replace, Proxy identity, NaN-through-proxy.
// ---------------------------------------------------------------------------

await test("deepBox: deeply nested (3+ levels) mutation triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: { b: { c: { d: 1 } } } });
  let val;
  bind(c, [0], () => {
    val = d.v.a.b.c.d;
  });
  assert.strictEqual(val, 1);
  d.v.a.b.c.d = 2;
  await tick();
  assert.strictEqual(val, 2);
});

await test("deepBox: array of objects, mutating an element's field triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [{ n: 1 }, { n: 2 }]);
  let total;
  bind(c, [0], () => {
    total = d.v[0].n + d.v[1].n;
  });
  assert.strictEqual(total, 3);
  d.v[0].n = 10;
  await tick();
  assert.strictEqual(total, 12);
});

await test("deepBox: sort() and reverse() both trigger and produce correct order", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [3, 1, 2]);
  let snap;
  bind(c, [0], () => {
    snap = Array.from(d.v);
  });
  d.v.sort();
  await tick();
  assert.deepStrictEqual(snap, [1, 2, 3]);
  d.v.reverse();
  await tick();
  assert.deepStrictEqual(snap, [3, 2, 1]);
});

await test("deepBox: shift() and unshift() both trigger", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2, 3]);
  let snap;
  bind(c, [0], () => {
    snap = Array.from(d.v);
  });
  d.v.unshift(0);
  await tick();
  assert.deepStrictEqual(snap, [0, 1, 2, 3]);
  d.v.shift();
  await tick();
  assert.deepStrictEqual(snap, [1, 2, 3]);
});

await test("deepBox: pop() triggers and returns the removed element", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2, 3]);
  let snap;
  bind(c, [0], () => {
    snap = Array.from(d.v);
  });
  const popped = d.v.pop();
  assert.strictEqual(popped, 3);
  await tick();
  assert.deepStrictEqual(snap, [1, 2]);
});

await test("deepBox: length truncation (arr.length = n) triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [1, 2, 3, 4, 5]);
  let snap;
  bind(c, [0], () => {
    snap = Array.from(d.v);
  });
  d.v.length = 2;
  await tick();
  assert.deepStrictEqual(snap, [1, 2]);
});

await test("deepBox: replacing the whole value swaps the underlying proxy target entirely", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  let snap;
  bind(c, [0], () => {
    snap = { ...d.v };
  });
  assert.deepStrictEqual(snap, { a: 1 });
  d.v = { b: 2 };
  await tick();
  assert.deepStrictEqual(snap, { b: 2 }, "old key gone entirely, not merged");
});

await test("deepBox: whole-value replace with the same reference is a no-op", async () => {
  const c = createContext(null);
  const obj = { a: 1 };
  const d = deepBox(c, 0, obj);
  let runs = 0;
  bind(c, [0], () => runs++);
  d.v = obj;
  await tick();
  assert.strictEqual(runs, 1, "x === v guard applies to deepBox's raw value too");
});

await test("deepBox: proxy identity stable across repeated reads of the same nested path", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { list: [1, 2, 3] });
  assert.strictEqual(d.v.list, d.v.list, "nested array wrapper is cached, not rebuilt per read");
});

await test("deepBox: proxy identity differs after whole-value replacement", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  const first = d.v;
  d.v = { a: 1 }; // different underlying object, deep-equal but not ===
  assert.notStrictEqual(d.v, first, "new raw object -> freshly wrapped proxy");
});

await test("deepBox: delete on a nested (not top-level) key triggers", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { outer: { a: 1, b: 2 } });
  let keys;
  bind(c, [0], () => {
    keys = Object.keys(d.v.outer).sort().join(",");
  });
  assert.strictEqual(keys, "a,b");
  delete d.v.outer.b;
  await tick();
  assert.strictEqual(keys, "a");
});

await test("deepBox: deleting a non-existent key does not trigger (had=false guard)", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  let runs = 0;
  bind(c, [0], () => runs++);
  delete d.v.doesNotExist;
  await tick();
  assert.strictEqual(runs, 1, "no spurious flush for deleting an absent key");
});

await test("deepBox: setting a nested field to NaN always triggers (NaN !== NaN check inside the proxy set trap)", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { n: NaN });
  let runs = 0;
  bind(c, [0], () => runs++);
  d.v.n = NaN;
  await tick();
  assert.strictEqual(runs, 2, "nested NaN write always reports as changed");
});

await test("deepBox: setting a nested field to the SAME primitive value is a no-op", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { n: 5 });
  let runs = 0;
  bind(c, [0], () => runs++);
  d.v.n = 5;
  await tick();
  assert.strictEqual(runs, 1, "old === new for a primitive -> proxy set trap skips notify");
});

await test("deepBox: primitive (non-object) values pass through the wrapper untouched", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, 42);
  assert.strictEqual(d.v, 42);
  const dNull = deepBox(c, 1, null);
  assert.strictEqual(dNull.v, null);
});

await test("deepBox: two different deepBox instances wrapping equal-shaped objects have independent proxy caches", () => {
  const c = createContext(null);
  const d1 = deepBox(c, 0, { a: 1 });
  const d2 = deepBox(c, 1, { a: 1 });
  assert.notStrictEqual(d1.v, d2.v, "distinct underlying objects -> distinct proxies");
});

await test("deepBox: Map accessors/methods work through the collection-aware handler (no more incompatible-receiver throw)", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, new Map([["a", 1]]));
  // `.size` (an accessor with internal slots) and `.get`/`.has` (methods) now
  // run against the real Map, so the native internal-slot check passes.
  assert.strictEqual(d.v.size, 1, ".size reads without throwing");
  assert.strictEqual(d.v.get("a"), 1, ".get returns the entry");
  assert.strictEqual(d.v.has("a"), true, ".has works");
  assert.strictEqual(d.v.has("z"), false);
});

console.log("boxes.deep.test.mjs: all " + passed + " tests passed");
