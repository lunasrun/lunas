// slots.test.mjs — slotBlock / slotContent (c-slots) against a minimal fake DOM.
// Run: node packages/lunas/test/slots.test.mjs
//
// Covers: default slot fills from parent content, fallback when unfilled, named
// routing, scoped props flowing up, parent-state reactivity into slot content,
// and teardown (no leaks / no late writes) on child unmount. Plus a never-panic
// fuzz over arbitrary slot/template nesting shapes.

import assert from "node:assert";
import {
  createContext,
  bind,
  markVar,
} from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { slotBlock, slotContent, mountChild } from "../src/blocks.mjs";
import { onDestroy, runDestroy } from "../src/lifecycle.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

// --- minimal fake DOM (same narrow surface the runtime uses) -----------------
class FakeNode {
  constructor(doc, kind, data) {
    this.ownerDocument = doc;
    this.kind = kind;
    this.data = data || "";
    this.childNodes = [];
    this.parentNode = null;
  }
  insertBefore(n, ref) {
    if (n.parentNode) n.parentNode._drop(n);
    const at =
      ref == null ? this.childNodes.length : this.childNodes.indexOf(ref);
    this.childNodes.splice(at, 0, n);
    n.parentNode = this;
    return n;
  }
  appendChild(n) {
    return this.insertBefore(n, null);
  }
  _drop(n) {
    const i = this.childNodes.indexOf(n);
    if (i >= 0) this.childNodes.splice(i, 1);
    n.parentNode = null;
  }
  remove() {
    if (this.parentNode) this.parentNode._drop(this);
  }
  get nextSibling() {
    if (!this.parentNode) return null;
    const sib = this.parentNode.childNodes;
    return sib[sib.indexOf(this) + 1] || null;
  }
}
const fakeDoc = {
  createTextNode: (d) => new FakeNode(fakeDoc, "text", d),
  createElement: (tag) => {
    const n = new FakeNode(fakeDoc, "element", "");
    n.tag = tag;
    return n;
  },
};
const shape = (parent) =>
  parent.childNodes
    .map((n) =>
      n.kind === "text" ? (n.data === "" ? "|" : n.data) : "<" + n.tag + ">"
    )
    .join(" ");

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- default slot: parent content wins over fallback -------------------------

await test("slotBlock inserts parent-provided content at the anchor", () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const factory = () => fakeDoc.createTextNode("from parent");
  const fallback = () => fakeDoc.createTextNode("fallback");
  slotBlock(child, anchor, factory, fallback);
  assert.strictEqual(shape(host), "from parent |");
});

await test("slotBlock shows fallback when no parent content given", () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const fallback = () => fakeDoc.createTextNode("fallback");
  slotBlock(child, anchor, undefined, fallback);
  assert.strictEqual(shape(host), "fallback |");
});

await test("slotBlock with neither content nor fallback renders nothing", () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  slotBlock(child, anchor, null, null);
  assert.strictEqual(shape(host), "|");
});

// --- named routing: two slots, distinct factories ----------------------------

await test("named slots route to their own factories", () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const aHead = anchorAppend(host);
  const aBody = anchorAppend(host);
  const slots = {
    header: () => fakeDoc.createTextNode("H"),
    body: () => fakeDoc.createTextNode("B"),
  };
  slotBlock(child, aHead, slots.header, null);
  slotBlock(child, aBody, slots.body, null);
  assert.strictEqual(shape(host), "H | B |");
});

// --- parent-state reactivity drives slot content in place --------------------

await test("parent state change updates parent-provided slot content in place", async () => {
  const parent = createContext(null);
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const count = box(parent, 0, 1);

  // Parent factory wires a text node against the PARENT context.
  const factory = (sp, onCleanup) =>
    slotContent(
      parent,
      () => {
        const t = fakeDoc.createTextNode("");
        bind(parent, [0], () => {
          t.data = "n=" + count.v;
        });
        return t;
      },
      sp,
      onCleanup
    );

  slotBlock(child, anchor, factory, null);
  assert.strictEqual(shape(host), "n=1 |");
  count.v = 5;
  await tick();
  assert.strictEqual(shape(host), "n=5 |", "parent state drove child-slot text");
});

// --- scoped slot: child-provided props flow up to parent content -------------

await test("scoped slot props flow from child up into parent content", () => {
  const parent = createContext(null);
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);

  // Parent content reads slotProps.item (provided by the child's <slot :item>).
  const factory = (sp, onCleanup) =>
    slotContent(
      parent,
      (props) => fakeDoc.createTextNode("item=" + props.item),
      sp,
      onCleanup
    );

  // Child exposes a scoped prop via slotPropsOf.
  slotBlock(child, anchor, factory, null, () => ({ item: "apple" }));
  assert.strictEqual(shape(host), "item=apple |");
});

// --- teardown: parent-owned binds unregister on child unmount ----------------

await test("child unmount tears down parent-owned slot binds (no late writes)", async () => {
  const parent = createContext(null);
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const count = box(parent, 0, 1);
  let writes = 0;

  const factory = (sp, onCleanup) =>
    slotContent(
      parent,
      () => {
        const t = fakeDoc.createTextNode("");
        bind(parent, [0], () => {
          writes++;
          t.data = "n=" + count.v;
        });
        return t;
      },
      sp,
      onCleanup
    );

  slotBlock(child, anchor, factory, null);
  assert.strictEqual(writes, 1, "initial bind ran once");

  // Unmount the child: its onDestroy runs the slot cleanup, dropping the bind.
  runDestroy(child);

  count.v = 9;
  await tick();
  assert.strictEqual(writes, 1, "no bind ran after teardown");
  // Parent context has no live dependents left for var 0.
  assert.strictEqual(
    (parent.deps[0] || []).length,
    0,
    "parent adjacency for var 0 is empty after teardown"
  );
});

await test("fallback content wired in child scope drops on child destroy", async () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const flag = box(child, 0, "a");
  let writes = 0;

  slotBlock(child, anchor, undefined, () => {
    const t = fakeDoc.createTextNode("");
    bind(child, [0], () => {
      writes++;
      t.data = flag.v;
    });
    return t;
  });
  assert.strictEqual(shape(host), "a |");
  assert.strictEqual(writes, 1);

  runDestroy(child);
  flag.v = "b";
  await tick();
  assert.strictEqual(writes, 1, "fallback bind dropped after child destroy");
});

// --- multi-root slot content travels as a group ------------------------------

await test("slotBlock inserts a multi-root node group before the anchor", () => {
  const child = createContext(null);
  const host = fakeDoc.createElement("div");
  const anchor = anchorAppend(host);
  const factory = () => [
    fakeDoc.createElement("a"),
    fakeDoc.createTextNode("mid"),
    fakeDoc.createElement("b"),
  ];
  slotBlock(child, anchor, factory, null);
  assert.strictEqual(shape(host), "<a> mid <b> |");
});

// --- never-panic fuzz: arbitrary factory/fallback/scoped combinations --------

await test("fuzz: slotBlock never throws across arbitrary shapes", () => {
  const factories = [
    undefined,
    null,
    () => null,
    () => fakeDoc.createTextNode("x"),
    () => [fakeDoc.createElement("i")],
    (sp) => fakeDoc.createTextNode(String(sp && sp.k)),
    (sp, oc) => {
      if (oc) oc(() => {});
      return fakeDoc.createTextNode("z");
    },
  ];
  const scopedGetters = [undefined, () => undefined, () => ({ k: 7 })];
  let ran = 0;
  for (const f of factories) {
    for (const fb of factories) {
      for (const sg of scopedGetters) {
        const child = createContext(null);
        const host = fakeDoc.createElement("div");
        const anchor = anchorAppend(host);
        assert.doesNotThrow(() => {
          slotBlock(child, anchor, f, fb, sg);
          runDestroy(child); // teardown must also never throw
        });
        ran++;
      }
    }
  }
  assert.ok(ran > 0);
});

console.log("\nslots.test.mjs: " + passed + " passed.");
