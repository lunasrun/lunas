// dom-shim.mjs — a minimal, dependency-free DOM sufficient to *run* a compiled
// Lunas component: it supports the narrow surface the runtime + emitted code
// touch, plus an `innerHTML` setter that parses the comment-free, whitespace-
// free static skeleton the compiler emits.
//
// This is a test fixture, not a spec-compliant DOM. It parses only the subset
// the skeleton pass produces: nested tags, void elements, plain text, and
// double-quoted attributes. That is exactly what `build_skeleton` emits.
//
// Installing this sets a global `document`, which the runtime's
// `component()`/anchor helpers read.

class FakeNode {
  constructor(doc, kind, data) {
    this.ownerDocument = doc;
    this.kind = kind; // "element" | "text"
    this.data = data || "";
    this.childNodes = [];
    this.parentNode = null;
    this.attributes = {};
    this._listeners = {};
    this._props = {}; // IDL properties set via `el.value = …` etc.
    this._classes = new Set(); // backs classList
  }
  // Minimal classList (add/remove/contains) for transition class sequencing.
  get classList() {
    const set = this._classes;
    const self = this;
    return {
      add: (...cs) => {
        for (const c of cs) set.add(c);
        self.attributes.class = Array.from(set).join(" ");
      },
      remove: (...cs) => {
        for (const c of cs) set.delete(c);
        self.attributes.class = Array.from(set).join(" ");
      },
      contains: (c) => set.has(c),
      toArray: () => Array.from(set),
    };
  }
  // isConnected: reachable from a node flagged `_lunasAttached` (attach() sets
  // it on a mounted root), matching the runtime's liveness fallback.
  get isConnected() {
    let n = this;
    while (n) {
      if (n._lunasAttached) return true;
      n = n.parentNode;
    }
    return false;
  }
  insertBefore(n, ref) {
    if (ref !== null && ref !== undefined && ref.parentNode !== this) {
      // Real-DOM semantics: a reference node that is not a child is an error.
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
    if (i >= 0) this.childNodes.splice(i, 1);
    n.parentNode = null;
  }
  remove() {
    if (this.parentNode) this.parentNode._drop(this);
  }
  get nextSibling() {
    if (!this.parentNode) return null;
    const sib = this.parentNode.childNodes;
    return sib[sib.indexOf(this) + 1] || null;
  }
  get firstChild() {
    return this.childNodes[0] || null;
  }
  splitText(off) {
    const tail = this.ownerDocument.createTextNode(this.data.slice(off));
    this.data = this.data.slice(0, off);
    this.parentNode.insertBefore(tail, this.nextSibling);
    return tail;
  }
  setAttribute(name, value) {
    this.attributes[name] = String(value);
  }
  getAttribute(name) {
    return name in this.attributes ? this.attributes[name] : null;
  }
  removeAttribute(name) {
    delete this.attributes[name];
  }
  addEventListener(ev, fn) {
    (this._listeners[ev] || (this._listeners[ev] = [])).push(fn);
  }
  removeEventListener(ev, fn) {
    const ls = this._listeners[ev];
    if (ls) {
      const i = ls.indexOf(fn);
      if (i >= 0) ls.splice(i, 1);
    }
  }
  // dispatch(name) fires listeners with a { type, target } event, so
  // transitionend handlers that check `ev.target === el` work.
  dispatch(ev, detail) {
    const event = Object.assign({ type: ev, target: this }, detail);
    for (const fn of (this._listeners[ev] || []).slice()) fn(event);
  }
  // IDL-property reflection so `el.value`, `el.checked`, `el.disabled` behave.
  get value() {
    return this._props.value ?? "";
  }
  set value(v) {
    this._props.value = v;
  }
  get checked() {
    return !!this._props.checked;
  }
  set checked(v) {
    this._props.checked = v;
  }
  get disabled() {
    return !!this._props.disabled;
  }
  set disabled(v) {
    this._props.disabled = v;
  }
  set innerHTML(html) {
    // A <template> parses into its `.content` DocumentFragment, mirroring the
    // real DOM: `template.innerHTML = …` populates `template.content`, and the
    // <template> element itself has no direct children. This is what lets the
    // runtime's parseFragment survive table-context skeletons (<tr>, <td>, …)
    // which a plain element parse would drop.
    const target = this.tag === "template" && this.content ? this.content : this;
    target.childNodes.length = 0;
    for (const n of parseHTML(html, target.ownerDocument, target.tag)) {
      target.appendChild(n);
    }
  }
  get innerHTML() {
    return this.innerHTMLString();
  }
  // querySelector — supports the narrow selector shapes teleport tests use:
  //   "#id", ".class", "tag". Depth-first search over the subtree.
  querySelector(sel) {
    const match = selectorMatcher(sel);
    const walk = (node) => {
      for (const child of node.childNodes) {
        if (child.kind === "element" && match(child)) return child;
        const found = walk(child);
        if (found) return found;
      }
      return null;
    };
    return walk(this);
  }
  // Serialize the subtree back to HTML — used to assert rendered output.
  get outerHTML() {
    if (this.kind === "text") return this.data;
    let s = "<" + this.tag;
    for (const k in this.attributes) s += ` ${k}="${this.attributes[k]}"`;
    s += ">";
    s += this.innerHTMLString();
    if (!VOID.has(this.tag)) s += `</${this.tag}>`;
    return s;
  }
  innerHTMLString() {
    return this.childNodes.map((n) => n.outerHTML).join("");
  }
}

const VOID = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
  "param", "source", "track", "wbr",
]);

// Table-context elements: the HTML parser only produces these inside a valid
// table insertion context. A real browser DROPS a `<tr>`/`<td>`/`<tbody>`/…
// parsed as the innerHTML of a non-table container (e.g. a `<div>`), which is
// the table-context bug the runtime's <template> fix addresses. The shim models
// that drop so a `<div>`-context parse of table content matches the browser and
// the buggy path fails loudly (the fix routes through a <template>, whose
// content fragment is a valid insertion context and keeps these elements).
//
// Maps a table-only tag to the set of parent tags it may legally appear under.
// A `<template>` content fragment (and the top-level `null` document context)
// is treated as permissive — the browser's template content is a valid parse
// context for any table-context element.
const TABLE_CONTEXT = {
  tr: new Set(["tbody", "thead", "tfoot", "table"]),
  td: new Set(["tr"]),
  th: new Set(["tr"]),
  tbody: new Set(["table"]),
  thead: new Set(["table"]),
  tfoot: new Set(["table"]),
  caption: new Set(["table"]),
  colgroup: new Set(["table"]),
  col: new Set(["colgroup", "table"]),
};

// Tiny recursive-descent parser for the skeleton subset. `containerTag` is the
// tag of the element whose innerHTML is being set (undefined/`"template"`/`null`
// = permissive context where any element survives, matching a <template>'s
// content fragment and document-level parsing in this shim).
function parseHTML(html, doc, containerTag) {
  let i = 0;
  const roots = [];
  const stack = [];
  const permissive =
    containerTag === undefined ||
    containerTag === null ||
    containerTag === "template";
  // The current parse context tag: the innermost open element, or (at the top
  // level) `null` when permissive (<template>/document) else the container tag.
  const contextTag = () =>
    stack.length ? stack[stack.length - 1].tag : permissive ? null : containerTag;
  const push = (node) => {
    if (stack.length) stack[stack.length - 1].appendChild(node);
    else roots.push(node);
  };
  while (i < html.length) {
    if (html[i] === "<") {
      if (html[i + 1] === "/") {
        // closing tag
        const end = html.indexOf(">", i);
        stack.pop();
        i = end + 1;
      } else {
        const end = html.indexOf(">", i);
        const raw = html.slice(i + 1, end);
        const { tag, attrs } = parseTag(raw);
        // Drop a table-context element whose parent context can't legally hold
        // it (browser parity). `null` context (permissive top level) allows it.
        const allowed = TABLE_CONTEXT[tag];
        const ctx = contextTag();
        if (allowed && ctx !== null && !allowed.has(ctx)) {
          // Skip this element AND its subtree: advance past the whole element.
          i = skipElement(html, i, tag);
          continue;
        }
        const el = doc.createElement(tag);
        for (const [k, v] of attrs) el.setAttribute(k, v);
        push(el);
        if (!VOID.has(tag)) stack.push(el);
        i = end + 1;
      }
    } else {
      const next = html.indexOf("<", i);
      const end = next === -1 ? html.length : next;
      const text = html.slice(i, end);
      if (text.length) push(doc.createTextNode(decode(text)));
      i = end;
    }
  }
  return roots;
}

// skipElement(html, start, tag) — advance the parse index past a whole element
// (its open tag, subtree, and matching close tag) starting at `start` (the `<`
// of the open tag). Used to drop a mis-nested table-context element and its
// content, approximating the browser dropping table tags parsed out of context.
// Void elements have no close tag. Handles simple nesting of the same tag.
function skipElement(html, start, tag) {
  const openEnd = html.indexOf(">", start);
  let i = openEnd + 1;
  if (VOID.has(tag)) return i;
  let depth = 1;
  const openRe = new RegExp("<" + tag + "(?:[\\s/>])", "i");
  const closeStr = "</" + tag;
  while (i < html.length && depth > 0) {
    const nextClose = html.toLowerCase().indexOf(closeStr, i);
    if (nextClose === -1) return html.length;
    // Count same-tag opens between i and nextClose to balance nesting.
    const between = html.slice(i, nextClose);
    let m;
    const re = new RegExp("<" + tag + "(?:[\\s/>])", "gi");
    while ((m = re.exec(between))) depth++;
    depth--; // consume this close
    i = html.indexOf(">", nextClose) + 1;
  }
  return i;
}

function parseTag(raw) {
  const m = raw.match(/^([a-zA-Z][a-zA-Z0-9-]*)/);
  const tag = m[1];
  const attrs = [];
  const re = /([a-zA-Z_:][a-zA-Z0-9_:.-]*)(?:="([^"]*)")?/g;
  let rest = raw.slice(tag.length);
  let mm;
  while ((mm = re.exec(rest))) {
    if (!mm[1]) continue;
    attrs.push([mm[1], decode(mm[2] ?? "")]);
  }
  return { tag, attrs };
}

// A minimal CSS selector matcher for querySelector: "#id" | ".class" | "tag".
function selectorMatcher(sel) {
  sel = sel.trim();
  if (sel[0] === "#") {
    const id = sel.slice(1);
    return (el) => el.getAttribute("id") === id;
  }
  if (sel[0] === ".") {
    const cls = sel.slice(1);
    return (el) => (el.getAttribute("class") || "").split(/\s+/).includes(cls);
  }
  return (el) => el.tag === sel;
}

function decode(s) {
  return s
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">");
}

export function installDom() {
  const doc = {
    createTextNode(data) {
      return new FakeNode(doc, "text", data);
    },
    createElement(tag) {
      const n = new FakeNode(doc, "element", "");
      n.tag = tag;
      // A <template> owns a `.content` DocumentFragment (kind "fragment"),
      // mirroring the real DOM. Its innerHTML setter populates `.content`, and
      // that fragment is a permissive parse context (table-context elements
      // survive) — see FakeNode.innerHTML and parseHTML. The fragment carries a
      // `null` tag so it reads as the permissive top-level parse context.
      if (tag === "template") {
        const frag = new FakeNode(doc, "fragment", "");
        frag.tag = null;
        n.content = frag;
      }
      return n;
    },
    createDocumentFragment() {
      const frag = new FakeNode(doc, "fragment", "");
      frag.tag = null;
      return frag;
    },
    // A document-level root so teleport targets attached here are reachable via
    // document.querySelector. Tests append their portal container to `body`.
    body: null,
    querySelector(sel) {
      return doc.body ? doc.body.querySelector(sel) : null;
    },
  };
  doc.body = doc.createElement("body");
  globalThis.document = doc;
  return doc;
}
