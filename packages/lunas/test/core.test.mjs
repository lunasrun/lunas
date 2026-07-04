// core.test.mjs — adjacency dispatch tests.
// Run: node packages/lunas/test/core.test.mjs

import assert from "node:assert";
import {
  createContext,
  bind,
  markVar,
  unbind,
  beginScope,
  endScope,
  dropScope,
} from "../src/core.mjs";

// wait for all pending microtasks (flush is queueMicrotask-scheduled)
const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("bind runs fn once immediately", () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0], () => runs++);
  assert.strictEqual(runs, 1);
});

await test("only affected binds run", async () => {
  const c = createContext(null);
  let a = 0;
  let b = 0;
  bind(c, [0], () => a++);
  bind(c, [1], () => b++);
  assert.strictEqual(a + b, 2); // initial runs
  markVar(c, 0);
  await tick();
  assert.strictEqual(a, 2, "dep-0 bind reran");
  assert.strictEqual(b, 1, "dep-1 bind did NOT rerun");
});

await test("multi-dep bind runs when any dep changes", async () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0, 2], () => runs++);
  markVar(c, 2);
  await tick();
  assert.strictEqual(runs, 2);
  markVar(c, 0);
  await tick();
  assert.strictEqual(runs, 3);
});

await test("dedup within a flush: same var marked twice -> one run", async () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0], () => runs++);
  markVar(c, 0);
  markVar(c, 0);
  await tick();
  assert.strictEqual(runs, 2, "initial + exactly one flush run");
});

await test("dedup across vars: bind on [0,1], both marked -> one run", async () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0, 1], () => runs++);
  markVar(c, 0);
  markVar(c, 1);
  await tick();
  assert.strictEqual(runs, 2);
});

await test("microtask batching: N synchronous writes -> one flush pass", async () => {
  const c = createContext(null);
  const log = [];
  bind(c, [0], () => log.push("a"));
  bind(c, [1], () => log.push("b"));
  log.length = 0;
  markVar(c, 0);
  markVar(c, 1);
  markVar(c, 0);
  assert.deepStrictEqual(log, [], "nothing runs synchronously");
  await tick();
  assert.deepStrictEqual(log, ["a", "b"], "each ran exactly once, in order");
});

await test("unbind stops delivery", async () => {
  const c = createContext(null);
  let runs = 0;
  const s = bind(c, [0], () => runs++);
  markVar(c, 0);
  await tick();
  assert.strictEqual(runs, 2);
  unbind(c, s);
  markVar(c, 0);
  await tick();
  assert.strictEqual(runs, 2, "no run after unbind");
});

await test("unbind while queued: pending flush skips the dead bind", async () => {
  const c = createContext(null);
  let runs = 0;
  const s = bind(c, [0], () => runs++);
  markVar(c, 0); // s is now queued
  unbind(c, s); // killed before the microtask flush
  await tick();
  assert.strictEqual(runs, 1, "only the initial run");
});

await test("scopes collect binds; dropScope unbinds recursively", async () => {
  const c = createContext(null);
  let outer = 0;
  let inner = 0;
  bind(c, [0], () => outer++);
  const scope = beginScope(c);
  bind(c, [0], () => inner++);
  const child = beginScope(c); // nested scope (e.g. nested block)
  bind(c, [0], () => inner++);
  endScope(c);
  endScope(c);
  void child;
  markVar(c, 0);
  await tick();
  assert.strictEqual(outer, 2);
  assert.strictEqual(inner, 4);
  dropScope(c, scope);
  markVar(c, 0);
  await tick();
  assert.strictEqual(outer, 3, "outer bind still live");
  assert.strictEqual(inner, 4, "scoped binds (incl. nested) dead");
});

console.log("core.test.mjs: all " + passed + " tests passed");
