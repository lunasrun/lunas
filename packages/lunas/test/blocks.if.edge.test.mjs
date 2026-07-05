// blocks.if.edge.test.mjs — edge-case coverage for ifBlock / ifChain in
// src/blocks.mjs: elseif cascades, no-match (-1), rapid toggles, nested if,
// scope teardown across branch switches (no leaked binds), multi-root branches.
// Run: node packages/lunas/test/blocks.if.edge.test.mjs

import assert from "node:assert";
import { createContext, bind, beginScope, endScope, dropScope } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { ifBlock, ifChain } from "../src/blocks.mjs";

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
    if (ref != null && ref.parentNode !== this)
      throw new Error("insertBefore: refNode is not a child");
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

// Count total live bind records registered on a context (across all vars).
const liveBinds = (c) => {
  const seen = new Set();
  for (const list of c.deps) if (list) for (const s of list) if (s.alive) seen.add(s);
  return seen.size;
};

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- ifChain: full elseif cascade with else ----------------------------------

await test("ifChain elseif cascade routes to the correct branch; else at end", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const n = box(c, 0, 0);
  // if n<0 -> neg ; elseif n===0 -> zero ; elseif n<10 -> small ; else big
  ifChain(
    c,
    anchor,
    [0],
    () => (n.v < 0 ? 0 : n.v === 0 ? 1 : n.v < 10 ? 2 : 3),
    [
      () => doc.createElement("neg"),
      () => doc.createElement("zero"),
      () => doc.createElement("small"),
      () => doc.createElement("big"),
    ]
  );
  assert.strictEqual(shape(parent), "<zero> |");
  n.v = -3;
  await tick();
  assert.strictEqual(shape(parent), "<neg> |");
  n.v = 5;
  await tick();
  assert.strictEqual(shape(parent), "<small> |");
  n.v = 42;
  await tick();
  assert.strictEqual(shape(parent), "<big> |", "else branch");
});

await test("ifChain no-match (-1) with no else renders nothing", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const n = box(c, 0, 5);
  ifChain(c, anchor, [0], () => (n.v > 0 ? 0 : n.v < 0 ? 1 : -1), [
    () => doc.createElement("pos"),
    () => doc.createElement("neg"),
  ]);
  assert.strictEqual(shape(parent), "<pos> |");
  n.v = 0;
  await tick();
  assert.strictEqual(shape(parent), "|", "no branch for -1");
  n.v = -2;
  await tick();
  assert.strictEqual(shape(parent), "<neg> |");
  n.v = 0;
  await tick();
  assert.strictEqual(shape(parent), "|");
});

await test("ifChain -1 -> branch -> -1 leaves no leaked binds", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const on = box(c, 0, false);
  const label = box(c, 1, "x");
  let runs = 0;
  ifChain(c, anchor, [0], () => (on.v ? 0 : -1), [
    () => {
      const el = doc.createElement("a");
      bind(c, [1], () => {
        runs++;
        el.data = label.v;
      });
      return el;
    },
  ]);
  assert.strictEqual(runs, 0, "hidden at start");
  const baseBinds = liveBinds(c);
  on.v = true;
  await tick();
  assert.strictEqual(runs, 1);
  assert.strictEqual(liveBinds(c), baseBinds + 1, "branch bind registered");
  on.v = false;
  await tick();
  assert.strictEqual(liveBinds(c), baseBinds, "branch bind unregistered on hide");
  label.v = "y";
  await tick();
  assert.strictEqual(runs, 1, "no leaked bind fired");
});

// --- rapid toggles (multiple sync writes coalesce into one flush) ------------

await test("ifBlock rapid synchronous toggles settle to the last state", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const show = box(c, 0, false);
  let makes = 0;
  ifBlock(c, anchor, [0], () => show.v, () => {
    makes++;
    return doc.createElement("s");
  });
  // Many writes in one tick: only the final value matters (run() is idempotent
  // wrt the current condition; make happens at most once per shown transition).
  show.v = true;
  show.v = false;
  show.v = true;
  await tick();
  assert.strictEqual(shape(parent), "<s> |", "settled to shown");
  assert.strictEqual(makes, 1, "coalesced: built once");
  show.v = false;
  show.v = true;
  show.v = false;
  await tick();
  assert.strictEqual(shape(parent), "|", "settled to hidden");
  assert.strictEqual(makes, 1, "no rebuild since it ended hidden");
});

await test("ifBlock toggle same value within a tick is a no-op (no rebuild)", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const show = box(c, 0, true);
  let makes = 0;
  ifBlock(c, anchor, [0], () => show.v, () => {
    makes++;
    return doc.createElement("s");
  });
  assert.strictEqual(makes, 1);
  const node = parent.childNodes[0];
  show.v = true; // re-write same value
  await tick();
  assert.strictEqual(parent.childNodes[0], node, "node not rebuilt");
  assert.strictEqual(makes, 1);
});

// --- nested if-in-if ---------------------------------------------------------

await test("nested ifBlock: inner branch tears down when outer hides", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const outer = box(c, 0, false);
  const inner = box(c, 1, false);
  const label = box(c, 2, "x");
  let innerRuns = 0;

  ifBlock(c, anchor, [0], () => outer.v, () => {
    const wrap = doc.createElement("outer");
    const innerAnchor = doc.createTextNode("");
    wrap.appendChild(innerAnchor);
    // nested :if built inside the outer branch's scope
    ifBlock(c, innerAnchor, [1], () => inner.v, () => {
      const el = doc.createElement("inner");
      bind(c, [2], () => {
        innerRuns++;
        el.data = label.v;
      });
      return el;
    });
    return wrap;
  });

  assert.strictEqual(shape(parent), "|", "all hidden");
  outer.v = true;
  await tick();
  assert.strictEqual(innerRuns, 0, "inner still hidden");
  inner.v = true;
  await tick();
  assert.strictEqual(innerRuns, 1, "inner shown, bind ran");
  label.v = "y";
  await tick();
  assert.strictEqual(innerRuns, 2, "inner bind live");

  // Hiding the OUTER must recursively drop the inner branch's binds.
  outer.v = false;
  await tick();
  const before = innerRuns;
  label.v = "z";
  await tick();
  assert.strictEqual(innerRuns, before, "inner bind dead after outer hide");
  assert.strictEqual(shape(parent), "|");
});

// --- multi-root branch inserts/removes as a group ----------------------------

await test("ifChain multi-root branches swap as whole groups", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  parent.appendChild(doc.createTextNode("H"));
  const anchor = anchorAppend(parent);
  parent.appendChild(doc.createTextNode("T"));
  const n = box(c, 0, 0);
  ifChain(c, anchor, [0], () => (n.v === 0 ? 0 : 1), [
    () => [doc.createElement("a"), doc.createElement("b")],
    () => [doc.createElement("x"), doc.createElement("y"), doc.createElement("z")],
  ]);
  assert.strictEqual(shape(parent), "H <a> <b> | T");
  n.v = 1;
  await tick();
  assert.strictEqual(shape(parent), "H <x> <y> <z> | T", "whole group swapped");
});

// --- branch switch tears the OLD scope, keeps no leaks -----------------------

await test("ifChain branch switch: old branch binds fully unregistered", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const which = box(c, 0, 0);
  const l0 = box(c, 1, "a");
  const l1 = box(c, 2, "b");
  let r0 = 0;
  let r1 = 0;
  ifChain(c, anchor, [0], () => which.v, [
    () => {
      const el = doc.createElement("zero");
      bind(c, [1], () => {
        r0++;
        el.data = l0.v;
      });
      return el;
    },
    () => {
      const el = doc.createElement("one");
      bind(c, [2], () => {
        r1++;
        el.data = l1.v;
      });
      return el;
    },
  ]);
  const base = liveBinds(c);
  assert.strictEqual(r0, 1);
  which.v = 1;
  await tick();
  assert.strictEqual(r1, 1, "branch 1 built");
  // branch 0's bind is gone; branch 1's is present => count unchanged.
  assert.strictEqual(liveBinds(c), base, "one branch bind at a time");
  l0.v = "A"; // writes to the now-dead branch's dep
  await tick();
  assert.strictEqual(r0, 1, "dead branch 0 bind never fires again");
  l1.v = "B";
  await tick();
  assert.strictEqual(r1, 2, "live branch 1 bind fires");
});

// --- destroy() while shown, then no further work -----------------------------

await test("ifBlock destroy() while shown removes content and kills binds", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const show = box(c, 0, true);
  const label = box(c, 1, "x");
  let runs = 0;
  const blk = ifBlock(c, anchor, [0], () => show.v, () => {
    const el = doc.createElement("s");
    bind(c, [1], () => {
      runs++;
      el.data = label.v;
    });
    return el;
  });
  assert.strictEqual(runs, 1);
  blk.destroy();
  assert.strictEqual(shape(parent), "|", "content removed on destroy");
  assert.strictEqual(liveBinds(c), 0, "all binds unregistered");
  show.v = false;
  show.v = true;
  label.v = "y";
  await tick();
  assert.strictEqual(runs, 1, "nothing runs after destroy");
});

// --- update() drives the block synchronously (for :for item patching) --------

await test("ifBlock.update() re-evaluates condition synchronously", () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  let flag = false;
  const blk = ifBlock(c, anchor, [0], () => flag, () => doc.createElement("s"));
  assert.strictEqual(shape(parent), "|");
  flag = true;
  blk.update(); // synchronous, no flush
  assert.strictEqual(shape(parent), "<s> |");
  flag = false;
  blk.update();
  assert.strictEqual(shape(parent), "|");
});

// --- scope homing across dropScope (item teardown recurses into :if) ---------

await test("ifChain built inside a home scope tears down with that scope", async () => {
  const c = createContext(null);
  const parent = doc.createElement("div");
  const anchor = anchorAppend(parent);
  const which = box(c, 0, 0);
  const label = box(c, 1, "x");
  let runs = 0;
  const home = beginScope(c);
  ifChain(c, anchor, [0], () => which.v, [
    () => {
      const el = doc.createElement("z");
      bind(c, [1], () => {
        runs++;
        el.data = label.v;
      });
      return el;
    },
  ]);
  endScope(c);
  assert.strictEqual(runs, 1);
  dropScope(c, home); // simulate the enclosing :for item being removed
  which.v = -1; // even switching now shouldn't resurrect anything
  label.v = "y";
  await tick();
  assert.strictEqual(runs, 1, "branch bind dropped with home scope");
});

console.log("\nblocks.if.edge.test.mjs: " + passed + " passed.");
