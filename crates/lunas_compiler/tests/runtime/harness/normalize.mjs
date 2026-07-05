// normalize.mjs — deterministic, diff-friendly HTML serialization of a mounted
// Lunas tree, built on the dom-shim's FakeNode structure.
//
// Normalization rules (documented in ../README.md):
//   1. Elements serialize as `<tag attr="v" ...>children</tag>`; void elements
//      (input, br, ...) have no closing tag and no children.
//   2. Attributes are emitted in a STABLE, alphabetical order by name, so the
//      output does not depend on insertion order. `class` is additionally
//      whitespace-collapsed (runs of spaces -> one, trimmed) so `:class` merges
//      are stable.
//   3. Text nodes emit their raw data. Adjacent text nodes (the compiler splits
//      interpolation anchors into multiple text nodes) are concatenated, so the
//      split is invisible in the expected file.
//   4. Insignificant whitespace between the pretty-printed source tags never
//      reaches here — the compiler emits a whitespace-free skeleton — so no
//      collapsing of inter-element whitespace is needed. Text WITHIN an element
//      is preserved verbatim (it is significant).
//   5. Attribute values are HTML-escaped for `"` and `&` only (matching what the
//      shim round-trips), keeping the output valid and stable.

const VOID = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
  "param", "source", "track", "wbr",
]);

function escapeAttr(v) {
  return String(v).replace(/&/g, "&amp;").replace(/"/g, "&quot;");
}

function collapseClass(v) {
  return String(v).trim().replace(/\s+/g, " ");
}

function serializeNode(node) {
  if (node.kind === "text") return node.data;
  const tag = node.tag;
  let s = "<" + tag;
  const names = Object.keys(node.attributes).sort();
  for (const name of names) {
    let v = node.attributes[name];
    if (name === "class") v = collapseClass(v);
    s += ` ${name}="${escapeAttr(v)}"`;
  }
  s += ">";
  if (!VOID.has(tag)) {
    s += serializeChildren(node.childNodes);
    s += `</${tag}>`;
  }
  return s;
}

function serializeChildren(childNodes) {
  // Concatenate adjacent text nodes so the compiler's anchor split is invisible.
  let out = "";
  for (const child of childNodes) out += serializeNode(child);
  return out;
}

// normalizeRoots(roots) — `roots` is either a single mounted node or an array of
// nodes (multi-root fragment). Returns the normalized HTML of all roots joined.
export function normalizeRoots(roots) {
  const list = Array.isArray(roots) ? roots : [roots];
  return list.map(serializeNode).join("");
}

// normalizeInner(node) — the normalized innerHTML of a single element (its
// children only). Used by the `expect(sel).html(...)` assertion.
export function normalizeInner(node) {
  return serializeChildren(node.childNodes);
}
