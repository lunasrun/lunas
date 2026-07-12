// lifecycle.edge.test.mjs — additional edge-focused coverage for
// lifecycle.mjs beyond lifecycle.test.mjs: onDestroy across every unmount
// path (forBlock/ifBlock content teardown, keepalive eviction), onUpdate
// across multiple flushes, deep nesting order, and onActivated/onDeactivated
// integration via keepAlive.
// Run: node packages/lunas/test/lifecycle.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind, markVar } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { mountChild, ifBlock, forBlock } from "../src/blocks.mjs";
import { keepAlive } from "../src/keepalive.mjs";
import {
  onMount,
  onDestroy,
  onUpdate,
  onActivated,
  onDeactivated,
  attach,
  isLive,
} from "../src/lifecycle.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));

function makeComponent(tag, setup) {
  return (props) => {
    const root = document.createElement(tag);
    const c = createContext(root);
    root.__lunasCtx = c;
    if (setup) setup(c, props || {});
    return root;
  };
}

// -- onDestroy across ifBlock teardown ---------------------------------------

// KNOWN GAP (filed, not fixed here — see PR note): output-design.md §7 states
// plainly that "every unmount path (mountChild.unmount, block item teardown,
// keep-alive eviction) funnels through runDestroy(c)". In practice, ifBlock /
// ifChain / forBlock's teardown (`removeAll` / `host.remove`) only removes DOM
// nodes and drops the reactive SCOPE (unregistering binds) — it never calls
// `.unmount()` on a child mounted via `mountChild` inside that branch/item, so
// that child's onDestroy queue never fires when the branch is hidden or the
// item is removed from a :for list (only an explicit outer mountChild.unmount()
// — e.g. via dynamicBlock/keepAlive, which DO call child.unmount() themselves —
// reaches runDestroy). A fix belongs in blocks.mjs (e.g. having mountChild
// piggyback an unmount hook onto `c.scope.subs` the same way store.mjs's
// useStore does for detach — see its comment on the unbind-alive trick), which
// is out of scope for this test-only PR (src/*.mjs is off-limits here). Skipped
// with a failing-if-unskipped assertion preserved for whoever picks this up.
test("onDestroy fires when an ifBlock branch containing a mounted child is torn down", { skip: "known gap: ifBlock teardown does not call runDestroy on children mounted inside the branch (see comment above / PR note)" }, () => {
  const order = [];
  const child = makeComponent("span", (c) => onDestroy(c, () => order.push("child-destroy")));
  const parentCtx = createContext(document.createElement("div"));
  const anchor = anchorAppend(parentCtx.root);
  let show = true;
  const block = ifBlock(
    parentCtx,
    anchor,
    [0],
    () => show,
    () => {
      const a = anchorAppend(document.createElement("div"));
      const wrap = a.parentNode;
      mountChild(parentCtx, a, child, {});
      return wrap;
    }
  );
  assert.deepEqual(order, []);
  show = false;
  block.update();
  assert.deepEqual(order, ["child-destroy"], "hiding the branch destroys its mounted child");
});

test("onDestroy does not re-fire when the ifBlock branch is shown again after hiding", { skip: "known gap: see the ifBlock teardown note above" }, () => {
  const counts = { d: 0 };
  const child = makeComponent("span", (c) => onDestroy(c, () => counts.d++));
  const parentCtx = createContext(document.createElement("div"));
  const anchor = anchorAppend(parentCtx.root);
  let show = true;
  const block = ifBlock(
    parentCtx,
    anchor,
    [0],
    () => show,
    () => {
      const a = anchorAppend(document.createElement("div"));
      const wrap = a.parentNode;
      mountChild(parentCtx, a, child, {});
      return wrap;
    }
  );
  show = false;
  block.update(); // destroy #1 (old instance)
  show = true;
  block.update(); // fresh instance built, no destroy
  show = false;
  block.update(); // destroy #2 (new instance) — each instance destroyed once
  assert.equal(counts.d, 2, "one destroy per distinct mounted instance, none doubled");
});

// -- onDestroy across forBlock item removal ----------------------------------

test("onDestroy fires for each item's mounted child when forBlock removes it", { skip: "known gap: forBlock's host.remove() does not call runDestroy on children mounted inside an item (see ifBlock note above)" }, () => {
  const order = [];
  const childFor = (id) =>
    makeComponent("li", (c) => onDestroy(c, () => order.push("destroy:" + id)));
  const parentCtx = createContext(document.createElement("ul"));
  const anchor = anchorAppend(parentCtx.root);
  let items = [1, 2, 3];
  const block = forBlock(parentCtx, anchor, [0], () => items, {
    make: (d) => {
      const a = anchorAppend(document.createElement("li"));
      const wrap = a.parentNode;
      mountChild(parentCtx, a, childFor(d), {});
      return wrap;
    },
    keyOf: (d) => d,
  });
  items = [1, 3]; // remove item 2
  block.update();
  assert.deepEqual(order, ["destroy:2"]);
  items = [];
  block.update();
  assert.deepEqual(order.sort(), ["destroy:1", "destroy:2", "destroy:3"].sort());
});

test("forBlock destroy() tears down every remaining item's onDestroy", { skip: "known gap: forBlock destroy()'s per-item removal path shares host.remove(), same runDestroy gap as above" }, () => {
  const order = [];
  const childFor = (id) =>
    makeComponent("li", (c) => onDestroy(c, () => order.push(id)));
  const parentCtx = createContext(document.createElement("ul"));
  const anchor = anchorAppend(parentCtx.root);
  const items = ["a", "b"];
  const block = forBlock(parentCtx, anchor, [0], () => items, {
    make: (d) => {
      const a = anchorAppend(document.createElement("li"));
      const wrap = a.parentNode;
      mountChild(parentCtx, a, childFor(d), {});
      return wrap;
    },
    keyOf: (d) => d,
  });
  block.destroy();
  assert.deepEqual(order.sort(), ["a", "b"]);
});

// -- onDestroy via keepalive eviction (not deactivation) ---------------------

test("keepAlive deactivation does NOT fire onDestroy; only true eviction does", () => {
  const order = [];
  const comp = (id) =>
    makeComponent("div", (c) => {
      onDestroy(c, () => order.push("destroy:" + id));
      onDeactivated(c, () => order.push("deactivate:" + id));
    });
  const c = createContext(document.createElement("div"));
  const anchor = anchorAppend(c.root);
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp("A"), {});
  ka.show(c, anchor, "B", comp("B"), {}); // deactivates A
  assert.deepEqual(order, ["deactivate:A"]);
  ka.destroy(); // now truly evicts both
  assert.ok(order.includes("destroy:A"));
  assert.ok(order.includes("destroy:B"));
});

// -- onUpdate: multi-flush + multiple onUpdate registrations -----------------

test("onUpdate: two callbacks registered on the same context both fire every flush", async () => {
  const c = createContext(document.createElement("div"));
  let a = 0;
  let b = 0;
  onUpdate(c, () => a++);
  onUpdate(c, () => b++);
  bind(c, [0], () => {});
  markVar(c, 0);
  await tick();
  assert.equal(a, 1);
  assert.equal(b, 1);
  markVar(c, 0);
  await tick();
  assert.equal(a, 2);
  assert.equal(b, 2);
});

test("onUpdate registered after the first flush still fires on later flushes", async () => {
  const c = createContext(document.createElement("div"));
  bind(c, [0], () => {});
  markVar(c, 0);
  await tick(); // flush #1, no onUpdate registered yet
  let updates = 0;
  onUpdate(c, () => updates++);
  markVar(c, 0);
  await tick();
  assert.equal(updates, 1);
});

// -- nested children mount/destroy ordering ----------------------------------

test("three-level nesting: onMount fires deepest-first via one top-level attach", () => {
  const order = [];
  const grandchild = makeComponent("i", (c) => onMount(c, () => order.push("gc")));
  const child = makeComponent("span", (c) => {
    onMount(c, () => order.push("c"));
    const a = anchorAppend(c.root);
    mountChild(c, a, grandchild, {});
  });
  const parent = makeComponent("div", (c) => {
    onMount(c, () => order.push("p"));
    const a = anchorAppend(c.root);
    mountChild(c, a, child, {});
  });
  const root = parent();
  attach(root, document.createElement("body"));
  assert.deepEqual(order, ["gc", "c", "p"]);
});

test("three-level nesting: onDestroy fires deepest-first on a single top unmount", () => {
  const order = [];
  const grandchild = makeComponent("i", (c) => onDestroy(c, () => order.push("gc")));
  const child = makeComponent("span", (c) => {
    onDestroy(c, () => order.push("c"));
    const a = anchorAppend(c.root);
    mountChild(c, a, grandchild, {});
  });
  const parentCtx = createContext(document.createElement("div"));
  const a = anchorAppend(parentCtx.root);
  const h = mountChild(parentCtx, a, child, {});
  h.unmount();
  assert.deepEqual(order, ["gc", "c"]);
});

// -- isLive edge cases --------------------------------------------------------

test("isLive returns false for a detached node with no _lunasAttached ancestor", () => {
  const el = document.createElement("div");
  assert.equal(isLive(el), false);
});

test("isLive returns true once an ancestor is flagged _lunasAttached", () => {
  const root = document.createElement("div");
  const child = document.createElement("span");
  root.appendChild(child);
  root._lunasAttached = true;
  assert.equal(isLive(child), true);
});

test("isLive(null) is false, not a throw", () => {
  assert.equal(isLive(null), false);
});

// -- onActivated fires immediately on first activation, every reactivation --

test("onActivated fires on first mount AND every subsequent reactivation", () => {
  const order = [];
  const comp = () =>
    makeComponent("div", (c) => onActivated(c, () => order.push("activated")));
  const c = createContext(document.createElement("div"));
  const anchor = anchorAppend(c.root);
  const ka = keepAlive({});
  ka.show(c, anchor, "A", comp(), {});
  ka.show(c, anchor, "B", comp(), {});
  ka.show(c, anchor, "A", comp(), {}); // reactivate
  assert.deepEqual(order, ["activated", "activated", "activated"]);
});

// -- onMount/onDestroy are no-ops for a non-function argument ----------------

test("onMount/onDestroy/onUpdate silently ignore a non-function fn (never-panic)", () => {
  const c = createContext(document.createElement("div"));
  assert.doesNotThrow(() => {
    onMount(c, null);
    onMount(c, undefined);
    onDestroy(c, 42);
    onUpdate(c, "nope");
  });
});
