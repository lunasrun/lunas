// slots.runtime.test.mjs — additional runtime coverage for slotBlock /
// slotContent (src/blocks.mjs), complementing slots.test.mjs. Focus: scoped
// props snapshot semantics, multiple onCleanup registrations, teardown when the
// slot content lives inside a :for item scope, and named-slot routing where one
// slot is filled and another falls back.
// Run: node packages/lunas/test/slots.runtime.test.mjs

import assert from "node:assert";
import {
  createContext,
  bind,
  beginScope,
  endScope,
  dropScope,
} from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { slotBlock, slotContent } from "../src/blocks.mjs";
import { onDestroy, runDestroy } from "../src/lifecycle.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

// --- minimal fake DOM --------------------------------------------------------
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
    const at = ref == null ? this.childNodes.length : this.childNodes.indexOf(ref);
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
    const s = this.parentNode.childNodes;
    return s[s.indexOf(this) + 1] || null;
  }
}
const doc = {
  createTextNode: (d) => new FakeNode(doc, "text", d),
  createElement: (t) => {
    const n = new FakeNode(doc, "element", "");
    n.tag = t;
    return n;
  },
};
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

// --- default slot with parent content beats fallback -------------------------

await test("default slot: parent content wins; fallback is not built", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  let fallbackBuilt = 0;
  slotBlock(
    child,
    anchor,
    () => doc.createTextNode("parent"),
    () => {
      fallbackBuilt++;
      return doc.createTextNode("fb");
    }
  );
  assert.strictEqual(shape(host), "parent |");
  assert.strictEqual(fallbackBuilt, 0, "fallback factory never invoked");
});

// --- named routing: one filled, one falls back -------------------------------

await test("named slots: filled slot renders content, unfilled shows fallback", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const aHead = anchorAppend(host);
  const aBody = anchorAppend(host);
  // header is provided by the parent; body is not, so its fallback shows.
  slotBlock(child, aHead, () => doc.createTextNode("H!"), () =>
    doc.createTextNode("noheader")
  );
  slotBlock(child, aBody, undefined, () => doc.createTextNode("defaultbody"));
  assert.strictEqual(shape(host), "H! | defaultbody |");
});

// --- scoped props snapshot: slotPropsOf is read exactly once -----------------

await test("scoped slot props are snapshotted once at build time", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  let propsReads = 0;
  const factory = (sp) => doc.createTextNode("item=" + sp.item);
  slotBlock(child, anchor, factory, null, () => {
    propsReads++;
    return { item: "apple" };
  });
  assert.strictEqual(shape(host), "item=apple |");
  assert.strictEqual(propsReads, 1, "slotPropsOf invoked exactly once");
});

await test("scoped slot: the same props object is passed to factory and fallback path", () => {
  // When a factory is present, fallback is ignored; verify the props object the
  // factory saw is the object returned by slotPropsOf (identity, not a copy).
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const propsObj = { item: "x", idx: 3 };
  let seen = null;
  slotBlock(child, anchor, (sp) => {
    seen = sp;
    return doc.createTextNode(String(sp.idx));
  }, null, () => propsObj);
  assert.strictEqual(seen, propsObj, "factory got the exact props object");
  assert.strictEqual(shape(host), "3 |");
});

// --- multiple onCleanup registrations all fire on child unmount --------------

await test("multiple onCleanup callbacks all run on child unmount", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const fired = [];
  const factory = (sp, onCleanup) => {
    onCleanup(() => fired.push("a"));
    onCleanup(() => fired.push("b"));
    onCleanup("not a function"); // ignored defensively
    return doc.createTextNode("c");
  };
  slotBlock(child, anchor, factory, null);
  assert.deepStrictEqual(fired, [], "no cleanup before unmount");
  runDestroy(child);
  assert.deepStrictEqual(fired, ["a", "b"], "both function cleanups fired");
});

// --- slotContent: parent-owned scope drops on child unmount ------------------

await test("slotContent binds live on the parent and drop on child unmount", async () => {
  const parent = createContext(null);
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const n = box(parent, 0, 1);
  let runs = 0;

  const factory = (sp, onCleanup) =>
    slotContent(
      parent,
      () => {
        const t = doc.createTextNode("");
        bind(parent, [0], () => {
          runs++;
          t.data = "n=" + n.v;
        });
        return t;
      },
      sp,
      onCleanup
    );

  slotBlock(child, anchor, factory, null);
  assert.strictEqual(shape(host), "n=1 |");
  assert.strictEqual(runs, 1);
  n.v = 2;
  await tick();
  assert.strictEqual(shape(host), "n=2 |", "parent drives the slot content");
  assert.strictEqual(runs, 2);

  runDestroy(child); // parent-owned scope dropped via onCleanup
  n.v = 3;
  await tick();
  assert.strictEqual(runs, 2, "no bind ran after child unmount");
  // Parent adjacency for var 0 is now empty (the only dependent was the slot).
  assert.strictEqual((parent.deps[0] || []).length, 0);
});

// --- slotContent homed in a :for item scope tears down with the item ---------

await test("slotContent homed in an item scope drops when that scope is dropped", async () => {
  const parent = createContext(null);
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const n = box(parent, 0, 1);
  let runs = 0;

  // Simulate the parent mounting the child inside a :for item scope: the scope
  // is open when slotContent runs, so its content scope homes under the item.
  const itemScope = beginScope(parent);
  const factory = (sp, onCleanup) =>
    slotContent(
      parent,
      () => {
        const t = doc.createTextNode("");
        bind(parent, [0], () => {
          runs++;
          t.data = "v=" + n.v;
        });
        return t;
      },
      sp,
      onCleanup
    );
  slotBlock(child, anchor, factory, null);
  endScope(parent);

  assert.strictEqual(runs, 1);
  // Dropping the item scope must tear down the slot content's parent-owned bind,
  // even though the bind lives on the parent context.
  dropScope(parent, itemScope);
  n.v = 5;
  await tick();
  assert.strictEqual(runs, 1, "slot content bind dropped with the item scope");
});

// --- fallback wired in child scope drops on child destroy --------------------

await test("fallback content (child scope) drops on child destroy", async () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const flag = box(child, 0, "a");
  let runs = 0;
  slotBlock(child, anchor, undefined, () => {
    const t = doc.createTextNode("");
    bind(child, [0], () => {
      runs++;
      t.data = flag.v;
    });
    return t;
  });
  assert.strictEqual(shape(host), "a |");
  assert.strictEqual(runs, 1);
  runDestroy(child);
  flag.v = "b";
  await tick();
  assert.strictEqual(runs, 1, "fallback bind dropped after destroy");
});

// --- empty parent content (factory returns null) -----------------------------

await test("factory returning null renders nothing (no fallback either)", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  // A function factory is present (so fallback is skipped) but yields no nodes.
  slotBlock(child, anchor, () => null, () => doc.createTextNode("fb"));
  assert.strictEqual(shape(host), "|", "no content, and fallback suppressed by presence of factory");
});

// --- onDestroy ordering: slot cleanup runs alongside other child hooks -------

await test("slot cleanup and a separate child onDestroy both fire once", () => {
  const child = createContext(null);
  const host = doc.createElement("div");
  const anchor = anchorAppend(host);
  const order = [];
  onDestroy(child, () => order.push("child-hook"));
  slotBlock(child, anchor, (sp, onCleanup) => {
    onCleanup(() => order.push("slot-cleanup"));
    return doc.createTextNode("x");
  }, null);
  runDestroy(child);
  runDestroy(child); // idempotent
  assert.ok(order.includes("child-hook"));
  assert.ok(order.includes("slot-cleanup"));
  assert.strictEqual(order.length, 2, "each hook fired exactly once");
});

console.log("\nslots.runtime.test.mjs: " + passed + " passed.");
