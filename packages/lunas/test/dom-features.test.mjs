// dom-features.test.mjs — runtime helpers for the DOM feature batch:
// normClass/setClass, normStyle/setStyle, dynamicBlock (:is), teleportBlock,
// and the fragment() multi-root component factory.
// Run: node packages/lunas/test/dom-features.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
import { createContext, markVar, flush } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import {
  component,
  fragment,
  anchorAppend,
  normClass,
  setClass,
  normStyle,
  setStyle,
} from "../src/dom.mjs";
import { mountChild, dynamicBlock, teleportBlock, ifBlock } from "../src/blocks.mjs";

installDom();
const tick = () => new Promise((r) => setTimeout(r, 0));

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- normClass ---------------------------------------------------------------

await test("normClass: string / object / array / falsy", () => {
  assert.strictEqual(normClass("a b"), "a b");
  assert.strictEqual(normClass({ a: true, b: false, c: 1 }), "a c");
  assert.strictEqual(normClass(["a", { b: true }, ["c", { d: 0 }]]), "a b c");
  assert.strictEqual(normClass(null), "");
  assert.strictEqual(normClass(false), "");
  assert.strictEqual(normClass([null, "", false]), "");
});

await test("setClass: merges static class with dynamic value", () => {
  const el = document.createElement("div");
  setClass(el, "base", { active: true, hidden: false });
  assert.strictEqual(el.getAttribute("class"), "base active");
  setClass(el, "base", ["x", "y"]);
  assert.strictEqual(el.getAttribute("class"), "base x y");
  setClass(el, "", null);
  assert.strictEqual(el.getAttribute("class"), null, "empty -> attr removed");
  setClass(el, "onlystatic", "");
  assert.strictEqual(el.getAttribute("class"), "onlystatic");
});

// --- normStyle ---------------------------------------------------------------

await test("normStyle: string / object camel->kebab / array / custom prop", () => {
  assert.strictEqual(normStyle("color: red"), "color: red");
  assert.strictEqual(
    normStyle({ backgroundColor: "blue", fontSize: "12px" }),
    "background-color: blue; font-size: 12px;"
  );
  assert.strictEqual(normStyle({ "--my-var": "3px" }), "--my-var: 3px;");
  assert.strictEqual(
    normStyle([{ color: "red" }, { color: "blue" }]),
    "color: red; color: blue;"
  );
  assert.strictEqual(normStyle({ color: null, top: false }), "");
});

await test("setStyle: merges static style with dynamic value", () => {
  const el = document.createElement("div");
  setStyle(el, "margin: 0", { color: "red" });
  assert.strictEqual(el.getAttribute("style"), "margin: 0; color: red;");
  setStyle(el, "", "color: green");
  assert.strictEqual(el.getAttribute("style"), "color: green");
  setStyle(el, "", null);
  assert.strictEqual(el.getAttribute("style"), null);
});

// --- dynamicBlock (:is) ------------------------------------------------------

const makeChild = (label) =>
  component("span", {}, "", (c, props) => {
    c.root.setAttribute("data-label", label);
    // Reactive props arrive as getters (output-design.md §6); invoke to seed.
    const msg = props && props.msg ? props.msg() : "";
    c.root.setAttribute("data-msg", msg);
  });

await test("dynamicBlock: remounts when the factory changes", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const A = makeChild("A");
  const B = makeChild("B");
  const which = box(c, 0, A);

  const handle = dynamicBlock(c, anchor, [0], () => which.v, {});
  assert.strictEqual(handle.handle.root.getAttribute("data-label"), "A");
  // Exactly one child node + the anchor.
  assert.strictEqual(
    parent.childNodes.filter((n) => n.kind === "element").length,
    1
  );

  which.v = B;
  await tick();
  assert.strictEqual(handle.handle.root.getAttribute("data-label"), "B");
  assert.strictEqual(
    parent.childNodes.filter((n) => n.kind === "element").length,
    1,
    "old child unmounted, only new one present"
  );
});

await test("dynamicBlock: falsy factory renders nothing; props re-seed", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const A = makeChild("A");
  const which = box(c, 0, null);
  const handle = dynamicBlock(c, anchor, [0], () => which.v, { msg: () => "hi" });
  assert.strictEqual(handle.handle, null, "nothing mounted initially");

  which.v = A;
  await tick();
  assert.strictEqual(handle.handle.root.getAttribute("data-msg"), "hi");
});

// --- teleportBlock -----------------------------------------------------------

await test("teleportBlock: renders into a selector target, not inline", () => {
  const c = createContext(null);
  const inlineParent = document.createElement("div");
  const anchor = anchorAppend(inlineParent);

  const portal = document.createElement("div");
  portal.setAttribute("id", "portal");
  document.body.appendChild(portal);

  const block = teleportBlock(c, anchor, () => "#portal", () => {
    const n = document.createElement("p");
    n.setAttribute("data-x", "1");
    return n;
  });

  // Content is in the portal, not the inline parent.
  assert.strictEqual(portal.childNodes.length, 1);
  assert.strictEqual(portal.childNodes[0].getAttribute("data-x"), "1");
  assert.strictEqual(
    inlineParent.childNodes.filter((n) => n.kind === "element").length,
    0
  );

  block.destroy();
  assert.strictEqual(portal.childNodes.length, 0, "teardown removes nodes");
  portal.remove();
});

await test("teleportBlock: accepts an Element target directly", () => {
  const c = createContext(null);
  const anchor = anchorAppend(document.createElement("div"));
  const target = document.createElement("section");
  const block = teleportBlock(c, anchor, () => target, () =>
    document.createElement("span")
  );
  assert.strictEqual(target.childNodes.length, 1);
  block.destroy();
  assert.strictEqual(target.childNodes.length, 0);
});

// --- fragment() multi-root component -----------------------------------------

await test("fragment: returns a node group carrying the context", () => {
  const factory = fragment({}, "<p></p><span></span>", (c) => {
    c.root.childNodes[0].setAttribute("data-a", "1");
    c.root.childNodes[1].setAttribute("data-b", "2");
  });
  const frag = factory({});
  assert.ok(Array.isArray(frag), "fragment is a node array");
  assert.strictEqual(frag.length, 2);
  assert.strictEqual(frag[0].tag, "p");
  assert.strictEqual(frag[0].getAttribute("data-a"), "1");
  assert.strictEqual(frag[1].getAttribute("data-b"), "2");
  assert.ok(frag.__lunasCtx, "context exposed for parent prop driving");
  assert.strictEqual(frag[0].parentNode, null, "detached, ready to mount");
});

await test("fragment: mountChild inserts and unmounts the whole group", () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const factory = fragment({}, "<p></p><span></span>", () => {});
  const handle = mountChild(c, anchor, factory, {});
  assert.strictEqual(
    parent.childNodes.filter((n) => n.kind === "element").length,
    2,
    "both roots inserted"
  );
  handle.unmount();
  assert.strictEqual(
    parent.childNodes.filter((n) => n.kind === "element").length,
    0,
    "both roots removed"
  );
});

await test("fragment: reactive prop drives a multi-root child", async () => {
  // Child fragment reads a prop into both roots. Refs are captured during
  // setup (while nodes are still attached to the host) exactly like the
  // emitted code does via refs(c.root, paths).
  const child = fragment({}, "<p></p><span></span>", (c, props) => {
    const e0 = c.root.childNodes[0];
    const e1 = c.root.childNodes[1];
    const p = { v: (props && props.label && props.label()) || "" };
    const apply = (x) => {
      e0.setAttribute("data-l", x);
      e1.setAttribute("data-l", x);
    };
    (c._props || (c._props = {})).label = {
      get v() {
        return p.v;
      },
      set v(x) {
        p.v = x;
        apply(x);
      },
    };
    apply(p.v);
  });
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const handle = mountChild(c, anchor, child, { label: () => "init" });
  assert.strictEqual(handle.root[0].getAttribute("data-l"), "init");
  handle.setProp("label", "changed");
  assert.strictEqual(handle.root[0].getAttribute("data-l"), "changed");
  assert.strictEqual(handle.root[1].getAttribute("data-l"), "changed");
});

console.log("dom-features.test.mjs: all " + passed + " tests passed");
