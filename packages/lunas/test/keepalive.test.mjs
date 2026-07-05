// keepalive.test.mjs — component instance caching + LRU + activate/deactivate.
// Run: node packages/lunas/test/keepalive.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { keepAlive } from "../src/keepalive.mjs";
import {
  onDestroy,
  onActivated,
  onDeactivated,
  onMount,
} from "../src/lifecycle.mjs";

installDom();

// A component factory that records lifecycle events into `log`, tagged by id.
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
  c.root._lunasAttached = true; // live host so onMount fires
  const anchor = anchorAppend(c.root);
  return { c, container: c.root, anchor };
}

// Serialize non-anchor children.
const shown = (container) =>
  container.childNodes
    .filter((n) => !(n.kind === "text" && n.data === ""))
    .map((n) => n.getAttribute("data-id"))
    .join(",");

test("cache hit preserves node identity (no rebuild)", () => {
  const { c, container, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  const A = comp("A", log);

  const h1 = ka.show(c, anchor, "A", A, {});
  const nodeA = h1.root;
  assert.equal(shown(container), "A");

  // Switch away to B, then back to A: same node object comes back.
  ka.show(c, anchor, "B", comp("B", log), {});
  assert.equal(shown(container), "B");
  const h2 = ka.show(c, anchor, "A", A, {});
  assert.strictEqual(h2.root, nodeA); // SAME node, not rebuilt
  assert.equal(shown(container), "A");
});

test("deactivate detaches without destroying; activate re-attaches", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp("A", log), {}); // mount + activate A
  ka.show(c, anchor, "B", comp("B", log), {}); // deactivate A, mount+activate B
  ka.show(c, anchor, "A", comp("A", log), {}); // reactivate A (no rebuild/destroy)

  // A was never destroyed; it deactivated then reactivated.
  assert.deepEqual(log, [
    "mount:A",
    "activate:A",
    "deactivate:A",
    "mount:B",
    "activate:B",
    "deactivate:B",
    "activate:A",
  ]);
  assert.equal(log.filter((e) => e === "destroy:A").length, 0);
});

test("LRU eviction fires onDestroy on the evicted instance", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 2 });
  ka.show(c, anchor, "A", comp("A", log), {});
  ka.show(c, anchor, "B", comp("B", log), {}); // cache: [A, B]
  assert.equal(ka.size, 2);
  ka.show(c, anchor, "C", comp("C", log), {}); // overflow → evict LRU (A)
  assert.equal(ka.size, 2);
  assert.ok(log.includes("destroy:A"), "A should be evicted+destroyed");
  assert.equal(ka.has("A"), false);
  assert.equal(ka.has("B"), true);
  assert.equal(ka.has("C"), true);
});

test("LRU order updates on access (recently shown is kept)", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({ max: 2 });
  ka.show(c, anchor, "A", comp("A", log), {}); // [A]
  ka.show(c, anchor, "B", comp("B", log), {}); // [A, B]
  ka.show(c, anchor, "A", comp("A", log), {}); // touch A → [B, A]
  ka.show(c, anchor, "C", comp("C", log), {}); // evict LRU = B
  assert.equal(ka.has("A"), true);
  assert.equal(ka.has("B"), false);
  assert.equal(ka.has("C"), true);
});

test("destroy() evicts and destroys every cached instance", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp("A", log), {});
  ka.show(c, anchor, "B", comp("B", log), {});
  ka.destroy();
  assert.ok(log.includes("destroy:A"));
  assert.ok(log.includes("destroy:B"));
  assert.equal(ka.size, 0);
});

test("re-showing the same key does not re-mount or re-activate churn", () => {
  const { c, anchor } = host();
  const log = [];
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp("A", log), {});
  const before = log.slice();
  ka.show(c, anchor, "A", comp("A", log), {}); // same key, already active
  // No deactivate/activate churn for staying on the same key.
  assert.deepEqual(log, before);
});
