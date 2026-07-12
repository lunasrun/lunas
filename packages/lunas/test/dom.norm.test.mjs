// dom.norm.test.mjs — normClass / normStyle / setClass / setStyle plus
// component / fragment / refs from src/dom.mjs. Covers string | object | array |
// nested-array class/style, falsy filtering, camelCase->kebab, static merge,
// and null/undefined/number values.
// Run: node packages/lunas/test/dom.norm.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
const document = installDom();

import {
  normClass,
  normStyle,
  setClass,
  setStyle,
  component,
  fragment,
  refs,
  on,
} from "../src/dom.mjs";

let passed = 0;
let skipped = 0;
function test(name, fn) {
  fn();
  passed++;
  console.log("  ok  " + name);
}
// test.skip — record a known-divergence test without running/failing it. The
// reason is printed so the PR reviewer sees the deferred assertion.
test.skip = (name, reason) => {
  skipped++;
  console.log("  SKIP  " + name + (reason ? "  (" + reason + ")" : ""));
};

// --- normClass ---------------------------------------------------------------

test("normClass: string passes through trimmed", () => {
  assert.strictEqual(normClass("btn primary"), "btn primary");
  assert.strictEqual(normClass("  spaced  "), "spaced");
});

test("normClass: object keeps truthy-valued keys only", () => {
  assert.strictEqual(
    normClass({ a: true, b: false, c: 1, d: 0, e: "x", f: "" }),
    "a c e"
  );
});

test("normClass: array flattens and joins", () => {
  assert.strictEqual(normClass(["a", "b", "c"]), "a b c");
});

test("normClass: nested arrays flatten recursively", () => {
  assert.strictEqual(
    normClass(["a", ["b", ["c", "d"]], "e"]),
    "a b c d e"
  );
});

test("normClass: mixed array of strings + objects", () => {
  assert.strictEqual(
    normClass(["base", { active: true, hidden: false }, ["extra"]]),
    "base active extra"
  );
});

test("normClass: nullish/false/empty-string entries in arrays are dropped", () => {
  // null, undefined, false, "" are all dropped (each normalizes to "").
  assert.strictEqual(
    normClass(["a", null, undefined, false, "", "b"]),
    "a b"
  );
});

// FIXED (was a KNOWN DIVERGENCE): a falsy *number* (0 / NaN) inside a class
// array is now dropped, matching Vue's :class array semantics. Previously
// normClass(0) stringified to "0" and leaked in as a bogus "0" class token
// because the array branch only skipped entries whose normalized result was
// "" (empty string), and normClass(0) === "0" (non-empty). The fix adds an
// `if (!v) continue;` guard in the array loop of src/dom.mjs so ALL falsy
// values (0, NaN, "", null, undefined, false) are dropped as bare array
// items — this does NOT affect :style object numeric VALUES (`{width: 0}`
// stays legitimate; only the :class ARRAY-ITEM path changed).
test("normClass: falsy numbers (0/NaN) in arrays are dropped (Vue-parity)", () => {
  assert.strictEqual(normClass(["a", 0, "b"]), "a b");
  assert.strictEqual(normClass(["a", NaN, "b"]), "a b");
  assert.strictEqual(normClass([0, "a"]), "a");
  assert.strictEqual(normClass([0, NaN, "", null, undefined, false]), "");
  assert.strictEqual(normClass(["x", 0, "y", NaN, "z"]), "x y z");
});

test("normClass: null / undefined / false => empty string", () => {
  assert.strictEqual(normClass(null), "");
  assert.strictEqual(normClass(undefined), "");
  assert.strictEqual(normClass(false), "");
});

test("normClass: number stringifies", () => {
  assert.strictEqual(normClass(42), "42");
});

test("normClass: empty object / empty array => empty string", () => {
  assert.strictEqual(normClass({}), "");
  assert.strictEqual(normClass([]), "");
});

// --- setClass (static + dynamic merge) ---------------------------------------

test("setClass merges static class with dynamic object", () => {
  const el = document.createElement("div");
  setClass(el, "static", { on: true, off: false });
  assert.strictEqual(el.getAttribute("class"), "static on");
});

test("setClass with only static (dynamic empty) keeps static", () => {
  const el = document.createElement("div");
  setClass(el, "base", null);
  assert.strictEqual(el.getAttribute("class"), "base");
});

test("setClass with neither static nor dynamic removes the attribute", () => {
  const el = document.createElement("div");
  el.setAttribute("class", "old");
  setClass(el, "", false);
  assert.strictEqual(el.getAttribute("class"), null);
});

test("setClass dynamic-only writes just the dynamic classes", () => {
  const el = document.createElement("div");
  setClass(el, "", ["x", "y"]);
  assert.strictEqual(el.getAttribute("class"), "x y");
});

// --- normStyle ---------------------------------------------------------------

test("normStyle: string passes through trimmed", () => {
  assert.strictEqual(normStyle("color: red"), "color: red");
});

test("normStyle: object camelCase keys become kebab-case props", () => {
  assert.strictEqual(
    normStyle({ backgroundColor: "red", fontSize: "12px" }),
    "background-color: red; font-size: 12px;"
  );
});

test("normStyle: custom properties (--x) pass through unchanged", () => {
  assert.strictEqual(
    normStyle({ "--brand": "blue", color: "black" }),
    "--brand: blue; color: black;"
  );
});

test("normStyle: null/false values in an object are skipped", () => {
  assert.strictEqual(
    normStyle({ color: "red", margin: null, padding: false, top: 0 }),
    "color: red; top: 0;"
  );
});

test("normStyle: numeric values stringify", () => {
  assert.strictEqual(normStyle({ zIndex: 10, opacity: 0.5 }), "z-index: 10; opacity: 0.5;");
});

test("normStyle: array merges left-to-right", () => {
  const out = normStyle([{ color: "red" }, "font-weight: bold", { top: "1px" }]);
  assert.match(out, /color: red;/);
  assert.match(out, /font-weight: bold/);
  assert.match(out, /top: 1px;/);
});

test("normStyle: null / undefined / false => empty string", () => {
  assert.strictEqual(normStyle(null), "");
  assert.strictEqual(normStyle(undefined), "");
  assert.strictEqual(normStyle(false), "");
});

test("normStyle: empty object => empty string", () => {
  assert.strictEqual(normStyle({}), "");
});

// --- setStyle (static + dynamic merge) ---------------------------------------

test("setStyle merges static style (adds trailing ;) with dynamic object", () => {
  const el = document.createElement("div");
  setStyle(el, "margin: 0", { color: "red" });
  assert.strictEqual(el.getAttribute("style"), "margin: 0; color: red;");
});

test("setStyle static already ending in ; merges cleanly", () => {
  const el = document.createElement("div");
  setStyle(el, "margin: 0;", { color: "red" });
  assert.strictEqual(el.getAttribute("style"), "margin: 0; color: red;");
});

test("setStyle with empty dynamic keeps the static style", () => {
  const el = document.createElement("div");
  setStyle(el, "display: none", null);
  assert.strictEqual(el.getAttribute("style"), "display: none;");
});

test("setStyle with nothing removes the attribute", () => {
  const el = document.createElement("div");
  el.setAttribute("style", "old: 1");
  setStyle(el, "", null);
  assert.strictEqual(el.getAttribute("style"), null);
});

// --- component / refs --------------------------------------------------------

test("component builds a root, parses skeleton HTML, exposes context", () => {
  const factory = component(
    "div",
    { id: "root" },
    "<span></span><b></b>",
    (c, props) => {
      c._seenProps = props;
    }
  );
  const root = factory({ hello: 1 });
  assert.strictEqual(root.tag, "div");
  assert.strictEqual(root.getAttribute("id"), "root");
  assert.strictEqual(root.childNodes.length, 2);
  assert.strictEqual(root.childNodes[0].tag, "span");
  assert.ok(root.__lunasCtx, "context attached to the root");
  assert.deepStrictEqual(root.__lunasCtx._seenProps, { hello: 1 });
});

test("refs: positional navigation to nested nodes", () => {
  const root = document.createElement("div");
  root.innerHTML = "<a></a><b><i></i><u></u></b>";
  const [aNode, uNode] = refs(root, [[0], [1, 1]]);
  assert.strictEqual(aNode.tag, "a");
  assert.strictEqual(uNode.tag, "u");
});

test("on() wires a listener resolved by refs against a component root", () => {
  const factory = component("div", {}, "<button></button>", () => {});
  const root = factory({});
  const [btn] = refs(root, [[0]]);
  let clicks = 0;
  on(btn, "click", () => clicks++);
  btn.dispatch("click");
  assert.strictEqual(clicks, 1);
});

// --- fragment (multi-root component) -----------------------------------------

test("fragment returns a node group carrying context on __lunasCtx", () => {
  const factory = fragment({}, "<a></a><b></b><c></c>", (c) => {
    c._wired = true;
  });
  const frag = factory({});
  assert.ok(Array.isArray(frag), "fragment is an array of nodes");
  assert.strictEqual(frag.length, 3);
  assert.strictEqual(frag[0].tag, "a");
  assert.strictEqual(frag[2].tag, "c");
  assert.ok(frag.__lunasCtx && frag.__lunasCtx._wired, "context on the group");
  // nodes are detached from the throwaway host (no parent).
  assert.strictEqual(frag[0].parentNode, null);
});

test("fragment wiring can capture refs against the parsed tree", () => {
  let captured = null;
  const factory = fragment({}, "<a></a><b><i></i></b>", (c) => {
    captured = refs(c.root, [[1, 0]])[0];
  });
  factory({});
  assert.strictEqual(captured.tag, "i", "ref resolved against the fragment host tree");
});

console.log(
  "\ndom.norm.test.mjs: " + passed + " passed" +
    (skipped ? ", " + skipped + " skipped" : "") + "."
);
