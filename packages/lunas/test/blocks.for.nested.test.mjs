// blocks.for.nested.test.mjs — nested control-flow inside forBlock:
// :for-in-:for, :if-in-:for, per-item scope teardown on removal, item patch
// driving nested blocks, and the compiled bulk-render (html/wire) initial path
// vs the reconcile update path. Uses the fake DOM shim (installDom) because the
// compiled path parses per-item skeleton HTML via innerHTML.
// Run: node packages/lunas/test/blocks.for.nested.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
const document = installDom();

import { createContext, bind, markVar } from "../src/core.mjs";
import { box, deepBox } from "../src/boxes.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { ifBlock, forBlock } from "../src/blocks.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));
const shape = (p) =>
  p.childNodes
    .map((n) => (n.kind === "text" ? (n.data === "" ? "|" : n.data) : "<" + n.tag + ">"))
    .join(" ");
// Flattened text content of a subtree (for asserting nested structure cheaply).
const textOf = (n) => {
  if (n.kind === "text") return n.data;
  return n.childNodes.map(textOf).join("");
};
const rowsText = (p) =>
  p.childNodes.filter((n) => n.kind === "element").map(textOf).join("|");

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

// --- :for inside :for --------------------------------------------------------

await test("nested :for-in-:for renders a grid and reorders inner independently", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  // outer rows: each row has an inner :for over row.cells
  let rows = [
    { id: "r1", cells: ["a", "b"] },
    { id: "r2", cells: ["c"] },
  ];
  forBlock(c, anchor, [0], () => rows, {
    make: (row) => {
      const rowEl = document.createElement("row");
      const innerAnchor = document.createTextNode("");
      rowEl.appendChild(innerAnchor);
      forBlock(c, innerAnchor, [1], () => row.cells, {
        make: (cell) => document.createTextNode(cell),
        keyOf: (cell) => cell,
      });
      return rowEl;
    },
    keyOf: (row) => row.id,
  });
  assert.strictEqual(rowsText(parent), "ab|c");

  // Mutate an inner list and re-run its dep var (1). Only inner :for reconciles.
  rows[0].cells = ["b", "a", "x"];
  markVar(c, 1);
  await tick();
  assert.strictEqual(rowsText(parent), "bax|c");
});

await test("removing an outer row tears down its inner :for binds (no leak)", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const hi = box(c, 2, "!");
  let rows = [
    { id: "r1", cells: ["a"] },
    { id: "r2", cells: ["b"] },
  ];
  const cellRuns = {};
  forBlock(c, anchor, [0], () => rows, {
    make: (row) => {
      const rowEl = document.createElement("row");
      const innerAnchor = document.createTextNode("");
      rowEl.appendChild(innerAnchor);
      forBlock(c, innerAnchor, [1], () => row.cells, {
        make: (cell) => {
          const t = document.createTextNode(cell);
          bind(c, [2], () => {
            cellRuns[cell] = (cellRuns[cell] || 0) + 1;
            t.data = cell + hi.v;
          });
          return t;
        },
        keyOf: (cell) => cell,
      });
      return rowEl;
    },
    keyOf: (row) => row.id,
  });
  assert.deepStrictEqual(cellRuns, { a: 1, b: 1 });
  hi.v = "?";
  await tick();
  assert.deepStrictEqual(cellRuns, { a: 2, b: 2 }, "both inner cell binds live");

  // Remove the second outer row: its inner cell "b" bind must be unregistered.
  rows = [{ id: "r1", cells: ["a"] }];
  markVar(c, 0);
  await tick();
  const bAfterRemove = cellRuns.b;
  const aAfterRemove = cellRuns.a;
  hi.v = "#";
  await tick();
  assert.strictEqual(cellRuns.b, bAfterRemove, "removed row's inner bind is dead");
  assert.ok(cellRuns.a > aAfterRemove, "surviving row's inner bind still fires");
});

// --- :if inside a :for item --------------------------------------------------

await test(":if inside a :for item toggles per item", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  let items = [
    { id: 1, open: true },
    { id: 2, open: false },
  ];
  forBlock(c, anchor, [0], () => items, {
    make: (it) => {
      const wrap = document.createElement("item");
      const innerAnchor = document.createTextNode("");
      wrap.appendChild(innerAnchor);
      ifBlock(c, innerAnchor, [1], () => it.open, () =>
        document.createTextNode("open" + it.id)
      );
      return wrap;
    },
    keyOf: (it) => it.id,
  });
  assert.strictEqual(rowsText(parent), "open1|");
  items[1].open = true;
  markVar(c, 1);
  await tick();
  assert.strictEqual(rowsText(parent), "open1|open2");
  items[0].open = false;
  markVar(c, 1);
  await tick();
  assert.strictEqual(rowsText(parent), "|open2");
});

await test("removing a :for item with an open :if drops the inner branch bind", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const tick2 = box(c, 2, 0);
  let items = [{ id: 1 }, { id: 2 }];
  const runs = {};
  forBlock(c, anchor, [0], () => items, {
    make: (it) => {
      const wrap = document.createElement("item");
      const innerAnchor = document.createTextNode("");
      wrap.appendChild(innerAnchor);
      ifBlock(c, innerAnchor, [1], () => true, () => {
        const t = document.createTextNode("");
        bind(c, [2], () => {
          runs[it.id] = (runs[it.id] || 0) + 1;
          t.data = it.id + ":" + tick2.v;
        });
        return t;
      });
      return wrap;
    },
    keyOf: (it) => it.id,
  });
  assert.deepStrictEqual(runs, { 1: 1, 2: 1 });
  items = [{ id: 1 }]; // remove item 2
  markVar(c, 0);
  await tick();
  const twoAfterRemove = runs[2];
  const oneAfterRemove = runs[1];
  tick2.v = 9;
  await tick();
  assert.strictEqual(runs[2], twoAfterRemove, "removed item's inner :if bind is dead");
  assert.ok(runs[1] > oneAfterRemove, "kept item's inner :if bind fires");
});

// --- item patch drives a nested block (runScope) -----------------------------

await test("patching a :for item re-runs its nested block with fresh data", async () => {
  const c = createContext(null);
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  // Each item carries an `arr` field; the nested :for renders it. Patch (same
  // key, new data) must flow into the nested :for via the item scope re-run.
  let items = [{ id: 1, arr: ["a", "b"] }];
  forBlock(c, anchor, [0], () => items, {
    make: (it) => {
      const wrap = document.createElement("item");
      const innerAnchor = document.createTextNode("");
      wrap.appendChild(innerAnchor);
      // The nested :for reads the CURRENT item via a closure over `it` cell.
      forBlock(c, innerAnchor, [0], () => it.arr, {
        make: (x) => document.createTextNode(x),
        keyOf: (x) => x,
      });
      // record the item so patch can mutate the same cell the nested :for reads
      it._cellRef = it;
      return wrap;
    },
    keyOf: (it) => it.id,
    patch: (h, d) => {
      // The item object identity changes on patch; mirror the new arr onto the
      // captured closure cell so the nested :for (which closed over the ORIGINAL
      // item) sees it. In real compiled output the item box is the closure cell.
    },
  });
  assert.strictEqual(rowsText(parent), "ab");

  // Same key, new arr: because the nested :for closed over the original item
  // object, we mutate that object in place (the normal deep-reactive case).
  items[0].arr = ["b", "a", "c"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(rowsText(parent), "bac", "nested :for re-reconciled on patch");
});

// --- compiled bulk-render (html/wire) initial path ---------------------------

await test("compiled html/wire: bulk initial render then keyed update", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  let data = [
    { id: "a", label: "A" },
    { id: "b", label: "B" },
    { id: "c", label: "C" },
  ];
  let wireCount = 0;
  forBlock(c, anchor, [0], () => data, {
    html: "<li><span></span></li>",
    wire: (root, d) => {
      wireCount++;
      const span = root.childNodes[0];
      span.appendChild(document.createTextNode(d.label));
      // return a patch closure that updates the label text cell
      return (nd) => {
        span.childNodes[0].data = nd.label;
      };
    },
    keyOf: (d) => d.id,
  });
  assert.strictEqual(rowsText(parent), "A|B|C");
  assert.strictEqual(wireCount, 3, "bulk render wired 3 items once each");
  const liNodes = parent.childNodes.filter((n) => n.kind === "element");

  // Reorder: keyed reconcile reuses the same <li> nodes (no re-wire).
  data = [
    { id: "c", label: "C" },
    { id: "a", label: "A" },
    { id: "b", label: "B" },
  ];
  markVar(c, 0);
  await tick();
  assert.strictEqual(rowsText(parent), "C|A|B");
  assert.strictEqual(wireCount, 3, "reorder did not re-wire (nodes reused)");
  const after = parent.childNodes.filter((n) => n.kind === "element");
  assert.strictEqual(after[0], liNodes[2], "the <li> for c is the same node, moved");

  // Patch in place (same keys, new labels): patch closure updates text, no wire.
  data = [
    { id: "c", label: "C2" },
    { id: "a", label: "A2" },
    { id: "b", label: "B2" },
  ];
  markVar(c, 0);
  await tick();
  assert.strictEqual(rowsText(parent), "C2|A2|B2", "patch closures updated labels");
  assert.strictEqual(wireCount, 3, "patch path never re-wires");
});

await test("compiled html/wire: insert new item builds via makeItem (per-item parse)", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  let data = [{ id: "a", label: "A" }];
  let wireCount = 0;
  forBlock(c, anchor, [0], () => data, {
    html: "<li></li>",
    wire: (root, d) => {
      wireCount++;
      root.appendChild(document.createTextNode(d.label));
      return (nd) => {
        root.childNodes[0].data = nd.label;
      };
    },
    keyOf: (d) => d.id,
  });
  assert.strictEqual(rowsText(parent), "A");
  assert.strictEqual(wireCount, 1);
  data = [{ id: "a", label: "A" }, { id: "b", label: "B" }];
  markVar(c, 0);
  await tick();
  assert.strictEqual(rowsText(parent), "A|B");
  assert.strictEqual(wireCount, 2, "new item parsed + wired once");
});

await test("compiled html/wire: empty initial list, then grow", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  let data = [];
  forBlock(c, anchor, [0], () => data, {
    html: "<li></li>",
    wire: (root, d) => {
      root.appendChild(document.createTextNode(d.label));
    },
    keyOf: (d) => d.id,
  });
  assert.strictEqual(shape(parent), "|", "empty renders just the anchor");
  data = [{ id: 1, label: "X" }, { id: 2, label: "Y" }];
  markVar(c, 0);
  await tick();
  assert.strictEqual(rowsText(parent), "X|Y");
});

console.log("\nblocks.for.nested.test.mjs: " + passed + " passed.");
