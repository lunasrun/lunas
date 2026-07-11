// dom.mjs — DOM construction & wiring helpers.
// See output-design.md §3, §5, §7–8.
//
// All node creation goes through ownerDocument when available so the module
// is testable against a fake DOM; in the browser this is the real document.

import { createContext } from "./core.mjs";

// component(tag, attrs, HTML, setup) — the compiled-component factory.
// Builds the root detached, bulk-parses the comment-free static skeleton via
// innerHTML, runs setup (wiring happens off-DOM), returns the root. The
// caller attaches it to the live DOM once.
export function component(tag, attrs, HTML, setup) {
  return (props) => {
    const root = document.createElement(tag);
    for (const k in attrs) root.setAttribute(k, attrs[k]);
    root.innerHTML = HTML; // ★ one native parse, detached
    const c = createContext(root);
    // Expose the context on the root so a parent's mountChild can push
    // reactive prop updates into the child's own reactive prop boxes
    // (output-design.md §6). The two contexts stay separate: a child event
    // mutates only the child's boxes, never the parent's.
    root.__lunasCtx = c;
    setup(c, props || {});
    return root;
  };
}

// refs(root, paths) — positional navigation to dynamic elements.
// paths = [[0], [1, 2], ...]: each path is child indices from root.
export const refs = (root, paths) =>
  paths.map((p) => p.reduce((n, i) => n.childNodes[i], root));

export const on = (el, ev, fn) => el.addEventListener(ev, fn);

// fragment(attrs, HTML, setup) — the compiled factory for a MULTI-ROOT
// component (output-design.md §7). Unlike `component`, there is no wrapper
// element: the static skeleton HTML has several top-level nodes, so the factory
// parses it into a throwaway host and returns the host's child nodes as a
// FRAGMENT — an Array of nodes carrying the component context on `__lunasCtx`
// (so a parent's mountChild can drive props exactly as for a single-root
// child). `attrs` is accepted for signature parity but not applied (there is no
// single root to attribute); it is ignored.
//
// The block helpers (mountChild/ifBlock/forBlock) already treat a node group as
// a unit via toNodes/firstNode, so a fragment mounts, moves, and unmounts as
// one. The context uses the first node as its `root` for positional refs
// against the whole child list.
export function fragment(attrs, HTML, setup) {
  return (props) => {
    // ★ one native parse, detached. Parse through a <template> (via
    // parseFragment) so a multi-root fragment whose top-level nodes are
    // table-context elements (`<tr>`, `<td>`, …) survives instead of being
    // dropped by a <div> parse context.
    const host = parseFragment(HTML, document);
    const c = createContext(host); // `c.root` = host: positional refs navigate it
    // Wire while the nodes are still attached to the host, so refs(c.root, …)
    // captured in setup resolve against the parsed tree (like a single-root
    // component). Binds keep the captured refs, not live host navigation.
    setup(c, props || {});
    // Snapshot AFTER setup so any top-level anchors created during wiring
    // (a top-level :if/:for/text slot) are part of the fragment node group and
    // travel with it. Then detach from the host (discarded) so the caller
    // inserts the fragment directly.
    const nodes = Array.from(host.childNodes);
    for (const n of nodes) n.remove();
    const frag = nodes;
    frag.__lunasCtx = c;
    return frag;
  };
}

// parseFragment(html, doc) — parse a bulk skeleton string into a detached
// container and return that container. The caller reads the parsed roots off
// the container's `.childNodes` (`childNodes[0]` for a single root, or
// `Array.from(childNodes)` for a multi-root group), so the container's identity
// never escapes.
//
// A `<template>` element is used rather than a throwaway `<div>`: a
// `<template>`'s `.content` fragment accepts ANY element — including
// table-context elements (`<tr>`, `<td>`, `<tbody>`, `<thead>`, `<col>`,
// `<colgroup>`) and `<option>` — which the HTML parser would otherwise DROP when
// assigned as the innerHTML of a `<div>` (a `<div>` is not a valid table/select
// insertion context). Without this, a `:for`/`:if` item whose skeleton is a
// `<tr>` parses to an empty `<div>` and `childNodes[0]` is `undefined`, crashing
// on `.childNodes` reads (the table-context bug). `<template>.content` is a
// DocumentFragment, and `.childNodes` on it is the same list the caller expects.
//
// Falls back to a `<div>` when `<template>` / `.content` is unavailable (e.g. a
// minimal test DOM without template support) so parsing of non-table skeletons
// keeps working everywhere.
export function parseFragment(html, doc) {
  if (typeof doc.createElement === "function") {
    const t = doc.createElement("template");
    // `.content` exists only on a real (or shim-supported) <template>.
    if (t && t.content) {
      t.innerHTML = html;
      return t.content;
    }
  }
  const el = doc.createElement("div");
  el.innerHTML = html;
  return el;
}

// fromHTML(html, near) — parse a static block skeleton (an :if branch, a :for
// item, …) into a detached container via one bulk parse, exactly like the
// component root build (§8: branches are built by their own parse when shown).
// `near` is any node used to reach the owner document, so blocks built inside a
// detached component still resolve a document (and tests can pass a fake-DOM
// node). Table-context skeletons (`<tr>`, `<td>`, …) survive because the parse
// goes through a `<template>` — see parseFragment.
export function fromHTML(html, near) {
  const doc =
    (near && near.ownerDocument) ||
    (typeof document !== "undefined" ? document : null);
  return parseFragment(html, doc);
}

// --- anchors -----------------------------------------------------------------
// Anchors are permanent EMPTY TEXT NODES created at wiring time (never
// comments — comments knock Blink off the fast-path parser; see
// output-design.md §2). Each helper returns the anchor.

function emptyText(near) {
  const doc =
    (near && near.ownerDocument) ||
    (typeof document !== "undefined" ? document : null);
  return doc.createTextNode("");
}

// anchorBefore(node) — anchor immediately before an existing node.
export function anchorBefore(node) {
  const a = emptyText(node);
  node.parentNode.insertBefore(a, node);
  return a;
}

// anchorBeforeSplit(textNode, utf16Offset) — split a static text node at the
// given UTF-16 offset and place the anchor between head and tail (i.e. the
// anchor sits before the tail). Used when a dynamic seam falls inside a text
// run.
export function anchorBeforeSplit(textNode, utf16Offset) {
  const tail = textNode.splitText(utf16Offset);
  const a = emptyText(textNode);
  tail.parentNode.insertBefore(a, tail);
  return a;
}

// anchorAppend(parent) — anchor as the last child of parent (e.g. a :for
// slot at the end of a container).
export function anchorAppend(parent) {
  const a = emptyText(parent);
  parent.appendChild(a);
  return a;
}

// --- class & style normalization (output-design.md §6, `:class` / `:style`) --
// Vue-parity semantics with Lunas syntax: `:class="expr"` where expr is a
// string | { cls: bool } | array (nested mix of those), and `:style="expr"`
// where expr is a string | { camelCaseProp: value } (arrays merge). The
// emitter special-cases the `class`/`style` attribute names and merges the
// normalized dynamic value with the element's static attribute.

// normClass(value) — flatten a class binding into a space-separated string.
// Falsy entries are dropped; object keys are included when their value is
// truthy; arrays are flattened recursively. Non-object/array values stringify.
export function normClass(value) {
  if (value == null || value === false) return "";
  if (typeof value === "string") return value.trim();
  if (Array.isArray(value)) {
    let out = "";
    for (const v of value) {
      // A falsy bare array item (0, NaN, "", null, undefined, false) is
      // dropped outright, matching Vue's :class array semantics — without
      // this, normClass(0) stringifies to "0" and leaks a bogus class token
      // (see dom.norm.test.mjs). This only affects bare array ITEMS; object
      // values (e.g. `:style="{width: 0}"`) are untouched.
      if (!v) continue;
      const s = normClass(v);
      if (s) out = out ? out + " " + s : s;
    }
    return out;
  }
  if (typeof value === "object") {
    let out = "";
    for (const k in value) {
      if (value[k]) out = out ? out + " " + k : k;
    }
    return out;
  }
  return String(value);
}

// setClass(el, staticClass, value) — merge the element's static class string
// with the normalized dynamic `value` and write the whole `class` attribute.
export function setClass(el, staticClass, value) {
  const dyn = normClass(value);
  const merged = staticClass ? (dyn ? staticClass + " " + dyn : staticClass) : dyn;
  if (merged) el.setAttribute("class", merged);
  else el.removeAttribute("class");
}

// camelToKebab(name) — `backgroundColor` -> `background-color`. Custom
// properties (`--x`) and already-kebab names pass through unchanged.
function camelToKebab(name) {
  if (name.charCodeAt(0) === 45 /* '-' */) return name; // --custom-prop
  return name.replace(/[A-Z]/g, (m) => "-" + m.toLowerCase());
}

// normStyle(value) — flatten a style binding into a `prop: value;` string.
// A string passes through; an object maps camelCase keys to kebab-case CSS
// properties; arrays merge left-to-right (later entries win).
export function normStyle(value) {
  if (value == null || value === false) return "";
  if (typeof value === "string") return value.trim();
  if (Array.isArray(value)) {
    let out = "";
    for (const v of value) {
      const s = normStyle(v);
      if (s) out = out ? out + (out.endsWith(";") ? " " : "; ") + s : s;
    }
    return out;
  }
  if (typeof value === "object") {
    let out = "";
    for (const k in value) {
      const v = value[k];
      if (v == null || v === false) continue;
      out += (out ? " " : "") + camelToKebab(k) + ": " + v + ";";
    }
    return out;
  }
  return String(value);
}

// setStyle(el, staticStyle, value) — merge the static style string with the
// normalized dynamic `value` and write the whole `style` attribute.
export function setStyle(el, staticStyle, value) {
  const dyn = normStyle(value);
  let base = staticStyle ? staticStyle.trim() : "";
  if (base && !base.endsWith(";")) base += ";";
  const merged = base ? (dyn ? base + " " + dyn : base) : dyn;
  if (merged) el.setAttribute("style", merged);
  else el.removeAttribute("style");
}
