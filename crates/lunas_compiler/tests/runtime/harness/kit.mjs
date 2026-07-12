// kit.mjs — the ergonomic assertion + interaction kit handed to a case's
// steps.mjs. Case authors write:
//
//   export default async ({ root, mount, tick, $, $$, click, setValue,
//                            dispatch, expect }) => { ... };
//
// All helpers operate over the dom-shim tree of the mounted component.
// See ../README.md for the full API reference.

import { normalizeInner } from "./normalize.mjs";

// A tick resolves after the runtime's microtask/setTimeout batch flushes.
export const tick = () => new Promise((r) => setTimeout(r, 0));

// Depth-first search over a mounted root (or array of roots) for elements
// matching a narrow CSS selector: "tag", ".class", "#id", or "tag.class".
function makeQuery(roots) {
  const list = Array.isArray(roots) ? roots : [roots];

  const matches = (el, sel) => {
    sel = sel.trim();
    // tag.class combos and bare forms.
    let tag = null;
    let cls = null;
    let id = null;
    const m = sel.match(/^([a-zA-Z][\w-]*)?(?:\.([\w-]+))?(?:#([\w-]+))?$/);
    if (m) {
      tag = m[1] || null;
      cls = m[2] || null;
      id = m[3] || null;
    } else if (sel[0] === ".") {
      cls = sel.slice(1);
    } else if (sel[0] === "#") {
      id = sel.slice(1);
    } else {
      tag = sel;
    }
    if (tag && el.tag !== tag) return false;
    if (cls && !(el.getAttribute("class") || "").split(/\s+/).includes(cls))
      return false;
    if (id && el.getAttribute("id") !== id) return false;
    return true;
  };

  const walkAll = (sel) => {
    const out = [];
    const visit = (n) => {
      for (const ch of n.childNodes || []) {
        if (ch.kind === "element" && matches(ch, sel)) out.push(ch);
        visit(ch);
      }
    };
    for (const r of list) {
      if (r.kind === "element" && matches(r, sel)) out.push(r);
      visit(r);
    }
    return out;
  };

  const $ = (sel) => {
    const found = walkAll(sel);
    if (found.length === 0) throw new Error(`$: no element matched \`${sel}\``);
    return found[0];
  };
  const $$ = (sel) => walkAll(sel);
  return { $, $$ };
}

// Resolve a selector-or-node argument to a node.
function resolve($, target) {
  if (typeof target === "string") return $(target);
  if (target && typeof target === "object") return target;
  throw new Error(`expected a selector string or node, got ${target}`);
}

export function makeKit(roots) {
  const { $, $$ } = makeQuery(roots);

  const click = async (target) => {
    resolve($, target).dispatch("click");
    await tick();
  };

  const dispatch = async (target, ev, detail) => {
    resolve($, target).dispatch(ev, detail);
    await tick();
  };

  // setValue writes the IDL value then fires an `input` event (two-way path).
  const setValue = async (target, value) => {
    const el = resolve($, target);
    el.value = value;
    el.dispatch("input");
    await tick();
  };

  // expect(x): x is a selector STRING or a NODE — always element assertions:
  //   .text(s) .html(s) .attr(name,v) .value(v) .prop(name,v) .count(n)
  //   .hasClass(c)
  // For raw value comparisons use the top-level `equal(actual, expected)` helper
  // (strings are always treated as selectors here, so `expect("a,b")` would try
  // to match elements — use `equal(...)` for plain values).
  const expect = (x) => {
    if (typeof x === "string") {
      return elementAssertions(x, $$(x));
    }
    if (x && typeof x === "object" && x.kind) {
      return elementAssertions(describe(x), [x]);
    }
    throw new Error(
      "expect(x): x must be a selector string or a node; use `equal(a, b)` for values"
    );
  };

  // equal(actual, expected) — a plain value assertion for anything not a DOM
  // node (e.g. a joined label string built by the step).
  const equal = (actual, expected) => {
    if (actual !== expected) {
      throw new Error(
        `equal: got ${JSON.stringify(actual)}, want ${JSON.stringify(expected)}`
      );
    }
  };

  return { root: Array.isArray(roots) ? roots[0] : roots, roots, mount: null,
           tick, $, $$, click, dispatch, setValue, expect, equal };
}

function describe(node) {
  return node.kind === "element" ? `<${node.tag}>` : "text";
}

function elementAssertions(label, nodes) {
  const first = () => {
    if (nodes.length === 0)
      throw new Error(`expect(${label}): no element matched`);
    return nodes[0];
  };
  return {
    text(expected) {
      const got = normalizeInner(first());
      if (got !== expected)
        throw new Error(
          `expect(${label}).text: got ${JSON.stringify(got)}, want ${JSON.stringify(expected)}`
        );
      return this;
    },
    html(expected) {
      const got = normalizeInner(first());
      if (got !== expected)
        throw new Error(
          `expect(${label}).html: got ${JSON.stringify(got)}, want ${JSON.stringify(expected)}`
        );
      return this;
    },
    attr(name, expected) {
      const got = first().getAttribute(name);
      if (got !== expected)
        throw new Error(
          `expect(${label}).attr(${name}): got ${JSON.stringify(got)}, want ${JSON.stringify(expected)}`
        );
      return this;
    },
    // IDL property (el.value, el.checked, ...) — used for `:value` binds, which
    // set the property, not the attribute.
    prop(name, expected) {
      const got = first()[name];
      if (got !== expected)
        throw new Error(
          `expect(${label}).prop(${name}): got ${JSON.stringify(got)}, want ${JSON.stringify(expected)}`
        );
      return this;
    },
    // Shorthand for .prop("value", ...).
    value(expected) {
      return this.prop("value", expected);
    },
    hasClass(cls) {
      const has = (first().getAttribute("class") || "")
        .split(/\s+/)
        .includes(cls);
      if (!has)
        throw new Error(`expect(${label}).hasClass(${cls}): class missing`);
      return this;
    },
    count(expected) {
      if (nodes.length !== expected)
        throw new Error(
          `expect(${label}).count: got ${nodes.length}, want ${expected}`
        );
      return this;
    },
  };
}
