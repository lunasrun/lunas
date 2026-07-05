// provide.test.mjs — provide/inject across the parent-context chain.
// Run: node packages/lunas/test/provide.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { mountChild } from "../src/blocks.mjs";
import { provide, inject, hasInjection } from "../src/provide.mjs";

installDom();

// Build a chain of N contexts linked as real mountChild children so `parent`
// is populated exactly as the runtime would. Returns the leaf handle chain.
function chain(depth, setups) {
  // setups[i](c) runs on level i. Returns array of contexts [root..leaf].
  const ctxs = [];
  const makeFactory = (i) => (props) => {
    const root = document.createElement("div");
    const c = createContext(root);
    root.__lunasCtx = c;
    ctxs[i] = c;
    if (setups[i]) setups[i](c);
    if (i + 1 < depth) {
      const a = anchorAppend(root);
      mountChild(c, a, makeFactory(i + 1), {});
    }
    return root;
  };
  makeFactory(0)();
  return ctxs;
}

test("inject resolves a value provided by an ancestor (3 levels)", () => {
  const ctxs = chain(3, [
    (c) => provide(c, "theme", "dark"),
    () => {},
    () => {},
  ]);
  assert.equal(inject(ctxs[2], "theme"), "dark");
  assert.equal(inject(ctxs[1], "theme"), "dark");
  assert.equal(inject(ctxs[0], "theme"), "dark");
});

test("nearest ancestor shadows a farther one", () => {
  const ctxs = chain(3, [
    (c) => provide(c, "k", "root"),
    (c) => provide(c, "k", "mid"), // shadows root
    () => {},
  ]);
  assert.equal(inject(ctxs[2], "k"), "mid");
  assert.equal(inject(ctxs[0], "k"), "root");
});

test("inject returns default when nothing provides the key", () => {
  const ctxs = chain(2, [() => {}, () => {}]);
  assert.equal(inject(ctxs[1], "missing", "fallback"), "fallback");
  assert.equal(inject(ctxs[1], "missing"), undefined);
});

test("symbol keys are supported and distinct from strings", () => {
  const KEY = Symbol("store");
  const ctxs = chain(2, [(c) => provide(c, KEY, { count: 1 }), () => {}]);
  assert.deepEqual(inject(ctxs[1], KEY), { count: 1 });
  assert.equal(inject(ctxs[1], "store", "no"), "no"); // string "store" ≠ Symbol
});

test("hasInjection distinguishes provided-undefined from absent", () => {
  const ctxs = chain(2, [(c) => provide(c, "maybe", undefined), () => {}]);
  assert.equal(hasInjection(ctxs[1], "maybe"), true);
  assert.equal(inject(ctxs[1], "maybe", "def"), undefined); // provided value wins
  assert.equal(hasInjection(ctxs[1], "nope"), false);
});

test("root component with no parent injects the default", () => {
  const c = createContext(document.createElement("div"));
  assert.equal(c.parent, null);
  assert.equal(inject(c, "x", 42), 42);
});
