// lifecycle.test.mjs — onMount / onDestroy / onUpdate + attach + mountChild
// integration against the fake DOM. Run: node packages/lunas/test/lifecycle.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind, markVar } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { mountChild } from "../src/blocks.mjs";
import {
  onMount,
  onDestroy,
  onUpdate,
  attach,
} from "../src/lifecycle.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));

// A minimal component-like factory: builds a root, attaches a context, runs a
// setup that may register lifecycle hooks. Mimics dom.mjs component().
function makeComponent(tag, setup) {
  return (props) => {
    const root = document.createElement(tag);
    const c = createContext(root);
    root.__lunasCtx = c;
    if (setup) setup(c, props || {});
    return root;
  };
}

test("onMount fires after attach, not at construction", () => {
  const order = [];
  const factory = makeComponent("div", (c) => {
    order.push("setup");
    onMount(c, () => order.push("mount"));
  });
  const root = factory();
  assert.deepEqual(order, ["setup"]); // not mounted yet
  const host = document.createElement("body");
  attach(root, host);
  assert.deepEqual(order, ["setup", "mount"]);
});

test("onMount fires once even across a second attach attempt", () => {
  let mounts = 0;
  const factory = makeComponent("div", (c) => onMount(c, () => mounts++));
  const root = factory();
  const host = document.createElement("body");
  attach(root, host);
  attach(root, host); // idempotent runMount
  assert.equal(mounts, 1);
});

test("nested children: child onMount fires via a single top-level attach", () => {
  const order = [];
  const child = makeComponent("span", (c) => {
    onMount(c, () => order.push("child-mount"));
  });
  const parent = makeComponent("div", (c, props) => {
    onMount(c, () => order.push("parent-mount"));
    const anchor = anchorAppend(c.root);
    mountChild(c, anchor, child, {}); // detached at this point
  });
  const root = parent();
  assert.deepEqual(order, []); // nothing fired while detached
  attach(root, document.createElement("body"));
  // children fire before the parent (subtree-in-DOM semantics)
  assert.deepEqual(order, ["child-mount", "parent-mount"]);
});

test("mountChild into a live tree fires child onMount immediately", () => {
  const order = [];
  const child = makeComponent("span", (c) => onMount(c, () => order.push("m")));
  const parentCtx = createContext(document.createElement("div"));
  parentCtx.root._lunasAttached = true; // simulate a live parent
  const anchor = anchorAppend(parentCtx.root);
  mountChild(parentCtx, anchor, child, {});
  assert.deepEqual(order, ["m"]);
});

test("onDestroy fires exactly once on unmount", () => {
  let destroys = 0;
  const child = makeComponent("span", (c) => onDestroy(c, () => destroys++));
  const parentCtx = createContext(document.createElement("div"));
  const anchor = anchorAppend(parentCtx.root);
  const h = mountChild(parentCtx, anchor, child, {});
  h.unmount();
  h.unmount(); // second unmount must not re-fire
  assert.equal(destroys, 1);
});

test("parent unmount tears down nested child onDestroy", () => {
  const order = [];
  const grandchild = makeComponent("i", (c) =>
    onDestroy(c, () => order.push("gc"))
  );
  const child = makeComponent("span", (c) => {
    onDestroy(c, () => order.push("c"));
    const a = anchorAppend(c.root);
    mountChild(c, a, grandchild, {});
  });
  const parentCtx = createContext(document.createElement("div"));
  const a = anchorAppend(parentCtx.root);
  const h = mountChild(parentCtx, a, child, {});
  h.unmount();
  assert.deepEqual(order, ["gc", "c"]); // deepest first
});

test("onUpdate fires after a flush that ran updates", async () => {
  let updates = 0;
  const c = createContext(document.createElement("div"));
  onUpdate(c, () => updates++);
  let seen = 0;
  bind(c, [0], () => {
    seen++;
  });
  assert.equal(updates, 0); // bind's initial run is not an update flush
  markVar(c, 0);
  await tick();
  assert.equal(updates, 1);
  markVar(c, 0);
  await tick();
  assert.equal(updates, 2); // re-arms every flush
});

test("onUpdate does not fire when a flush ran no updates", async () => {
  let updates = 0;
  const c = createContext(document.createElement("div"));
  onUpdate(c, () => updates++);
  // Schedule an empty flush via afterFlush-style pending (mark then nothing dep).
  markVar(c, 99); // no binds on index 99 → empty queue
  await tick();
  assert.equal(updates, 0);
});

test("onMount registered after attach still runs (deferred microtask)", async () => {
  const root = document.createElement("div");
  const c = createContext(root);
  root.__lunasCtx = c;
  attach(root, document.createElement("body"));
  let ran = false;
  onMount(c, () => {
    ran = true;
  });
  assert.equal(ran, false); // deferred
  await tick();
  assert.equal(ran, true);
});
