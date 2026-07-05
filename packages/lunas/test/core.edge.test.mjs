// core.edge.test.mjs — adjacency dispatch edge cases beyond core.test.mjs:
// multi-bind fan-out/fan-in, dedup nuances, unbind/bind mutations mid-flush,
// nested scope teardown ordering, afterFlush semantics.
// Run: node packages/lunas/test/core.edge.test.mjs

import assert from "node:assert";
import {
  createContext,
  bind,
  markVar,
  flush,
  unbind,
  afterFlush,
  beginScope,
  endScope,
  dropScope,
  runScope,
} from "../src/core.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

await test("multiple binds on one var: all run, each exactly once per flush", async () => {
  const c = createContext(null);
  let a = 0;
  let b = 0;
  let d = 0;
  bind(c, [0], () => a++);
  bind(c, [0], () => b++);
  bind(c, [0], () => d++);
  markVar(c, 0);
  await tick();
  assert.strictEqual(a, 2);
  assert.strictEqual(b, 2);
  assert.strictEqual(d, 2);
});

await test("one bind on many vars: fan-in, each markVar dedups to one run per flush", async () => {
  const c = createContext(null);
  let runs = 0;
  bind(c, [0, 1, 2, 3], () => runs++);
  markVar(c, 0);
  markVar(c, 1);
  markVar(c, 2);
  markVar(c, 3);
  await tick();
  assert.strictEqual(runs, 2, "initial + exactly one coalesced run");
});

await test("markVar on a var with no binds is a harmless no-op", async () => {
  const c = createContext(null);
  // deps[5] was never populated by any bind() call.
  assert.doesNotThrow(() => markVar(c, 5));
  await tick();
});

await test("markVar on an index that only ever had unbound binds", async () => {
  const c = createContext(null);
  const s = bind(c, [0], () => {});
  unbind(c, s);
  assert.doesNotThrow(() => markVar(c, 0));
  await tick();
});

await test("flush() can be called directly/synchronously (bypassing markVar scheduling)", () => {
  const c = createContext(null);
  let runs = 0;
  const s = bind(c, [0], () => runs++);
  c.queue.push(s);
  s.q = true;
  flush(c);
  assert.strictEqual(runs, 2);
  assert.strictEqual(c.queue.length, 0, "queue drained");
  assert.strictEqual(c.pending, false);
});

await test("flush() with empty queue is a safe no-op and does not touch onUpdate", () => {
  const c = createContext(null);
  let updates = 0;
  c.onUpdate = () => updates++;
  flush(c);
  assert.strictEqual(updates, 0, "onUpdate only fires when something actually ran");
});

await test("bind added during a flush: new bind is registered in deps[i] before the outer bind finishes registering", async () => {
  // bind(c, deps, fn) runs fn() BEFORE pushing itself into c.deps[i]. So if
  // s1's own fn synchronously registers s2 (also on var 0), s2 gets pushed
  // into c.deps[0] first (during s1's registration-time run), and s1 is only
  // pushed once its own bind() call returns. That flips adjacency order for
  // this one var: c.deps[0] === [s2, s1], not [s1, s2].
  const c = createContext(null);
  const log = [];
  let added = null;
  bind(c, [0], () => {
    log.push("first");
    if (!added) {
      added = bind(c, [0], () => log.push("second"));
    }
  });
  assert.deepStrictEqual(c.deps[0].map((s) => s === added), [true, false], "s2 precedes s1 in deps[0]");
  log.length = 0;
  markVar(c, 0);
  await tick();
  assert.deepStrictEqual(log, ["second", "first"], "flush runs in deps[] order: s2 then s1");
  log.length = 0;
  markVar(c, 0);
  await tick();
  assert.deepStrictEqual(log, ["second", "first"], "both now subscribed for every future flush");
});

await test("unbind mid-flush: a live bind unbinds another queued bind before it runs", async () => {
  const c = createContext(null);
  const log = [];
  let sTarget;
  bind(c, [0], () => {
    log.push("killer");
    if (sTarget) unbind(c, sTarget);
  });
  sTarget = bind(c, [0], () => log.push("victim"));
  log.length = 0;
  markVar(c, 0);
  await tick();
  // "killer" is queued before "victim" (registered first), so it runs first
  // and unbinds victim before victim's queued turn -> victim's fn is skipped
  // because flush() checks s.alive before invoking fn.
  assert.deepStrictEqual(log, ["killer"], "victim skipped: unbound before its turn");
});

await test("unbind is idempotent: calling twice does not throw or double-splice", () => {
  const c = createContext(null);
  const s = bind(c, [0, 1], () => {});
  unbind(c, s);
  assert.doesNotThrow(() => unbind(c, s));
  assert.strictEqual(c.deps[0].length, 0);
  assert.strictEqual(c.deps[1].length, 0);
});

await test("afterFlush: callback rides an already-pending flush", async () => {
  const c = createContext(null);
  const order = [];
  bind(c, [0], () => order.push("paint"));
  order.length = 0; // drop the immediate registration-time run
  markVar(c, 0); // schedules a flush
  afterFlush(c, () => order.push("post"));
  await tick();
  assert.deepStrictEqual(order, ["paint", "post"], "post-flush cb runs after the update pass");
});

await test("afterFlush: with nothing pending, still schedules its own flush", async () => {
  const c = createContext(null);
  let ran = false;
  afterFlush(c, () => {
    ran = true;
  });
  assert.strictEqual(ran, false, "not synchronous");
  await tick();
  assert.strictEqual(ran, true);
});

await test("afterFlush: multiple callbacks run in registration order", async () => {
  const c = createContext(null);
  const order = [];
  afterFlush(c, () => order.push(1));
  afterFlush(c, () => order.push(2));
  afterFlush(c, () => order.push(3));
  await tick();
  assert.deepStrictEqual(order, [1, 2, 3]);
});

await test("afterFlush callback that registers another afterFlush: runs via its own fresh microtask flush", async () => {
  // c.post is nulled out before draining, so a callback that calls afterFlush
  // again schedules a brand-new flush (c.pending was false again by then).
  // That's a fresh queueMicrotask, which still drains before our
  // setTimeout-based `tick()` helper resolves -- microtasks fully empty
  // before any macrotask runs -- so both callbacks are visible after one tick.
  const c = createContext(null);
  const order = [];
  afterFlush(c, () => {
    order.push("first");
    afterFlush(c, () => order.push("second"));
  });
  await tick();
  assert.deepStrictEqual(order, ["first", "second"], "nested afterFlush resolves within the same tick() wait");
});

await test("nested scopes: dropping a parent scope tears down all descendants", async () => {
  const c = createContext(null);
  let a = 0;
  let b = 0;
  let d = 0;
  const top = beginScope(c);
  bind(c, [0], () => a++);
  const mid = beginScope(c);
  bind(c, [0], () => b++);
  const leaf = beginScope(c);
  bind(c, [0], () => d++);
  endScope(c); // close leaf
  endScope(c); // close mid
  endScope(c); // close top
  void mid;
  void leaf;
  markVar(c, 0);
  await tick();
  assert.strictEqual(a, 2);
  assert.strictEqual(b, 2);
  assert.strictEqual(d, 2);
  dropScope(c, top);
  markVar(c, 0);
  await tick();
  assert.strictEqual(a, 2, "top-level scoped bind dead");
  assert.strictEqual(b, 2, "mid-level scoped bind dead too");
  assert.strictEqual(d, 2, "leaf-level scoped bind dead too");
});

await test("dropping an inner scope leaves sibling/outer scopes untouched", async () => {
  const c = createContext(null);
  let outer = 0;
  let inner = 0;
  const outerScope = beginScope(c);
  bind(c, [0], () => outer++);
  const innerScope = beginScope(c);
  bind(c, [0], () => inner++);
  endScope(c);
  endScope(c);
  dropScope(c, innerScope);
  markVar(c, 0);
  await tick();
  assert.strictEqual(outer, 2, "outer scope still live after inner dropped");
  assert.strictEqual(inner, 1, "inner scope's bind is dead, no rerun");
  // outerScope's children array should have had innerScope spliced out.
  assert.strictEqual(outerScope.children.includes(innerScope), false);
});

await test("scope dropped, then dropped again: safe no-op (empty arrays)", () => {
  const c = createContext(null);
  const scope = beginScope(c);
  bind(c, [0], () => {});
  endScope(c);
  dropScope(c, scope);
  assert.doesNotThrow(() => dropScope(c, scope));
});

await test("c.scope restored correctly after endScope even with nested begin/end", () => {
  const c = createContext(null);
  assert.strictEqual(c.scope, null);
  const s1 = beginScope(c);
  assert.strictEqual(c.scope, s1);
  const s2 = beginScope(c);
  assert.strictEqual(c.scope, s2);
  endScope(c);
  assert.strictEqual(c.scope, s1, "back to s1 after closing s2");
  endScope(c);
  assert.strictEqual(c.scope, null, "back to null after closing s1");
});

await test("endScope on a context with no open scope is a safe no-op", () => {
  const c = createContext(null);
  assert.doesNotThrow(() => endScope(c));
  assert.strictEqual(c.scope, null);
});

await test("runScope: re-runs live binds in a scope and its children, skips dead ones", async () => {
  const c = createContext(null);
  const log = [];
  const scope = beginScope(c);
  const s1 = bind(c, [0], () => log.push("s1"));
  const child = beginScope(c);
  bind(c, [0], () => log.push("child"));
  endScope(c);
  endScope(c);
  log.length = 0;
  unbind(c, s1);
  runScope(c, scope);
  assert.deepStrictEqual(log, ["child"], "dead bind s1 skipped, live child bind ran");
});

await test("runScope: a sub that creates a new child scope on every call accumulates children, each snapshot only sees prior ones", () => {
  // bind/beginScope/endScope run synchronously and immediately at
  // registration, so the *first* nested child already exists in
  // scope.children before runScope is ever invoked. runScope() itself
  // snapshots scope.children up front (before running subs), so a child
  // created *during this call's* subs loop is deferred to the *next*
  // runScope call rather than visited in the same pass -- this is what makes
  // runScope safe against a sub that conjures new scopes on the fly (e.g. an
  // ifBlock flipping on) without infinite/duplicate recursion in one pass.
  const c = createContext(null);
  const log = [];
  const scope = beginScope(c);
  bind(c, [0], () => {
    log.push("dynamic");
    const inner = beginScope(c);
    bind(c, [0], () => log.push("inner"));
    endScope(c);
    void inner;
  });
  endScope(c);
  assert.strictEqual(scope.children.length, 1, "registration already created one child");
  log.length = 0;
  runScope(c, scope);
  // subs loop reruns "dynamic", which calls beginScope/endScope again ->
  // logs "inner" for the freshly-registered bind. But runScope() does NOT
  // set c.scope to `scope` while walking (it calls s.fn() directly, not
  // through begin/endScope), so that fresh beginScope() call attaches to
  // whatever c.scope ambiently is (null here, since we're outside any
  // begin/endScope block) rather than to `scope`. It is therefore never
  // added to scope.children and is orphaned relative to this scope tree.
  // Meanwhile runScope's children snapshot (taken before the subs loop) has
  // just the one pre-existing child from registration time -> visiting it
  // reruns its bind, logging "inner" a second time.
  assert.deepStrictEqual(log, ["dynamic", "inner", "inner"]);
  assert.strictEqual(scope.children.length, 1, "scope.children unchanged: the new nested scope attached to null, not this scope");
});

await test("dropScope on a scope whose child was already dropped independently", () => {
  const c = createContext(null);
  const parent = beginScope(c);
  bind(c, [0], () => {});
  const child = beginScope(c);
  bind(c, [0], () => {});
  endScope(c);
  endScope(c);
  dropScope(c, child); // drop child first
  assert.doesNotThrow(() => dropScope(c, parent)); // then parent
  assert.strictEqual(parent.children.length, 0);
});

console.log("core.edge.test.mjs: all " + passed + " tests passed");
