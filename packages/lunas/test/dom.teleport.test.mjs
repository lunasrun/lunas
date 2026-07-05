// dom.teleport.test.mjs — teleportBlock teardown/leak coverage (src/blocks.mjs).
//
// teleportBlock moves its content to an external target (a selector resolved
// via document.querySelector, or an Element passed directly) instead of
// inlining it at its anchor. Because the content lives OUTSIDE the owning
// component's own subtree, none of the usual "remove this subtree" teardown
// paths (a :for/:if item's node removal, a component's own root removal on
// unmount) ever touch it — only explicit teardown wiring does. This file
// pins that wiring: teardown must fire (and the nodes must leave the target)
// when:
//   1. the OWNING COMPONENT unmounts directly (mountChild(...).unmount()),
//   2. the teleport call sits inside a :for item that gets removed,
//   3. the teleport call sits inside an :if branch that toggles off.
//
// Run: node packages/lunas/test/dom.teleport.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
const document = installDom();

import { createContext, bind, markVar } from "../src/core.mjs";
import { box, deepBox } from "../src/boxes.mjs";
import { component, anchorAppend } from "../src/dom.mjs";
import { teleportBlock, mountChild, ifBlock, forBlock } from "../src/blocks.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// Fresh portal target attached under document.body so "#portal" selector
// resolution (document.querySelector) works like it would in a real DOM.
function makePortal(id) {
  const portal = document.createElement("div");
  portal.setAttribute("id", id);
  document.body.appendChild(portal);
  return portal;
}

// --- 1. owning component unmounts directly (mountChild(...).unmount()) ------

await test("teleportBlock: owner unmount (mountChild.unmount()) removes teleported nodes from target", () => {
  const portal = makePortal("portal-owner-unmount");

  // A component whose setup calls teleportBlock at its own TOP LEVEL (no
  // enclosing :if/:for scope) — this is the case addDisposer alone can't
  // catch, since addDisposer is a no-op when no scope is open. Only the
  // onDestroy(c, destroy) wiring covers it.
  const Widget = component("div", {}, "", (c) => {
    teleportBlock(c, anchorAppend(c.root), () => "#portal-owner-unmount", () => {
      const n = document.createElement("p");
      n.setAttribute("data-owned", "1");
      return n;
    });
  });

  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  const parentCtx = createContext(null);
  const handle = mountChild(parentCtx, anchor, Widget, {});

  assert.strictEqual(portal.childNodes.length, 1, "teleported content landed in the target");
  assert.strictEqual(portal.childNodes[0].getAttribute("data-owned"), "1");

  handle.unmount();

  assert.strictEqual(
    portal.childNodes.length,
    0,
    "unmounting the owning component removed the teleported nodes from the target (no leak)"
  );
  portal.remove();
});

// --- 2. teleport inside a :for item that gets removed ------------------------

await test("teleportBlock inside a :for item: removing the item cleans up the teleported nodes", async () => {
  const portal = makePortal("portal-for-item");
  const c = createContext(null);
  const listHost = document.createElement("ul");
  const anchor = anchorAppend(listHost);
  const arr = deepBox(c, 0, ["a", "b", "c"]);

  forBlock(c, anchor, [0], () => Array.from(arr.v), {
    make: (d) => {
      const marker = document.createTextNode(d);
      teleportBlock(c, marker, () => "#portal-for-item", () => {
        const n = document.createElement("span");
        n.setAttribute("data-item", d);
        return n;
      });
      return marker;
    },
    keyOf: (d) => d,
  });

  assert.strictEqual(portal.childNodes.length, 3, "one teleported node per :for item");

  // Remove "b" from the list — its item scope is dropped, which must tear
  // down its teleport too (addDisposer path: the teleport call happened
  // while the item's scope was open).
  arr.v = ["a", "c"];
  await tick();

  assert.strictEqual(
    portal.childNodes.length,
    2,
    "removed item's teleported node was cleaned up, siblings untouched"
  );
  const remaining = portal.childNodes.map((n) => n.getAttribute("data-item"));
  assert.deepStrictEqual(remaining, ["a", "c"]);

  // Clearing the whole list must clean up everything.
  arr.v = [];
  await tick();
  assert.strictEqual(portal.childNodes.length, 0, "clearing the list cleans up every teleport");
  portal.remove();
});

// --- 3. teleport inside an :if branch that toggles off -----------------------

await test("teleportBlock inside an :if branch: toggling off cleans up the teleported nodes", async () => {
  const portal = makePortal("portal-if-branch");
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);
  const show = box(c, 0, true);

  ifBlock(c, anchor, [0], () => show.v, () => {
    const marker = document.createTextNode("");
    teleportBlock(c, marker, () => "#portal-if-branch", () => {
      const n = document.createElement("em");
      n.setAttribute("data-if", "1");
      return n;
    });
    return marker;
  });

  assert.strictEqual(portal.childNodes.length, 1, "teleported while the branch is shown");

  show.v = false;
  await tick();

  assert.strictEqual(
    portal.childNodes.length,
    0,
    "toggling the :if branch off removed the teleported nodes (no leak)"
  );

  // Toggling back on re-teleports fresh content.
  show.v = true;
  await tick();
  assert.strictEqual(portal.childNodes.length, 1, "re-showing the branch teleports again");
  portal.remove();
});

// --- destroy() is idempotent across both teardown paths ----------------------

await test("teleportBlock: destroy() is safe to call once even when both disposer paths could fire", () => {
  const portal = makePortal("portal-idempotent");
  const c = createContext(null);
  const host = document.createElement("div");
  const anchor = anchorAppend(host);

  const block = teleportBlock(c, anchor, () => "#portal-idempotent", () =>
    document.createElement("i")
  );
  assert.strictEqual(portal.childNodes.length, 1);
  block.destroy();
  assert.strictEqual(portal.childNodes.length, 0);
  // Calling destroy() again (e.g. a second teardown path firing) must not
  // throw or double-remove/double-drop.
  assert.doesNotThrow(() => block.destroy());
  portal.remove();
});

console.log("\ndom.teleport.test.mjs: " + passed + " passed.");
