// dom.mountchild.test.mjs — mountChild lifecycle & wiring (src/blocks.mjs):
// mount/unmount, multi-root (fragment) child, onDestroy fires exactly once,
// parent link + _children registration, and unmount removing all nodes.
// Uses the fake DOM shim so fragment()/component() innerHTML parsing works.
// Run: node packages/lunas/test/dom.mountchild.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
const document = installDom();

import { createContext, bind, beginScope, endScope, dropScope } from "../src/core.mjs";
import { box, prop } from "../src/boxes.mjs";
import { anchorAppend, component, fragment } from "../src/dom.mjs";
import { mountChild } from "../src/blocks.mjs";
import { onDestroy, onMount, attach, isLive } from "../src/lifecycle.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));
const shape = (p) =>
  p.childNodes
    .map((n) => (n.kind === "text" ? (n.data === "" ? "|" : n.data) : "<" + n.tag + ">"))
    .join(" ");

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- basic mount / unmount ---------------------------------------------------

await test("mountChild inserts a single-root child before the anchor; unmount removes it", () => {
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  const Child = component("kid", {}, "<span></span>", () => {});
  const m = mountChild(c, anchor, Child, {});
  assert.strictEqual(shape(host), "<kid> |");
  assert.ok(m.ctx, "handle exposes the child context");
  m.unmount();
  assert.strictEqual(shape(host), "|", "child removed on unmount");
});

// --- parent link + _children registration -----------------------------------

await test("mountChild links child.ctx.parent to the parent and registers under _children", () => {
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  const Child = component("kid", {}, "<i></i>", () => {});
  const m = mountChild(c, anchor, Child, {});
  assert.strictEqual(m.ctx.parent, c, "child.parent points at the mounting context");
  assert.ok(c._children && c._children.includes(m.ctx), "child registered on _children");
  m.unmount();
  assert.ok(!c._children.includes(m.ctx), "child de-registered on unmount");
});

// --- multi-root (fragment) child mounts/unmounts as a group ------------------

await test("mountChild handles a multi-root fragment child (all nodes travel together)", () => {
  const c = createContext(null);
  const host = document.createElement("div");
  host.appendChild(document.createTextNode("H"));
  const anchor = anchorAppend(host);
  host.appendChild(document.createTextNode("T"));
  const Child = fragment({}, "<a></a><b></b><c></c>", () => {});
  const m = mountChild(c, anchor, Child, {});
  assert.strictEqual(shape(host), "H <a> <b> <c> | T", "all fragment nodes inserted");
  m.unmount();
  assert.strictEqual(shape(host), "H | T", "unmount removed every fragment node");
});

// --- onDestroy fires exactly once --------------------------------------------

await test("child onDestroy fires exactly once on unmount", () => {
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  let destroys = 0;
  const Child = component("kid", {}, "<x></x>", (cc) => {
    onDestroy(cc, () => destroys++);
  });
  const m = mountChild(c, anchor, Child, {});
  assert.strictEqual(destroys, 0, "not destroyed while mounted");
  m.unmount();
  assert.strictEqual(destroys, 1, "onDestroy ran once");
  m.unmount(); // idempotent: runDestroy guards with _destroyed
  assert.strictEqual(destroys, 1, "onDestroy does not fire a second time");
});

// --- onMount fires when inserted into a live tree ----------------------------

await test("child onMount fires immediately when the anchor is already live", () => {
  const parentRoot = component("root", {}, "<slot></slot>", () => {})({});
  const liveHost = document.createElement("body");
  attach(parentRoot, liveHost); // marks parentRoot live
  const c = parentRoot.__lunasCtx;
  const anchor = anchorAppend(parentRoot);

  let mounted = 0;
  const Child = component("kid", {}, "<x></x>", (cc) => {
    onMount(cc, () => mounted++);
  });
  assert.ok(isLive(anchor), "anchor is in a live tree");
  mountChild(c, anchor, Child, {});
  assert.strictEqual(mounted, 1, "onMount fired at mount time (live insertion point)");
});

await test("child onMount stays pending until an ancestor attach() drains it", () => {
  // Parent root is NOT attached yet. mountChild inserts the child into the
  // detached parent; the child's onMount must not fire until attach().
  const parentRoot = component("root", {}, "<hold></hold>", () => {})({});
  const c = parentRoot.__lunasCtx;
  const anchor = anchorAppend(parentRoot);
  let mounted = 0;
  const Child = component("kid", {}, "<x></x>", (cc) => {
    onMount(cc, () => mounted++);
  });
  mountChild(c, anchor, Child, {});
  assert.strictEqual(mounted, 0, "detached: onMount pending");
  attach(parentRoot, document.createElement("body"));
  assert.strictEqual(mounted, 1, "attach drained the child onMount");
});

// --- reactive prop bridge (setProp) ------------------------------------------

await test("mountChild.setProp pushes into the child's reactive prop box", async () => {
  const parent = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  let childText = null;
  const Child = (props) => {
    const root = document.createElement("kid");
    const cc = createContext(root);
    root.__lunasCtx = cc;
    const v = prop(cc, "v", 0, props.v, "def");
    const t = document.createTextNode("");
    root.appendChild(t);
    childText = t;
    bind(cc, [0], () => {
      t.data = String(v.v);
    });
    return root;
  };
  const n = box(parent, 0, 7);
  const m = mountChild(parent, anchor, Child, { v: () => n.v });
  bind(parent, [0], () => m.setProp("v", n.v));
  assert.strictEqual(childText.data, "7", "seeded from the getter");
  n.v = 12;
  await tick();
  assert.strictEqual(childText.data, "12", "parent write flowed into the child");
});

// --- unmount tears down a nested child chain (recursive destroy) -------------

await test("unmount recurses: a grandchild's onDestroy also fires", () => {
  const root = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  let grandDestroyed = 0;

  const Grand = component("grand", {}, "<g></g>", (gc) => {
    onDestroy(gc, () => grandDestroyed++);
  });
  const Child = component("child", {}, "<c></c>", (cc) => {
    // The child mounts a grandchild during its own setup, linking it under cc.
    const gAnchor = anchorAppend(cc.root);
    mountChild(cc, gAnchor, Grand, {});
  });

  const m = mountChild(root, anchor, Child, {});
  assert.strictEqual(grandDestroyed, 0);
  m.unmount();
  assert.strictEqual(grandDestroyed, 1, "grandchild destroy fired via recursion");
});

// --- never-panic: a non-object `c` (a primitive :for item) must not throw -----

await test("mountChild does not throw when `c` is a primitive (never sets _children on a number)", () => {
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  let destroyed = 0;
  const Child = component("kid", {}, "<n></n>", (cc) => {
    onDestroy(cc, () => destroyed++);
  });
  // Passing a primitive as `c` used to crash: "Cannot create property
  // '_children' on number". It must mount cleanly and still drive teardown.
  let m;
  assert.doesNotThrow(() => {
    m = mountChild(3, anchor, Child, {});
  }, "primitive c must not throw");
  assert.strictEqual(shape(host), "<kid> |", "child still mounted");
  m.unmount();
  assert.strictEqual(destroyed, 1, "onDestroy still fires via the handle");
  assert.strictEqual(shape(host), "|", "child removed on unmount");
});

// --- scope-tied teardown: dropping the enclosing scope unmounts the child -----

await test("dropping the enclosing scope unmounts a child mounted inside it (fires onDestroy, clears _children)", () => {
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  let destroyed = 0;
  const Child = component("kid", {}, "<x></x>", (cc) => {
    onDestroy(cc, () => destroyed++);
  });

  // Simulate a :for/:if item: open a scope, mount the child inside it.
  const scope = beginScope(c);
  const m = mountChild(c, anchor, Child, {});
  endScope(c);

  assert.strictEqual(shape(host), "<kid> |", "child mounted inside the item scope");
  assert.ok(c._children.includes(m.ctx), "child linked under parent _children");
  assert.strictEqual(destroyed, 0, "alive while the item is present");

  // Item removed: dropScope must tear the child down.
  dropScope(c, scope);
  assert.strictEqual(destroyed, 1, "onDestroy fired when the item scope dropped");
  assert.ok(!c._children.includes(m.ctx), "child de-registered from _children");
  assert.strictEqual(shape(host), "|", "child DOM removed");

  // Idempotent: a later explicit unmount (or a second drop) is a no-op.
  m.unmount();
  assert.strictEqual(destroyed, 1, "no double onDestroy");
});

console.log("\ndom.mountchild.test.mjs: " + passed + " passed.");
