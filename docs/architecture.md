# Architecture Overview

This page explains how Lunas works end to end: how a `.lunas` file becomes a
JavaScript module, how the runtime builds and updates the DOM, and the
benchmark-locked decisions that shape both. It is a reader-friendly overview — for
the exact contract, see the design docs linked at the bottom.

Lunas has two halves:

- a **compiler** written in Rust (a Cargo workspace under `crates/`), and
- a **tiny, dependency-free JavaScript runtime** (`packages/lunas`, ES2015,
  no build step, no `BigInt`).

The compiler does all the analysis; the runtime does almost no thinking at
runtime. That split is the source of Lunas's speed.

---

## The compile pipeline

A `.lunas` single-file component bundles an `html:` template, a `style:` block, and
a `script:` block (TypeScript or JavaScript). It flows through the crates like
this:

```
.lunas
  → lunas_parser        (.lunas syntax: split blocks/directives, build the
                         ParsedFile + a binding-aware template IR)
  → lunas_script        (JS/TS analysis via SWC: bindings, per-function mutation
                         sets, free identifiers / dependency sets, TS→JS)
  → lunas_compiler      (resolution: numbered reactive variables, each dynamic
                         template part with its dependency set, each handler with
                         its write set → a ResolvedComponent)
  → codegen             (emit the JS module described below)
  → JS module           (a component(...) / fragment(...) factory + runtime calls)
```

Key ideas along the way:

- **Lossless, span-everywhere parsing.** Every node carries a file-absolute byte
  range; line/column is derived on demand. This powers the language server
  (go-to-definition, find-references) as well as diagnostics.
- **Strict layering.** The `.lunas` syntax parser carries no JS/TS toolchain
  dependency; all JS/TS work is isolated in `lunas_script`. Every crate builds for
  `wasm32-unknown-unknown`, so the front end can run in the browser.
- **Error recovery over hard failure.** The parser always returns a tree and
  reports problems as diagnostics; public entry points never panic.
- **Resolution, not codegen, computes the dependency graph.** `lunas_compiler`
  produces a `ResolvedComponent`: each reactive variable gets an **index**, each
  dynamic template part gets the **set of variable indices it reads** (already
  expanded transitively through function calls), and each handler gets its **write
  set**. The generator consumes this and does **not** re-analyze.

> **Status:** the parser front end, the JS/TS analysis suite, and the resolution
> layer are implemented and well-tested; code generation is the phase being built
> out. The runtime that codegen targets already exists in `packages/lunas`.

---

## What the compiler emits

A single-root component compiles to a `component(tag, attrs, HTML, setup)` factory
(a multi-root template compiles to `fragment(...)` — no wrapper element). For a
counter:

```js
import { component, box, deepBox, refs, on, bind, ifBlock, forBlock } from "lunas";

// Static skeleton: comment-free, whitespace-free. Dynamic seams are NOT here —
// they become runtime anchors. Hoisted once per module.
const HTML = `<button>+1</button><p></p><ul></ul>`;

export default component("div", { class: "counter" }, HTML, (c, props) => {
  const count   = box(c, 0, props.start ?? 0);  // index 0: reassigned only
  const history = deepBox(c, 1, []);            // index 1: deeply mutated

  const [btn, p, ul] = refs(c.root, [[0], [1], [2]]);  // positional nav
  on(btn, "click", () => { count.v++; history.v.push(count.v); });

  bind(c, [0], () => { p.textContent = `count: ${count.v}`; });   // reactive text
  ifBlock(c, ul, [0], () => count.v > 0, POSITIVE);               // :if
  forBlock(c, ul, [1], () => history.v, ITEM);                    // :for
});
```

The [syntax → output mapping](#pointers-to-the-design-docs) table in the design doc
lists every directive's emitted shape (`${expr}`, `:attr`, `::model`, `@event`,
`:if`/`:for`, child components, slots, `:class`/`:style`, `:html`, `:ref`,
`<component :is>`, `<teleport>`).

---

## The runtime model

The runtime is deliberately small — the whole reactive core fits in a few
functions. Its shape follows from a handful of decisions.

### 1. Static DOM is built in bulk by `innerHTML`

The static skeleton is kept as one large, contiguous, **comment-free,
whitespace-free** string and parsed in a single `root.innerHTML = HTML` off-DOM.
A mostly-static component is essentially one native parse. This beats
`cloneNode`-based construction on every machine measured (see below).

### 2. Runtime text anchors mark the dynamic seams

Dynamic insertion points (`:if`, `:for`, child components, text interpolations
inside a run) are **not** in the static HTML. Instead, empty text nodes are
created at wiring time and `insertBefore`d at their positions
(`anchorBefore` / `anchorBeforeSplit` / `anchorAppend`). This keeps the static
HTML on the browser's fast-path parser (comments would knock it off) while still
marking every place content can appear or disappear.

### 3. Positional refs, not ids

Dynamic elements are reached by **positional navigation** — walking
`childNodes[i]` from the root (`refs(root, paths)`) — not `id` + `getElementById`.
Positional nav is ~2× faster, works on a **detached** tree (so all wiring happens
before the tree is attached), and needs no id bytes or cleanup.

### 4. Build detached → wire → attach once

`document.createElement` → `innerHTML` → positional refs → create anchors → wire
(`on`, `bind`, blocks) → return the detached root. The caller touches the live DOM
exactly **once** via [`attach(root, host)`](./api/lifecycle.md#attach), which also
fires the subtree's `onMount` hooks.

### 5. Compile-time dependency dispatch (adjacency), no VDOM

This is the reactive heart. Each reactive variable has an index; each dynamic part
knows the exact set of indices it reads (its `deps`), resolved at compile time. The
graph is stored as **adjacency**: each variable holds the list of update functions
(`bind` records) that read it — the inverse of the parts' dependency sets.

- A **write** enqueues that variable's dependent update functions, deduplicated by
  a per-function flag.
- A microtask **flush** runs the queue — only the affected parts, never every
  dynamic part. Cost is O(affected).
- There is **no runtime dependency discovery**, no per-node effect objects, no
  virtual DOM diff. Updates are targeted node mutations and anchored
  insert/remove; `innerHTML` is never used on update (re-parsing would destroy
  state, listeners, focus, and selection).

Adjacency is plain arrays, so it has **no width limit and no overflow case**. The
compiler may specialize small components (≤ 31 reactive vars) to a single `number`
bitmask for the smallest constant factor. `BigInt` is rejected outright: it is
slower than `number` **and** raises the compatibility floor above the competition.
The reactive core is plain ES2015 getters/setters — no `Proxy` anywhere — so the
floor is *below* Vue 3 / Svelte 5 / Solid, which all require `Proxy` for their
reactivity.

### 6. Per-variable box specialization

The compiler classifies each reactive variable from its analysis and picks the
lightest reactive cell:

| Classification | Box | Cost |
| --- | --- | --- |
| Reassigned only (`x = …`) | [`box`](./api/reactivity.md#box) — plain getter/setter | lightest, no Proxy |
| Deeply mutated (`arr.push`, `obj.k = …`) | [`deepBox`](./api/reactivity.md#deepbox) — raw value; the compiler injects a `touch()`/`touchElem()` call after the mutation to set the bit | no Proxy; a deep mutation costs one bit-set |
| Shared across components (prop passed down + mutated) | [`shared`](./api/reactivity.md#shared) — marks every dependent component | cross-component, no signals |

Module-level state generalizes `shared` into a [store](./api/store.md): created
once and imported by many components, same adjacency contract.

### 7. Control-flow blocks and scopes

Each block ([`ifBlock`](./api/blocks-and-control-flow.md#ifblock),
[`ifChain`](./api/blocks-and-control-flow.md#ifchain),
[`forBlock`](./api/blocks-and-control-flow.md#forblock), and the child/slot/teleport
helpers) is anchored at a permanent text node and collects the binds created
inside its content into a **scope**. When the content is removed, the scope is
dropped — every inner bind is unregistered recursively, so removed content never
receives updates and never leaks. Scope homing keeps the scope tree congruent with
the block tree, so dropping a `:for` item's scope also drops the binds of any
nested `:if`/`:for` alive inside it.

`:for` is special: the **initial render** builds all items as one concatenated
`innerHTML` parse (the fast path), then wires each item. **Updates** run a keyed
reconciler — prefix/suffix trimming, a key→index map, and a
longest-increasing-subsequence pass to minimize node moves. `items` is read lazily
at flush time, so one flush sees the final state of any number of synchronous
mutations.

---

## Benchmark-locked decisions

Every non-obvious construction decision is backed by a cross-hardware
micro-benchmark (Apple Silicon Mac + three GCP Xeon VMs, Chrome/Blink 149). The
highlights (design doc §2 and §4):

| Decision | Why (measured) |
| --- | --- |
| Static build = `innerHTML`, not `cloneNode` | `clone+append` is **1.1–1.32× slower** on every machine; the browser's parse beats the copy. |
| No comments in the static HTML | A single comment node drops Blink out of its fast-path parser — the whole `innerHTML` becomes **~4–6× slower**. |
| Anchors = runtime text nodes | Keeps the static HTML comment-free (fast path) while still marking dynamic insertion points. |
| Element refs = positional navigation | ~2× faster than `getElementById`, works on a detached tree, no id bytes or cleanup. |
| Whitespace-free static HTML | Stable `childNodes` positions for nav, plus smaller output. |
| Adjacency dispatch, `number`/`Uint32Array`, never `BigInt` | O(affected) flush, no width limit; the reactive core stays plain ES2015 — no `Proxy`. |

> The fast-path-parser penalty is a Blink characteristic; Gecko/WebKit were not
> measured, so re-verify the comment penalty there before relying on it.

**SSR is designed-for but deferred.** The static HTML string is exactly what a
server would emit, and because anchors are created at runtime (not embedded),
hydration can reuse the client wiring path — skip `innerHTML`, then run the same
positional-nav + anchor-creation + `bind` steps against the server-rendered
subtree. The SSR codegen mode is a later phase; nothing in the current design
blocks it.

---

## Pointers to the design docs

- **`crates/lunas_compiler/docs/output-design.md`** — the authoritative
  compiled-output shape and runtime contract. §1–2 (where the speed comes from,
  construction strategy), §4 (the reactivity model and box specialization), §5 (the
  minimal runtime, every export sketched), §6 (the full syntax → output mapping),
  §7–8 (build/mount lifecycle and control-flow details), §9 (SSR readiness).
- **`crates/lunas_compiler/docs/for-diff-design.md`** — the keyed `:for`
  reconciliation (update-path) algorithm.
- **`crates/lunas_parser/DESIGN.md`** and
  **`crates/lunas_parser/docs/template-design.md`** — the span model, layering, the
  parser-vs-AST-parser split, and the template binding layer.
- **`packages/lunas/README.md`** — the runtime package's own export table.
- The [API reference](./README.md#api-reference) documents every runtime symbol
  per-signature.
