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
    setup(c, props || {});
    return root;
  };
}

// refs(root, paths) — positional navigation to dynamic elements.
// paths = [[0], [1, 2], ...]: each path is child indices from root.
export const refs = (root, paths) =>
  paths.map((p) => p.reduce((n, i) => n.childNodes[i], root));

export const on = (el, ev, fn) => el.addEventListener(ev, fn);

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
