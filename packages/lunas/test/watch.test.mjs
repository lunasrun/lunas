// watch.test.mjs — user-facing watchers.
// Run: node packages/lunas/test/watch.test.mjs

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

await test("watch fires on change, not on registration by default", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  watch(c, [0], () => runs++);
  assert.strictEqual(runs, 0, "no immediate run");
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 1);
});

await test("watch { immediate: true } runs once at registration", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  watch(c, [0], () => runs++, { immediate: true });
  assert.strictEqual(runs, 1, "ran immediately");
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 2, "and again on change");
});

await test("watch stop() unbinds", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  const stop = watch(c, [0], () => runs++);
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 1);
  stop();
  a.v = 3;
  await tick();
  assert.strictEqual(runs, 1, "no run after stop()");
});

await test("watch is torn down by dropScope", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  let runs = 0;
  const scope = beginScope(c);
  watch(c, [0], () => runs++);
  endScope(c);
  a.v = 2;
  await tick();
  assert.strictEqual(runs, 1);
  dropScope(c, scope);
  a.v = 3;
  await tick();
  assert.strictEqual(runs, 1, "scoped watcher dead after dropScope");
});

await test("watch multi-dep fires on any dep", async () => {
  const c = createContext(null);
  const a = box(c, 0, 1);
  const b = box(c, 1, 1);
  let runs = 0;
  watch(c, [0, 1], () => runs++);
  a.v = 9;
  await tick();
  assert.strictEqual(runs, 1);
  b.v = 9;
  await tick();
  assert.strictEqual(runs, 2);
});

await test("watchEffect runs immediately and on change", async () => {
  const c = createContext(null);
  const a = box(c, 0, 5);
  const seen = [];
  const stop = watchEffect(c, [0], () => seen.push(a.v));
  assert.deepStrictEqual(seen, [5], "effect ran on registration");
  a.v = 6;
  await tick();
  assert.deepStrictEqual(seen, [5, 6]);
  stop();
  a.v = 7;
  await tick();
  assert.deepStrictEqual(seen, [5, 6], "no run after stop()");
});

console.log("watch.test.mjs: all " + passed + " tests passed");
