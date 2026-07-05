// provide.edge.test.mjs — additional edge-focused coverage for provide.mjs
// beyond provide.test.mjs: deeper chains (4-5 levels), sibling isolation
// (one branch's provide doesn't leak to another), overwriting a provide on
// the same context, provide after a child already mounted (late provide),
// and inject reading a falsy-but-defined value correctly.
// Run: node packages/lunas/test/provide.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import { mountChild } from "../src/blocks.mjs";
import { provide, inject, hasInjection } from "../src/provide.mjs";

installDom();

// Build a chain of N contexts linked as real mountChild children.
function chain(depth, setups) {
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

// Build a tree with a branching factor: root -> [branchA, branchB], each
// with its own linear chain of `depth` further levels.
function tree(setupRoot, branches) {
  const root = document.createElement("div");
  const rootCtx = createContext(root);
  root.__lunasCtx = rootCtx;
  if (setupRoot) setupRoot(rootCtx);
  const branchCtxs = branches.map((setups) => {
    const a = anchorAppend(root);
    const ctxs = [];
    const makeFactory = (i) => (props) => {
      const r = document.createElement("div");
      const c = createContext(r);
      r.__lunasCtx = c;
      c.parent = null; // will be set by mountChild for the top of this branch
      ctxs[i] = c;
      if (setups[i]) setups[i](c);
      if (i + 1 < setups.length) {
        const aa = anchorAppend(r);
        mountChild(c, aa, makeFactory(i + 1), {});
      }
      return r;
    };
    mountChild(rootCtx, a, makeFactory(0), {});
    return ctxs;
  });
  return { rootCtx, branchCtxs };
}

// -- deeper chains -------------------------------------------------------------

test("inject resolves through a 5-level chain", () => {
  const ctxs = chain(5, [
    (c) => provide(c, "k", "root-value"),
    () => {},
    () => {},
    () => {},
    () => {},
  ]);
  assert.equal(inject(ctxs[4], "k"), "root-value");
});

test("shadowing at the middle of a 5-level chain only affects descendants below it", () => {
  const ctxs = chain(5, [
    (c) => provide(c, "k", "L0"),
    () => {},
    (c) => provide(c, "k", "L2"), // shadow starting here
    () => {},
    () => {},
  ]);
  assert.equal(inject(ctxs[0], "k"), "L0");
  assert.equal(inject(ctxs[1], "k"), "L0");
  assert.equal(inject(ctxs[2], "k"), "L2");
  assert.equal(inject(ctxs[3], "k"), "L2");
  assert.equal(inject(ctxs[4], "k"), "L2");
});

// -- sibling branch isolation --------------------------------------------------

test("a provide in one branch never leaks into a sibling branch", () => {
  const { branchCtxs } = tree(null, [
    [(c) => provide(c, "theme", "dark")],
    [() => {}],
  ]);
  assert.equal(inject(branchCtxs[0][0], "theme"), "dark");
  assert.equal(inject(branchCtxs[1][0], "theme", "fallback"), "fallback",
    "sibling branch never saw branch A's provide");
});

test("root-level provide is visible to every branch (common ancestor)", () => {
  const { branchCtxs } = tree((root) => provide(root, "app", "shared"), [
    [() => {}],
    [() => {}],
  ]);
  assert.equal(inject(branchCtxs[0][0], "app"), "shared");
  assert.equal(inject(branchCtxs[1][0], "app"), "shared");
});

// -- overwriting a provide on the same context ---------------------------------

test("a second provide() call with the same key on the same context overwrites the first", () => {
  const c = createContext(document.createElement("div"));
  provide(c, "k", "first");
  assert.equal(inject(c, "k"), "first");
  provide(c, "k", "second");
  assert.equal(inject(c, "k"), "second");
});

test("provide() returns the value it stored", () => {
  const c = createContext(document.createElement("div"));
  const ret = provide(c, "k", { a: 1 });
  assert.deepEqual(ret, { a: 1 });
});

// -- late provide: child mounted before parent's provide() runs ---------------

test("a provide registered on the parent after the child already mounted is still visible (chain is live, walked lazily on inject)", () => {
  const parentCtx = createContext(document.createElement("div"));
  const a = anchorAppend(parentCtx.root);
  let childCtx;
  const childFactory = () => {
    const root = document.createElement("span");
    childCtx = createContext(root);
    root.__lunasCtx = childCtx;
    return root;
  };
  mountChild(parentCtx, a, childFactory, {});
  // Parent provides AFTER the child was already mounted — inject walks the
  // live chain at call time, so this still resolves.
  provide(parentCtx, "late", "value");
  assert.equal(inject(childCtx, "late"), "value");
});

// -- falsy-but-provided values --------------------------------------------------

test("inject distinguishes a provided falsy value (0, '', false) from absent", () => {
  const ctxs = chain(2, [
    (c) => {
      provide(c, "zero", 0);
      provide(c, "empty", "");
      provide(c, "falseVal", false);
    },
    () => {},
  ]);
  assert.strictEqual(inject(ctxs[1], "zero", "def"), 0);
  assert.strictEqual(inject(ctxs[1], "empty", "def"), "");
  assert.strictEqual(inject(ctxs[1], "falseVal", "def"), false);
  assert.strictEqual(hasInjection(ctxs[1], "zero"), true);
});

// -- multiple distinct keys on one context --------------------------------------

test("a single context can provide many independent keys", () => {
  const c = createContext(document.createElement("div"));
  provide(c, "a", 1);
  provide(c, "b", 2);
  provide(c, "c", 3);
  assert.equal(inject(c, "a"), 1);
  assert.equal(inject(c, "b"), 2);
  assert.equal(inject(c, "c"), 3);
  assert.equal(hasInjection(c, "d"), false);
});

// -- Symbol keys don't collide with same-named string keys across levels ------

test("a Symbol key and a same-spelled string key coexist without collision across the chain", () => {
  const SYM = Symbol("theme");
  const ctxs = chain(3, [
    (c) => {
      provide(c, SYM, "symbol-value");
      provide(c, "theme", "string-value");
    },
    () => {},
    () => {},
  ]);
  assert.equal(inject(ctxs[2], SYM), "symbol-value");
  assert.equal(inject(ctxs[2], "theme"), "string-value");
});

test("two distinct Symbol() calls with the same description are different keys", () => {
  const a = Symbol("dup");
  const b = Symbol("dup");
  const c = createContext(document.createElement("div"));
  provide(c, a, "A");
  assert.equal(inject(c, a), "A");
  assert.equal(inject(c, b, "missing"), "missing");
});
