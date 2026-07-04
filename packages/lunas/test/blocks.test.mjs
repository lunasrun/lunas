// blocks.test.mjs — ifBlock / forBlock / mountChild / anchor tests against a
// minimal fake DOM (only the surface the runtime touches).
// Run: node packages/lunas/test/blocks.test.mjs

import assert from "node:assert";
import { createContext, bind, markVar } from "../src/core.mjs";
import { box, deepBox } from "../src/boxes.mjs";
import {
  anchorBefore,
  anchorBeforeSplit,
  anchorAppend,
  on,
} from "../src/dom.mjs";
import { ifBlock, forBlock, mountChild } from "../src/blocks.mjs";

const tick = () => new Promise((r) => setTimeout(r, 0));

// ---------------------------------------------------------------------------
// Minimal fake DOM: parentNode/childNodes, insertBefore/appendChild/remove,
// before, splitText, addEventListener. Nothing else — this doubles as a check
// that the runtime touches only this narrow surface.
// ---------------------------------------------------------------------------
class FakeNode {
  constructor(doc, kind, data) {
    this.ownerDocument = doc;
    this.kind = kind; // "element" | "text"
    this.data = data || "";
    this.childNodes = [];
    this.parentNode = null;
    this._listeners = {};
  }
  insertBefore(n, ref) {
    if (ref !== null && ref !== undefined && ref.parentNode !== this) {
      throw new Error("insertBefore: refNode is not a child");
    }
    if (n.parentNode) n.parentNode._drop(n);
    const at =
      ref === null || ref === undefined
        ? this.childNodes.length
        : this.childNodes.indexOf(ref);
    this.childNodes.splice(at, 0, n);
    n.parentNode = this;
    return n;
  }
  appendChild(n) {
    return this.insertBefore(n, null);
  }
  _drop(n) {
    const i = this.childNodes.indexOf(n);
    if (i < 0) throw new Error("_drop: not a child");
    this.childNodes.splice(i, 1);
    n.parentNode = null;
  }
  remove() {
    if (this.parentNode) this.parentNode._drop(this);
  }
  before(n) {
    this.parentNode.insertBefore(n, this);
  }
  get nextSibling() {
    if (!this.parentNode) return null;
    const sib = this.parentNode.childNodes;
    return sib[sib.indexOf(this) + 1] || null;
  }
  splitText(off) {
    const tail = this.ownerDocument.createTextNode(this.data.slice(off));
    this.data = this.data.slice(0, off);
    this.parentNode.insertBefore(tail, this.nextSibling);
    return tail;
  }
  addEventListener(ev, fn) {
    (this._listeners[ev] || (this._listeners[ev] = [])).push(fn);
  }
  dispatch(ev) {
    for (const fn of this._listeners[ev] || []) fn();
  }
}
const fakeDoc = {
  createTextNode(data) {
    return new FakeNode(fakeDoc, "text", data);
  },
  createElement(tag) {
    const n = new FakeNode(fakeDoc, "element", "");
    n.tag = tag;
    return n;
  },
};
// visible order of a parent's children, text data or element tag; anchors are
// empty text nodes and shown as "|"
const shape = (parent) =>
  parent.childNodes
    .map((n) => (n.kind === "text" ? (n.data === "" ? "|" : n.data) : "<" + n.tag + ">"))
    .join(" ");

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- anchors -----------------------------------------------------------------

await test("anchorBefore / anchorAppend / anchorBeforeSplit", () => {
  const parent = fakeDoc.createElement("div");
  const el = fakeDoc.createElement("p");
  parent.appendChild(el);
  const a1 = anchorBefore(el);
  assert.strictEqual(shape(parent), "| <p>");
  assert.strictEqual(a1.data, "");

  const a2 = anchorAppend(parent);
  assert.strictEqual(shape(parent), "| <p> |");
  assert.strictEqual(a2.parentNode, parent);

  const t = fakeDoc.createTextNode("hello world");
  parent.appendChild(t);
  const a3 = anchorBeforeSplit(t, 5);
  // "hello" | " world" — anchor sits before the tail
  assert.strictEqual(shape(parent), "| <p> | hello |  world");
  assert.strictEqual(t.data, "hello");
  assert.strictEqual(a3.nextSibling.data, " world");
});

// --- ifBlock -----------------------------------------------------------------

await test("ifBlock toggles single-root branch at the anchor", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("div");
  const anchor = anchorAppend(parent);
  const show = box(c, 0, false);
  let makes = 0;
  ifBlock(c, anchor, [0], () => show.v, () => {
    makes++;
    return fakeDoc.createElement("span");
  });
  assert.strictEqual(shape(parent), "|", "initially hidden");
  show.v = true;
  await tick();
  assert.strictEqual(shape(parent), "<span> |");
  show.v = false;
  await tick();
  assert.strictEqual(shape(parent), "|");
  show.v = true;
  await tick();
  assert.strictEqual(shape(parent), "<span> |");
  assert.strictEqual(makes, 2, "branch rebuilt per show");
});

await test("ifBlock multi-root branch inserts/removes as a group", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("div");
  parent.appendChild(fakeDoc.createTextNode("head"));
  const anchor = anchorAppend(parent);
  parent.appendChild(fakeDoc.createTextNode("tail"));
  const show = box(c, 0, true);
  ifBlock(c, anchor, [0], () => show.v, () => [
    fakeDoc.createElement("a"),
    fakeDoc.createTextNode("mid"),
    fakeDoc.createElement("b"),
  ]);
  assert.strictEqual(shape(parent), "head <a> mid <b> | tail");
  show.v = false;
  await tick();
  assert.strictEqual(shape(parent), "head | tail");
});

await test("ifBlock teardown: inner binds dropped on hide; destroy() unbinds all", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("div");
  const anchor = anchorAppend(parent);
  const show = box(c, 0, true);
  const label = box(c, 1, "x");
  let innerRuns = 0;
  const blk = ifBlock(c, anchor, [0], () => show.v, () => {
    const el = fakeDoc.createElement("span");
    bind(c, [1], () => {
      innerRuns++;
      el.data = label.v;
    });
    return el;
  });
  assert.strictEqual(innerRuns, 1);
  label.v = "y";
  await tick();
  assert.strictEqual(innerRuns, 2, "inner bind live while shown");
  show.v = false;
  await tick();
  label.v = "z";
  await tick();
  assert.strictEqual(innerRuns, 2, "inner bind dead after hide");
  show.v = true;
  await tick();
  assert.strictEqual(innerRuns, 3, "fresh branch has fresh bind");
  blk.destroy();
  assert.strictEqual(shape(parent), "|", "destroy removes content");
  show.v = false;
  show.v = true;
  label.v = "w";
  await tick();
  assert.strictEqual(innerRuns, 3, "nothing runs after destroy");
});

// --- forBlock ------------------------------------------------------------------

await test("forBlock initial render + reorder reuses nodes via reconciler", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("ul");
  const anchor = anchorAppend(parent);
  const list = deepBox(c, 0, ["a", "b", "c"]);
  let makes = 0;
  forBlock(c, anchor, [0], () => Array.from(list.v), {
    make: (d) => {
      makes++;
      return fakeDoc.createTextNode(d);
    },
    keyOf: (d) => d,
  });
  assert.strictEqual(shape(parent), "a b c |");
  assert.strictEqual(makes, 3);
  const before = parent.childNodes.slice(0, 3);

  list.v.reverse();
  await tick();
  assert.strictEqual(shape(parent), "c b a |");
  assert.strictEqual(makes, 3, "reorder created no new nodes");
  const after = parent.childNodes.slice(0, 3);
  assert.strictEqual(after[0], before[2], "same node objects, moved");
  assert.strictEqual(after[2], before[0]);
});

await test("forBlock insert/remove mixed; empty <-> filled", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("ul");
  const anchor = anchorAppend(parent);
  let arr = ["a", "b", "c", "d"];
  forBlock(c, anchor, [0], () => arr, {
    make: (d) => fakeDoc.createTextNode(d),
    keyOf: (d) => d,
  });
  arr = ["d", "a", "x", "c"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "d a x c |");
  arr = [];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "|");
  arr = ["q"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "q |");
});

await test("forBlock multi-root items move/remove as groups", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("dl");
  const anchor = anchorAppend(parent);
  let arr = ["a", "b"];
  forBlock(c, anchor, [0], () => arr, {
    // each item is a <dt>/<dd>-style pair: two top-level nodes
    make: (d) => [fakeDoc.createTextNode(d + "1"), fakeDoc.createTextNode(d + "2")],
    keyOf: (d) => d,
  });
  assert.strictEqual(shape(parent), "a1 a2 b1 b2 |");
  arr = ["b", "a"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "b1 b2 a1 a2 |");
  arr = ["a"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(shape(parent), "a1 a2 |");
});

await test("forBlock item scope teardown: removed item's binds are dead", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("ul");
  const anchor = anchorAppend(parent);
  const hi = box(c, 1, "!");
  let arr = ["a", "b"];
  const runsPerKey = { a: 0, b: 0 };
  forBlock(c, anchor, [0], () => arr, {
    make: (d) => {
      const n = fakeDoc.createTextNode(d);
      bind(c, [1], () => {
        runsPerKey[d]++;
        n.data = d + hi.v;
      });
      return n;
    },
    keyOf: (d) => d,
  });
  assert.deepStrictEqual(runsPerKey, { a: 1, b: 1 });
  hi.v = "!!";
  await tick();
  assert.deepStrictEqual(runsPerKey, { a: 2, b: 2 });
  arr = ["a"]; // remove b
  markVar(c, 0);
  await tick();
  hi.v = "!!!";
  await tick();
  assert.deepStrictEqual(runsPerKey, { a: 3, b: 2 }, "b's bind unregistered");
});

await test("forBlock destroy() removes items and unbinds everything", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("ul");
  const anchor = anchorAppend(parent);
  const dep = box(c, 1, 0);
  let itemRuns = 0;
  let arr = ["a", "b"];
  const blk = forBlock(c, anchor, [0], () => arr, {
    make: (d) => {
      const n = fakeDoc.createTextNode(d);
      bind(c, [1], () => itemRuns++);
      return n;
    },
    keyOf: (d) => d,
  });
  assert.strictEqual(itemRuns, 2);
  blk.destroy();
  assert.strictEqual(shape(parent), "|");
  dep.v = 1;
  arr = ["z"];
  markVar(c, 0);
  await tick();
  assert.strictEqual(itemRuns, 2, "no item binds after destroy");
  assert.strictEqual(shape(parent), "|", "list bind itself is dead");
});

await test("forBlock patch updates item in place (same key, new data)", async () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("ul");
  const anchor = anchorAppend(parent);
  let arr = [{ id: 1, txt: "one" }];
  forBlock(c, anchor, [0], () => arr, {
    make: (d) => fakeDoc.createTextNode(d.txt),
    keyOf: (d) => d.id,
    patch: (n, d) => {
      n.data = d.txt;
    },
  });
  const node = parent.childNodes[0];
  arr = [{ id: 1, txt: "uno" }];
  markVar(c, 0);
  await tick();
  assert.strictEqual(parent.childNodes[0], node, "node reused, not rebuilt");
  assert.strictEqual(shape(parent), "uno |");
});

// --- mountChild + on ----------------------------------------------------------

await test("mountChild inserts child root before anchor; unmount removes", () => {
  const c = createContext(null);
  const parent = fakeDoc.createElement("div");
  const anchor = anchorAppend(parent);
  const Child = (props) => {
    const el = fakeDoc.createElement("child");
    el.data = props.name;
    return el;
  };
  const m = mountChild(c, anchor, Child, { name: "kid" });
  assert.strictEqual(shape(parent), "<child> |");
  m.unmount();
  assert.strictEqual(shape(parent), "|");
});

await test("on() wires an event listener (fake dispatch)", () => {
  const el = fakeDoc.createElement("button");
  let clicks = 0;
  on(el, "click", () => clicks++);
  el.dispatch("click");
  assert.strictEqual(clicks, 1);
});

console.log("blocks.test.mjs: all " + passed + " tests passed");
