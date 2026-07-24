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
  d.touch(); // deep nested field write -> compiler-injected structural touch
  await tick();
  assert.strictEqual(val, 2);
});

await test("deepBox: array of objects, mutating an element's field triggers (touchElem)", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [{ n: 1 }, { n: 2 }]);
  let total;
  bind(c, [0], () => {
    total = d.v[0].n + d.v[1].n;
  });
  assert.strictEqual(total, 3);
  d.v[0].n = 10;
  d.touchElem(d.v[0]); // element-field mutation of a direct array element
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
  d.touch();
  await tick();
  assert.deepStrictEqual(snap, [1, 2, 3]);
  d.v.reverse();
  d.touch();
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
  d.touch();
  await tick();
  assert.deepStrictEqual(snap, [0, 1, 2, 3]);
  d.v.shift();
  d.touch();
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
  d.touch();
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
  d.touch();
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

await test("deepBox: nested value identity is stable across repeated reads (raw, uncached)", () => {
  const c = createContext(null);
  const list = [1, 2, 3];
  const d = deepBox(c, 0, { list });
  assert.strictEqual(d.v.list, d.v.list, "reads return the same raw nested value");
  assert.strictEqual(d.v.list, list, "no proxy wrapper — the raw array itself");
});

await test("deepBox: value identity differs after whole-value replacement", () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  const first = d.v;
  d.v = { a: 1 }; // different underlying object, deep-equal but not ===
  assert.notStrictEqual(d.v, first, "new raw object after reassign");
});

await test("deepBox: delete on a nested (not top-level) key triggers via touch", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, { outer: { a: 1, b: 2 } });
  let keys;
  bind(c, [0], () => {
    keys = Object.keys(d.v.outer).sort().join(",");
  });
  assert.strictEqual(keys, "a,b");
  delete d.v.outer.b;
  d.touch();
  await tick();
  assert.strictEqual(keys, "a");
});

await test("deepBox: a deep mutation without an injected touch does NOT notify (proxy-free)", async () => {
  // Documents the new contract: reactivity for a deep mutation comes from the
  // compiler-injected touch(), not a runtime Proxy. A bare nested write that is
  // never followed by touch() is invisible to dependents (the compiler always
  // emits the touch in real components).
  const c = createContext(null);
  const d = deepBox(c, 0, { a: 1 });
  let runs = 0;
  bind(c, [0], () => runs++);
  d.v.a = 2; // mutated the raw value, but no touch()
  await tick();
  assert.strictEqual(runs, 1, "no flush without an explicit touch()");
  d.touch(); // now signal it
  await tick();
  assert.strictEqual(runs, 2, "touch() delivers the pending mutation");
});

await test("deepBox: touch() always notifies, even for a same-value nested write", async () => {
  // Unlike the old Proxy set trap (which had an old===new guard), the injected
  // touch() is unconditional — the compiler cannot cheaply prove a mutation was
  // a no-op, so a deep write always flushes dependents (Svelte-family behavior).
  const c = createContext(null);
  const d = deepBox(c, 0, { n: 5 });
  let runs = 0;
  bind(c, [0], () => runs++);
  d.v.n = 5;
  d.touch();
  await tick();
  assert.strictEqual(runs, 2, "touch() notifies unconditionally");
});

await test("deepBox: primitive (non-object) values pass through untouched", () => {
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

// ---------------------------------------------------------------------------
// Raw element identity: reads return the raw elements directly (no proxy
// wrapping), so a keyed `:for` swap (r = arr.slice(); reorder; arr = r) never
// accumulates proxy layers — there are no proxies at all. Element identity is
// just object identity, stable by construction across read/store cycles.
// ---------------------------------------------------------------------------
await test("deepBox: reads return raw elements; identity is stable across read-and-store cycles", async () => {
  const c = createContext(null);
  const raw0 = { id: 1 };
  const d = deepBox(c, 0, [raw0, { id: 2 }, { id: 3 }]);
  d.observeElems(); // fine mode

  assert.strictEqual(d.v[0], raw0, "reads return the raw element, not a proxy");

  // Simulate several swap cycles: slice, reorder, reassign the whole value.
  for (let cycle = 0; cycle < 5; cycle++) {
    const r = d.v.slice();
    const t = r[0];
    r[0] = r[2];
    r[2] = t;
    d.v = r;
  }

  // raw0's element is the SAME object no matter how many reorder cycles ran.
  const eAfter = d.v.find((e) => e === raw0);
  assert.ok(eAfter, "raw0's element survives the reorder cycles");
  assert.strictEqual(eAfter, raw0, "still the same raw object — no wrapper layers");
});

// A field write on a stored element, followed by touchElem, still marks the box
// (fine mode): element attribution now comes from the explicit call, not a trap.
await test("deepBox: element-field write + touchElem still triggers reactivity in fine mode", async () => {
  const c = createContext(null);
  const d = deepBox(c, 0, [{ id: 1, label: "a" }, { id: 2, label: "b" }]);
  d.observeElems();
  let runs = 0;
  bind(c, [0], () => { void d.v.length; runs++; });
  const r = d.v.slice();
  d.v = r; // a read-store cycle (like a swap)
  await tick();
  const before = runs;
  d.v[0].label = "changed";
  d.touchElem(d.v[0]); // element-field mutation of a direct element
  await tick();
  assert.ok(runs > before, "element-field write + touchElem flushes dependents");
  assert.ok(d._elems.has(d.v[0]), "the touched element is recorded for fine patching");
});

console.log("boxes.deep.test.mjs: all " + passed + " tests passed");
