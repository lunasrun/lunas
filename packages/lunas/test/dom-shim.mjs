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
  dispatch(ev) {
    for (const fn of this._listeners[ev] || []) fn({ type: ev });
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
    this.childNodes.length = 0;
    for (const n of parseHTML(html, this.ownerDocument)) {
      this.appendChild(n);
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

// Tiny recursive-descent parser for the skeleton subset.
function parseHTML(html, doc) {
  let i = 0;
  const roots = [];
  const stack = [];
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
      return n;
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
