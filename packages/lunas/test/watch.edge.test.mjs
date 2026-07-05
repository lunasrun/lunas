// watch.edge.test.mjs — watch/watchEffect edge cases beyond watch.test.mjs:
// unrelated-var isolation, multi-dep coalescing, scope-drop cleanup
// interaction with stop(), async callback microtask ordering, double-stop
// safety, watch vs watchEffect immediate semantics under scopes.
// Run: node packages/lunas/test/watch.edge.test.mjs

import assert from "node:assert";
import {
  createContext,
  beginScope,
  endScope,
  dropScope,
} from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { watch, watchEffect } from "../src/watch.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("watch does not fire when an unrelated variable changes", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const b = box(c, 1, 1);
  let runs = 0;
  watch(c, [0], () => runs++);
  b.v = 99;
  await tick();
  assert.strictEqual(runs, 0, "watch(deps=[0]) ignores writes to index 1");
});

await test("watch multi-dep: both deps changing in the same tick still fires exactly once", async () => {
  const c = createContext(null);
  const x = box(c, 0, 1);
  const y = box(c, 1, 1);
  let runs = 0;
  watch(c, [0, 1], () => runs++);
  x.v = 2;
  y.v = 2;
  await tick();
  assert.strictEqual(runs, 1, "coalesced into one flush -> one callback invocation");
});

await test("watch with an async callback: the callback itself is fire-and-forget from the runtime's perspective, both halves land within the same tick() wait (microtask draining)", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const order = [];
  watch(c, [0], async () => {
    order.push("start");
    await Promise.resolve();
    order.push("end");
  });
  a.v = 2;
  await tick();
  assert.deepStrictEqual(order, ["start", "end"], "async continuation is itself a microtask, drains before the macrotask tick");
});

await test("watch async callback: a SECOND change fires a new async invocation independently (no queuing/collapsing of callback runs themselves)", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const order = [];
  watch(c, [0], async (n) => {
    order.push("start:" + a.v);
    await Promise.resolve();
    order.push("end:" + a.v);
  });
  a.v = 2;
  await tick();
  a.v = 3;
  await tick();
  assert.deepStrictEqual(order, ["start:2", "end:2", "start:3", "end:3"]);
});

await test("watchEffect: scope-drop stops it, and calling the returned stop() afterward is a safe no-op", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  const scope = beginScope(c);
  const stop = watchEffect(c, [0], () => runs++);
  endScope(c);
  assert.strictEqual(runs, 1, "immediate effect run at registration");
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 2);
  dropScope(c, scope);
  a.v = 3;
  await tick();
  assert.strictEqual(runs, 2, "dropScope tore it down: no run for the third write");
  assert.doesNotThrow(() => stop(), "stop() after the scope already dropped it must not throw");
  a.v = 4;
  await tick();
  assert.strictEqual(runs, 2, "still dead after the redundant stop() call");
});

await test("watch: scope-drop stops it even when registered with { immediate: true }", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  const scope = beginScope(c);
  watch(c, [0], () => runs++, { immediate: true });
  endScope(c);
  assert.strictEqual(runs, 1, "immediate run happened at registration");
  dropScope(c, scope);
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 1, "no further run: scope drop killed it before any change");
});

await test("watch: calling stop() twice does not throw and does not affect other watchers", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runsA = 0;
  let runsB = 0;
  const stopA = watch(c, [0], () => runsA++);
  watch(c, [0], () => runsB++);
  stopA();
  assert.doesNotThrow(() => stopA());
  a.v = 2;
  await tick();
  assert.strictEqual(runsA, 0, "stopped watcher never fires");
  assert.strictEqual(runsB, 1, "sibling watcher on the same dep is unaffected");
});

await test("watchEffect: multiple independent effects on the same dep all run once per change, in registration order", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const order = [];
  watchEffect(c, [0], () => order.push("first:" + a.v));
  watchEffect(c, [0], () => order.push("second:" + a.v));
  order.length = 0; // drop the two immediate registration-time runs
  a.v = 2;
  await tick();
  assert.deepStrictEqual(order, ["first:2", "second:2"]);
});

await test("watch: stopping one watcher mid-flush does not affect a sibling watcher on the same dep queued in the same pass", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let stopB;
  let runsA = 0;
  let runsB = 0;
  watch(c, [0], () => {
    runsA++;
    stopB(); // stop the sibling from inside this callback
  });
  stopB = watch(c, [0], () => runsB++);
  a.v = 2;
  await tick();
  assert.strictEqual(runsA, 1);
  // Whether runsB observes this flush depends on adjacency order (watcher A
  // is registered, hence queued, before watcher B), so A's callback runs
  // first and unbinds B before B's queued turn -- matching core.mjs's
  // documented "unbind mid-flush" semantics (see core.edge.test.mjs).
  assert.strictEqual(runsB, 0, "B unbound before its turn in this same flush pass");
  a.v = 3;
  await tick();
  assert.strictEqual(runsB, 0, "B permanently stopped, no further runs");
});

await test("watch immediate: the initial synchronous run happens before the function returns (not deferred to a microtask)", () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let ran = false;
  watch(c, [0], () => {
    ran = true;
  }, { immediate: true });
  assert.strictEqual(ran, true, "no await needed to observe the immediate run");
});

await test("watchEffect: dep read inside the effect reflects the value at the time of that run, not a stale closure", async () => {
  const c = createContext(null);
  const a = box(c, 0, "a");
  const seen = [];
  watchEffect(c, [0], () => seen.push(a.v));
  a.v = "b";
  a.v = "c"; // coalesced: effect should only see the final "c", not "b"
  await tick();
  assert.deepStrictEqual(seen, ["a", "c"], "intermediate write 'b' coalesced away");
});

console.log("watch.edge.test.mjs: all " + passed + " tests passed");
