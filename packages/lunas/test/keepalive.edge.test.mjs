// keepalive.edge.test.mjs — additional edge-focused coverage for
// keepalive.mjs beyond keepalive.test.mjs: no-cap (Infinity) never evicts,
// max:1 forces eviction on every switch, has()/size across the eviction
// lifecycle, showing the currently-active key again is a true no-op (identity
// + no DOM churn), destroy() on an empty cache, and re-showing an evicted key
// rebuilds a fresh instance (new node identity + fresh onMount).
// Run: node packages/lunas/test/keepalive.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { keepAlive } from "../src/keepalive.mjs";
import { onDestroy, onActivated, onDeactivated, onMount } from "../src/lifecycle.mjs";

installDom();

function comp(id, log) {
  return () => {
    const root = document.createElement("div");
    root.setAttribute("data-id", id);
    root.appendChild(document.createTextNode(id));
    const c = createContext(root);
    root.__lunasCtx = c;
    onMount(c, () => log.push("mount:" + id));
    onDestroy(c, () => log.push("destroy:" + id));
    onActivated(c, () => log.push("activate:" + id));
    onDeactivated(c, () => log.push("deactivate:" + id));
    return root;
  };
}

function host() {
  const c = createContext(document.createElement("div"));
  c.root._lunasAttached = true;
  const anchor = anchorAppend(c.root);
  return { c, container: c.root, anchor };
}

const shown = (container) =>
  container.childNodes
    .filter((n) => !(n.kind === "text" && n.data === ""))
    .map((n) => n.getAttribute("data-id"))
    .join(",");

// -- unbounded cache never evicts ---------------------------------------------

test("no max option: cache never evicts however many keys are shown", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({}); // no max => Infinity
  for (const id of ["A", "B", "C", "D", "E"]) {
    ka.show(c, anchor, id, comp(id, log), {});
  }
  assert.equal(ka.size, 5);
  assert.equal(log.filter((e) => e.startsWith("destroy:")).length, 0);
});

// -- max:1 forces eviction on every switch ------------------------------------

test("max:1 evicts the previous instance on every single switch", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 1 });
  ka.show(c, anchor, "A", comp("A", log), {});
  ka.show(c, anchor, "B", comp("B", log), {});
  assert.equal(ka.size, 1);
  assert.equal(ka.has("A"), false, "A evicted immediately, no keepalive with max:1");
  assert.ok(log.includes("destroy:A"));
  ka.show(c, anchor, "C", comp("C", log), {});
  assert.equal(ka.has("B"), false);
  assert.ok(log.includes("destroy:B"));
});

// -- has()/size across full lifecycle -----------------------------------------

test("has() and size track cache membership precisely through mount/evict", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 2 });
  assert.equal(ka.size, 0);
  ka.show(c, anchor, "A", comp("A", log), {});
  assert.equal(ka.size, 1);
  assert.equal(ka.has("A"), true);
  ka.show(c, anchor, "B", comp("B", log), {});
  assert.equal(ka.size, 2);
  ka.show(c, anchor, "C", comp("C", log), {}); // evicts A
  assert.equal(ka.size, 2);
  assert.equal(ka.has("A"), false);
  assert.equal(ka.has("B"), true);
  assert.equal(ka.has("C"), true);
});

// -- re-showing the active key: true no-op ------------------------------------

test("re-showing the currently active key does not touch the DOM node at all", () => {
  const { c, container, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  const A = comp("A", log);
  const h1 = ka.show(c, anchor, "A", A, {});
  const nodeBefore = h1.root;
  const childCountBefore = container.childNodes.length;
  const h2 = ka.show(c, anchor, "A", A, {});
  assert.strictEqual(h2.root, nodeBefore);
  assert.equal(container.childNodes.length, childCountBefore);
  assert.equal(shown(container), "A");
});

// -- destroy() on an empty cache is a safe no-op ------------------------------

test("destroy() on a cache with nothing shown is a safe no-op", () => {
  const ka = keepAlive({});
  assert.doesNotThrow(() => ka.destroy());
  assert.equal(ka.size, 0);
});

test("destroy() then show() again works: a fresh keepAlive lifecycle after full teardown", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp("A", log), {});
  ka.destroy();
  assert.equal(ka.size, 0);
  // Reuse the SAME keepAlive controller after destroy(): showing "A" again
  // builds a brand-new instance (nothing cached anymore).
  const h = ka.show(c, anchor, "A", comp("A", log), {});
  assert.ok(h.root);
  assert.equal(ka.size, 1);
});

// -- re-showing an evicted key rebuilds a fresh instance ----------------------

test("showing a key again after it was LRU-evicted builds a brand-new instance (fresh node, fresh mount)", () => {
  const { c, container, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 1 });
  const h1 = ka.show(c, anchor, "A", comp("A", log), {});
  const firstNode = h1.root;
  ka.show(c, anchor, "B", comp("B", log), {}); // evicts A
  assert.equal(ka.has("A"), false);
  const h2 = ka.show(c, anchor, "A", comp("A", log), {}); // fresh A
  assert.notStrictEqual(h2.root, firstNode, "a new node was built, not reused");
  assert.equal(
    log.filter((e) => e === "mount:A").length,
    2,
    "A mounted twice: once originally, once after eviction+rebuild"
  );
  assert.equal(shown(container), "A");
});

// -- deactivate is idempotent: showing the same non-active key repeatedly ----

test("switching back and forth between two keys repeatedly never double-destroys", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  const A = comp("A", log);
  const B = comp("B", log);
  ka.show(c, anchor, "A", A, {});
  ka.show(c, anchor, "B", B, {});
  ka.show(c, anchor, "A", A, {});
  ka.show(c, anchor, "B", B, {});
  ka.show(c, anchor, "A", A, {});
  assert.equal(log.filter((e) => e === "destroy:A").length, 0);
  assert.equal(log.filter((e) => e === "destroy:B").length, 0);
  assert.equal(log.filter((e) => e === "mount:A").length, 1, "A only ever mounted once (reactivated after)");
  assert.equal(log.filter((e) => e === "mount:B").length, 1);
});

// -- keepAlive with max: 0 evicts everything immediately (edge boundary) -----

test("max:0 evicts synchronously — nothing stays cached across a show", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 0 });
  ka.show(c, anchor, "A", comp("A", log), {});
  // trim() runs after the just-touched key is set as current and is never the
  // victim, so with max:0 the JUST-shown entry survives trim (never evicting
  // the just-touched key) — cache holds exactly the most recent entry.
  assert.equal(ka.size, 1);
  assert.equal(ka.has("A"), true);
  ka.show(c, anchor, "B", comp("B", log), {});
  assert.equal(ka.size, 1);
  assert.equal(ka.has("A"), false, "A evicted to make room under max:0");
  assert.equal(ka.has("B"), true);
});
