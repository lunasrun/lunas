# Lunas compiled-output design & runtime contract

This document specifies what the Lunas code generator emits (the compiled `.js`
for a component) and the minimal runtime it targets. It is the design that
follows from the investigation recorded in this project's discussion: every
non-obvious decision here is backed by a cross-hardware micro-benchmark
(Apple Silicon Mac + 3× GCP `Xeon@2.20GHz, 2 vCPU` Debian VMs, Chrome/Blink 149).

The generator's **input** is a `ResolvedComponent` (see `lunas_compiler`): the
numbered reactive variables, each dynamic template part with its dependency
mask, and each handler with its write mask. This document is the **output** side.

---

## 1. Where the speed comes from (two levers)

Lunas is fast at *initial render*, and the speed has two compounding sources:

1. **Static DOM is built by the browser's native parser in bulk** via
   `innerHTML`, with dynamic parts excluded and represented as lightweight
   anchors. The static string is kept as large and contiguous as possible, so a
   mostly-static component is essentially one parse.
2. **Dependencies are resolved at compile time**, so runtime reactivity setup is
   nearly free — just registering precomputed dependency bits. There is no
   runtime dependency-tracking graph, no per-part effect closures, no VDOM.

Updates are not "magically light"; the win is *construction offloaded to the
native parser* + *near-zero reactivity setup*.

---

## 2. Construction strategy (benchmark-locked)

| Decision | Why (measured) |
|---|---|
| **Static build = `innerHTML`**, not `cloneNode` | `clone+append` is **1.1–1.32× slower** than `innerHTML` on every machine tested; the gap widens slightly on slower CPUs. `cloneNode` skips parsing but needs a separate adopt step, and Blink's parse is faster than the copy. |
| **No comments in the static HTML** | A comment node (`<!>` *or* `<!---->`) drops Blink out of its fast-path HTML parser: the whole `innerHTML` becomes **~4–6× slower**. Hardware-independent. |
| **Anchors = runtime text nodes** (`createTextNode`+`insertBefore`), created after the parse | Keeps the static HTML comment-free (fast path) while still marking dynamic insertion points. This is what the original Lunas did, and it is correct. |
| **Element refs = positional navigation** (`firstChild`/`nextSibling`/`childNodes[i]`), not `id`+`getElementById` | Positional nav is **~2× faster** (0.45–0.66×) across all machines, works on a *detached* tree, and needs no id bytes, hash lookups, or `removeAttribute` cleanup. |
| **Static HTML is whitespace-free and comment-free** | Whitespace-free ⇒ stable `childNodes` positions for nav (and smaller output). Comment-free ⇒ fast path. |
| **Build detached → wire → attach once** | Because positional nav needs no document attachment (unlike `getElementById`), all wiring happens off-DOM; the live tree is touched exactly once. |

> Note: the fast-path-parser penalty is a Blink characteristic. Gecko/WebKit
> were **not** measured; re-verify before relying on the comment penalty there.

---

## 3. Component output contract

A component compiles to a factory function. Example — a counter with reactive
text, an event handler, an `:if`, and a `:for` over a deeply-mutated array:

```js
import { component, box, deepBox, refs, on, bind, ifBlock, forBlock } from "lunas";

// Static skeleton: comment-free, whitespace-free. Dynamic seams are NOT here;
// they become runtime anchors. Hoisted once per module.
const HTML = `<button>+1</button><p></p><ul></ul>`;

export default component("div", { class: "counter" }, HTML, (c, props) => {
  // Only mutated bindings are boxed. The compiler picks the box kind per var,
  // and gives each a reactive index (0, 1, …):
  const count   = box(c, 0, props.start ?? 0);  // index 0: reassigned only
  const history = deepBox(c, 1, []);             // index 1: array .push() -> Proxy

  function inc() { count.v++; history.v.push(count.v); }

  // Positional nav to the dynamic elements (detached, no ids).
  const [btn, p, ul] = refs(c.root, [[0], [1], [2]]);

  on(btn, "click", inc);

  // Reactive part: (dep indices it reads, updateFn). Runs on flush only when one
  // of its deps changed.
  bind(c, [0], () => { p.textContent = `count: ${count.v}`; });

  // :if count > 0  — anchor inserted before `ul` at build time.
  ifBlock(c, /*anchor before*/ ul, [0], () => count.v > 0, POSITIVE);

  // :for n of history — anchor inside `ul`.
  forBlock(c, ul, [1], () => history.v, ITEM);
});
```

- **Root**: `document.createElement("div")` + attributes, then
  `root.innerHTML = HTML`. Single-root is the fast common path. A multi-root
  component omits the wrapper and builds a fragment (see §7).
- **`HTML`** is hoisted at module scope so it is defined once and shared by all
  instances (the string, not the DOM — each instance re-parses it, which the
  benchmark shows is cheaper than cloning).

---

## 4. Reactivity model (compile-time dependency dispatch)

This is the evolution of Lunas's existing model (Svelte-4 family), **not**
auto-tracking signals.

- Each reactive variable has an **index** (`ResolvedComponent.reactive_vars[i].index`).
- Each dynamic part knows the **set of variable indices it reads**
  (`dynamics[j].deps`), already expanded transitively through function calls at
  compile time.
- A write **enqueues that variable's dependent update functions** (deduplicated);
  a microtask **flush** runs only the affected ones — not every dynamic part.
- **No runtime dependency discovery.** The graph is fully known at compile time.

### Per-variable box specialization (chosen by the compiler)

| Var classification (from analysis) | Box | Cost |
|---|---|---|
| Reassigned only (`x = …`) | `box` — plain getter/setter that sets the bit | lightest, no Proxy |
| Deeply mutated (`arr.push`, `obj.k = …`) | `deepBox` — Proxy that sets the bit on mutation | Proxy only where needed |
| Shared across components (passed as prop and mutated) | `shared` — sets the dirty bit in *every* dependent component | cross-component without signals |

This resolves the classic weaknesses of pure static wiring (deep mutation,
cross-component flow) **at compile time**, avoiding a runtime hybrid.

### Dispatch representation (no BigInt)

The graph is stored as **adjacency** by default: each reactive variable holds the
list of update functions that read it (the inverse of `dynamics[].deps`). A write
enqueues those functions; `flush` runs the queue, deduplicated by a per-function
flag. This makes `flush` cost **O(affected parts)** — unaffected dynamics are
never touched — and, being plain arrays, it has **no width limit and no overflow
case**. This aligns with Lunas's "many parallel dynamic blocks" target: an update
touches neither unrelated static DOM nor unrelated dynamic parts.

The compiler may specialize small components (≤ 31 reactive vars) to a single
`number` **bitmask** (`dirty |= 1<<i`; run binds where `mask & dirty`) for the
smallest constant factor. If a wide bitmask is ever preferred over adjacency, use
a `Uint32Array` of 32-bit chunks — **never `BigInt`**.

`BigInt` is rejected on two grounds: it is markedly slower than `number`
(heap-allocated, non-primitive) **and** it is the least compatible option (ES2020
/ Safari 14+, no efficient polyfill). Lunas's compatibility floor is set by
`Proxy` (ES2015 / Safari 10+, used by `deepBox` for deep mutation) — the same
floor as Vue 3 / Svelte 5 / Solid. Adjacency, `number`, and `Uint32Array` all
stay at or below that floor; `BigInt` would raise it above the competition.

`handlers[].writes` is used at compile time for validation / dead-code
elimination; at runtime the box setter already enqueues dependents, so the write
mask is not needed live.

---

## 5. Minimal runtime

The entire reactive core, tree-shakeable:

```js
// --- reactive core: adjacency dispatch (default) ---
export function bind(c, deps, fn) {         // deps: reactive indices this update reads
  const s = { fn, q: false }; fn();         // initial run -> correct first paint, no flush
  for (const i of deps) (c.deps[i] ??= []).push(s);
  return s;
}
function markVar(c, i) {                     // reactive var i changed
  const ds = c.deps[i];
  if (ds) for (const s of ds) if (!s.q) { s.q = true; c.queue.push(s); }
  if (!c.pending) { c.pending = true; queueMicrotask(() => flush(c)); }
}
function flush(c) {
  c.pending = false; const q = c.queue; c.queue = [];
  for (const s of q) { s.q = false; s.fn(); }   // only affected parts run
}

export function box(c, i, v) {               // reassign-only var at reactive index i
  return { get v() { return v; }, set v(x) { if (x !== v) { v = x; markVar(c, i); } } };
}
export function deepBox(c, i, v) { /* Proxy wrapping arrays/objects; markVar(c,i) on mutation */ }

// --- DOM ---
export function component(tag, attrs, HTML, setup) {
  return (props) => {
    const root = document.createElement(tag);
    for (const k in attrs) root.setAttribute(k, attrs[k]);
    root.innerHTML = HTML;                 // ★ bulk native parse (detached)
    const c = { root, deps: [], queue: [], pending: false };
    setup(c, props);                       // wire (still detached)
    return root;                           // caller attaches once
  };
}
export const refs = (root, paths) => paths.map(p => p.reduce((n, i) => n.childNodes[i], root));
export const on = (el, ev, fn) => el.addEventListener(ev, fn);

// --- control flow (anchors are runtime text nodes) ---
export function ifBlock(c, before, deps, cond, make) { /* insert/remove make() at a text anchor */ }
export function forBlock(c, into, deps, items, make) { /* keyed list at a text anchor */ }
export function mountChild(c, before, Child, props) { /* Child(props) inserted at a text anchor */ }
```

No signal-tracking stack, no VDOM, no per-node effect objects.

---

## 6. Syntax → output mapping

| `.lunas` | Output |
|---|---|
| `${expr}` (deps ≠ ∅) | `bind(c, deps, () => textNode.data = …)` on a text node |
| `${expr}` (deps = ∅) | assigned once at build, no `bind` |
| `:name="e"` | `bind(c, deps, () => setAttr(el, "name", e))` (or `el.prop =` for known props) |
| `::name="lv"` | above **+** `on(el, "input", () => lv = el.value)` (write-back) |
| `@event="h()"` | `on(el, "event", h)` — box setters notify, so no explicit write mask needed |
| static `class="a ${x}"` | text nodes / attr set; interpolations become `bind`s |
| `:if` / `:elseif` / `:else` | one `ifBlock` chain per cascade, anchored; branch built by its own `innerHTML` when shown |
| `:for="n of items"` | `forBlock`; **initial render = one `innerHTML` of the concatenated items**, updates = keyed diff |
| `<Child :p="e"/>` | `mountChild` at an anchor; `p` passed as a getter so the child can `bind` to it |
| `@input name:type = v` | `props.name ?? v` at the top of `setup` |

---

## 7. Build / mount lifecycle

1. `document.createElement(rootTag)` + set static attributes.
2. `root.innerHTML = HTML` — one native parse of the comment-free, whitespace-free
   static skeleton (all statics, no dynamic content).
3. **Positional nav** to grab refs to dynamic elements and control-flow insertion
   points (`refs(root, paths)`), all off-DOM.
4. **Create anchors** for `:if` / `:for` / child components as text nodes and
   `insertBefore` them at their positions.
5. Wire: `on(...)`, `bind(...)`, `ifBlock/forBlock/mountChild(...)`. `bind`
   performs the initial assignment, so the first paint is correct with no flush.
6. Return `root`; the caller attaches it to the live DOM **once**.

**Multi-root component**: skip the wrapper element — parse the interior into a
throwaway `<div>` (or the mount host), take its child nodes as the roots, and
track the set (or a start/end anchor pair) so the block can be removed/moved as
a unit (§8).

---

## 8. Anchors & control-flow details

- **`:if`**: a text anchor marks the slot. When the condition becomes true, the
  branch is built (its own `innerHTML`) and inserted before the anchor; when
  false, its nodes are removed. The anchor is permanent, so the slot position is
  always known.
- **`:for`**: initial render builds **all items as one `innerHTML` string** (the
  fast path), then wires each item. Updates use a keyed diff (insert / remove /
  move individual items); items are **not** re-`innerHTML`ed wholesale.
- **Multi-root blocks** (a branch/item with several top-level nodes): track the
  node list, or delimit with a start/end anchor pair, so removal/move affects the
  whole group. Single-root blocks use the cheap one-node path (compiler picks).
- **Updates never call `innerHTML`.** Re-parsing would destroy state, listeners,
  focus, and selection. Updates are targeted node mutations + anchored insert/remove.

---

## 9. SSR readiness (deferred, but designed-for)

The static HTML string is exactly what a server would emit. Because anchors are
created at runtime (not embedded), **hydration reuses the CSR wiring path**: skip
`innerHTML` (the server DOM already exists), then run the same positional-nav +
anchor-creation + `bind` steps against the server-rendered subtree. Comment-free
HTML keeps server output small and avoids the parse penalty on the client's
initial document parse too. SSR is a later codegen mode; nothing here blocks it.

---

## 10. Compiler pipeline

```
.lunas → lunas_parser → lunas_script → lunas_compiler (ResolvedComponent) → codegen → this output
                                                            │
      reactive_vars (bits) · dynamics (dep masks) · handlers (write masks)
```

The generator consumes a `ResolvedComponent` and emits the shape above. It does
**not** re-analyze; all dependency resolution is already done.

---

## 11. Open / deferred

- Keyed-diff algorithm for `:for` updates (initial render is settled; reconcile
  strategy is not).
- SSR / hydration codegen mode.
- Cross-engine verification of the comment fast-path penalty (Blink measured;
  Gecko/WebKit not).
- Compile-time flattening of purely-static child components into the parent's
  static string (optional optimization).
