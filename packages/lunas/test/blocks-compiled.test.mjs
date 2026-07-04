// blocks-compiled.test.mjs — the compiled control-flow surface the code
// generator targets: forBlock's html/wire mode (bulk innerHTML initial render,
// per-item wiring, patch-through-runScope) and fromHTML branch building.
// Uses the dom-shim (innerHTML-capable), the same fake DOM the compiled-output
// exec tests run against.
// Run: node packages/lunas/test/blocks-compiled.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind, markVar } from "../src/core.mjs";
import { box, deepBox } from "../src/boxes.mjs";
import { fromHTML, anchorAppend } from "../src/dom.mjs";
import { ifBlock, forBlock } from "../src/blocks.mjs";

installDom();
const tick = () => new Promise((r) => setTimeout(r, 0));

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

// --- fromHTML ------------------------------------------------------------------

await test("fromHTML parses a branch skeleton into a detached scratch element", () => {
  const near = document.createTextNode("");
  const r = fromHTML('<p class="a">hi</p><span></span>', near);
  assert.strictEqual(r.childNodes.length, 2);
  assert.strictEqual(r.childNodes[0].tag, "p");
  assert.strictEqual(r.childNodes[0].getAttribute("class"), "a");
  assert.strictEqual(r.childNodes[1].tag, "span");
  assert.strictEqual(r.parentNode, null, "detached");
});

// --- forBlock compiled mode ------------------------------------------------------

await test("forBlock html/wire: bulk initial render + per-item wiring", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  const list = deepBox(c, 0, [
    { id: 1, txt: "one" },
    { id: 2, txt: "two" },
  ]);
  let wires = 0;
  forBlock(c, anchor, [0], () => Array.from(list.v), {
    html: "<li></li>",
    wire: (root, d0) => {
      wires++;
      let d = d0;
      const t = document.createTextNode("");
      root.appendChild(t);
      bind(c, [], () => {
        t.data = d.txt;
      });
      return (nd) => {
        d = nd;
      };
    },
    keyOf: (d) => d.id,
  });
  assert.strictEqual(wires, 2, "each item wired once");
  assert.strictEqual(parent.childNodes.length, 3, "2 items + anchor");
  assert.strictEqual(parent.childNodes[0].outerHTML, "<li>one</li>");
  assert.strictEqual(parent.childNodes[1].outerHTML, "<li>two</li>");
});

await test("forBlock html/wire: push appends, reorder moves same nodes, patch refreshes data", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  const list = deepBox(c, 0, [
    { id: 1, txt: "a" },
    { id: 2, txt: "b" },
  ]);
  forBlock(c, anchor, [0], () => Array.from(list.v), {
    html: "<li></li>",
    wire: (root, d0) => {
      let d = d0;
      const t = document.createTextNode("");
      root.appendChild(t);
      bind(c, [], () => {
        t.data = d.txt;
      });
      return (nd) => {
        d = nd;
      };
    },
    keyOf: (d) => d.id,
  });
  const [n1, n2] = [parent.childNodes[0], parent.childNodes[1]];

  list.v.push({ id: 3, txt: "c" });
  await tick();
  assert.strictEqual(parent.childNodes.length, 4);
  assert.strictEqual(parent.childNodes[0], n1, "no rebuild on append");
  assert.strictEqual(parent.childNodes[2].outerHTML, "<li>c</li>");

  list.v.reverse();
  await tick();
  assert.strictEqual(parent.childNodes[0].outerHTML, "<li>c</li>");
  assert.strictEqual(parent.childNodes[1], n2, "same node object moved");
  assert.strictEqual(parent.childNodes[2], n1, "same node object moved");

  // patch: same key, new data — node reused, text refreshed via runScope
  list.v[2] = { id: 1, txt: "A!" };
  await tick();
  assert.strictEqual(parent.childNodes[2], n1, "patched in place");
  assert.strictEqual(parent.childNodes[2].outerHTML, "<li>A!</li>");
});

await test("forBlock html/wire: removed item's binds are dead; empty <-> filled", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  const hi = box(c, 1, "!");
  let arr = ["a", "b"];
  const runs = { a: 0, b: 0 };
  forBlock(c, anchor, [0], () => arr.slice(), {
    html: "<li></li>",
    wire: (root, d0) => {
      const key = d0;
      const t = document.createTextNode("");
      root.appendChild(t);
      bind(c, [1], () => {
        runs[key]++;
        t.data = key + hi.v;
      });
    },
    keyOf: (d) => d,
  });
  assert.deepStrictEqual(runs, { a: 1, b: 1 });
  arr = ["a"];
  markVar(c, 0);
  await tick();
  hi.v = "!!";
  await tick();
  assert.strictEqual(runs.b, 1, "removed item's bind unregistered");
  arr = [];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "|");
  arr = ["b"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(parent.childNodes[0].outerHTML, "<li>b!!</li>");
});

await test("forBlock html/wire: nested ifBlock re-evaluates on patch and dies with its item", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  let arr = [{ id: 1, on: false }];
  let innerBinds = 0;
  const flag = box(c, 1, "*");
  forBlock(c, anchor, [0], () => arr.slice(), {
    html: "<li></li>",
    wire: (root, d0) => {
      let d = d0;
      const a = anchorAppend(root);
      ifBlock(c, a, [], () => d.on, () => {
        const el = document.createElement("em");
        bind(c, [1], () => {
          innerBinds++;
          el.setAttribute("data-x", flag.v);
        });
        return el;
      });
      return (nd) => {
        d = nd;
      };
    },
    keyOf: (d) => d.id,
  });
  assert.strictEqual(parent.childNodes[0].childNodes.length, 1, "just the if anchor");

  // patch with on=true — runScope re-runs the nested ifBlock's bind
  arr = [{ id: 1, on: true }];
  markVar(c, 0);
  await tick();
  assert.strictEqual(parent.childNodes[0].childNodes[0].tag, "em", "branch shown after patch");
  assert.strictEqual(innerBinds, 1);

  // removing the item drops the nested branch's binds (scope homing)
  arr = [];
  markVar(c, 0);
  await tick();
  flag.v = "**";
  await tick();
  assert.strictEqual(innerBinds, 1, "nested branch bind dead after item removal");
});

await test("forBlock html/wire: duplicate keys warn and fall back to index keys", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  const warnings = [];
  forBlock(c, anchor, [0], () => ["x", "x"], {
    html: "<li></li>",
    wire: (root, d0) => {
      root.appendChild(document.createTextNode(d0));
    },
    keyOf: (d) => d,
    onWarn: (m) => warnings.push(m),
  });
  assert.strictEqual(parent.childNodes.length, 3, "both items rendered");
  assert.strictEqual(warnings.length, 1, "one duplicate-key warning");
});

await test("forBlock update() forces a reconcile (used by outer patch paths)", async () => {
  const c = createContext(null);
  const parent = document.createElement("ul");
  const anchor = anchorAppend(parent);
  let arr = ["a"];
  const blk = forBlock(c, anchor, [], () => arr.slice(), {
    html: "<li></li>",
    wire: (root, d0) => {
      root.appendChild(document.createTextNode(d0));
    },
    keyOf: (d) => d,
  });
  assert.strictEqual(parent.childNodes.length, 2);
  arr = ["a", "b"];
  blk.update(); // no dep fired; forced refresh
  assert.strictEqual(parent.childNodes.length, 3);
  assert.strictEqual(parent.childNodes[1].outerHTML, "<li>b</li>");
});

console.log("blocks-compiled.test.mjs: all " + passed + " tests passed");
