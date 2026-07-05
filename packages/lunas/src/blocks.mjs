// blocks.mjs — control-flow blocks anchored at permanent text nodes.
// See output-design.md §8 and for-diff-design.md.
//
// Every block collects the binds created inside its content into a scope
// (core.mjs beginScope/endScope) and drops that scope when the content is
// removed, so removed content never receives updates and never leaks.
//
// Scope homing: a block remembers the scope that was open when it was CREATED
// (its "home" — e.g. the enclosing :for item's scope) and opens its content
// scopes under that home, even when the content is (re)built later from a
// flush where no scope is open. This keeps the scope tree congruent with the
// block tree, so dropping an item's scope recursively drops the binds of any
// nested :if/:for content alive inside it (for-diff-design.md §5, §6).

import {
  bind,
  unbind,
  beginScope,
  endScope,
  dropScope,
  runScope,
  addDisposer,
} from "./core.mjs";
import { createForState, seedForState, reconcile, extractKeys } from "./for_diff.mjs";
import { isLive, runMount, runDestroy, onDestroy } from "./lifecycle.mjs";

const toNodes = (h) => (Array.isArray(h) ? h : [h]);
const firstNode = (h) => (Array.isArray(h) ? h[0] : h);

// Run `fn` inside a fresh scope parented to `home` (not to whatever scope
// happens to be open), restoring the previous open scope afterwards.
// Returns { result, scope }.
function inScopeAt(c, home, fn) {
  const prev = c.scope;
  c.scope = home;
  const scope = beginScope(c);
  let result;
  try {
    result = fn();
  } finally {
    endScope(c);
    c.scope = prev;
  }
  return { result, scope };
}

// ifBlock(c, anchor, deps, cond, make)
// `make()` returns a node (single-root branch) or an array of nodes
// (multi-root branch — the compiler knows which and emits accordingly).
// The branch is inserted before the permanent anchor when cond() becomes
// truthy and removed (with scope teardown) when it becomes falsy.
// Returns a handle with update() (re-evaluate now; used by :for item patching)
// and destroy() for whole-block teardown.
export function ifBlock(c, anchor, deps, cond, make) {
  const home = c.scope;
  let nodes = null;
  let scope = null;

  const insert = () => {
    const r = inScopeAt(c, home, make);
    scope = r.scope;
    nodes = toNodes(r.result);
    const p = anchor.parentNode;
    for (const n of nodes) p.insertBefore(n, anchor);
  };
  const removeAll = () => {
    for (const n of nodes) n.remove();
    nodes = null;
    dropScope(c, scope);
    scope = null;
  };

  const run = () => {
    const want = !!cond();
    if (want === (nodes !== null)) return;
    if (want) insert();
    else removeAll();
  };
  const s = bind(c, deps, run);

  return {
    update: run,
    destroy() {
      unbind(c, s);
      if (nodes !== null) removeAll();
    },
  };
}

// ifChain(c, anchor, deps, which, makes)
// One :if/:elseif/:else cascade at a single permanent anchor. `which()`
// returns the index of the branch that should be shown (compiled from the
// cascade's conditions), or -1 for "no branch" (a cascade without :else whose
// conditions are all false). Exactly one branch is alive at a time; switching
// tears the old branch's scope down and builds the new one via its own make.
// Returns a handle with update() and destroy(), like ifBlock.
export function ifChain(c, anchor, deps, which, makes) {
  const home = c.scope;
  let cur = -1;
  let nodes = null;
  let scope = null;

  const removeAll = () => {
    for (const n of nodes) n.remove();
    nodes = null;
    dropScope(c, scope);
    scope = null;
  };

  const run = () => {
    const idx = which();
    if (idx === cur) return;
    if (nodes !== null) removeAll();
    cur = idx;
    if (idx >= 0) {
      const r = inScopeAt(c, home, makes[idx]);
      scope = r.scope;
      nodes = toNodes(r.result);
      const p = anchor.parentNode;
      for (const n of nodes) p.insertBefore(n, anchor);
    }
  };
  const s = bind(c, deps, run);

  return {
    update: run,
    destroy() {
      unbind(c, s);
      if (nodes !== null) removeAll();
      cur = -1;
    },
  };
}

// forBlock(c, anchor, deps, items, opts)
//   items — closure returning the current array (read lazily at flush time)
//
// Item construction — one of two modes (the compiler picks one per site):
//   opts.make(itemData, key, index)  — build one item; returns node or array.
//   opts.html + opts.wire            — compiled mode (output-design.md §8):
//     opts.html                — the item's static skeleton HTML (one string,
//                                single-root). The INITIAL render concatenates
//                                it N times into ONE bulk innerHTML parse; on
//                                updates a new item parses its own copy.
//     opts.wire(root, d, i)    — wire one item's dynamics against its root
//                                node (binds, listeners, nested blocks). May
//                                return a patch closure `(d, i) => …` that
//                                updates the item's data cell; after it runs,
//                                the item's whole scope is re-run (runScope)
//                                so every item-local bind — including nested
//                                block binds — sees the new data.
//
//   opts.keyOf(itemData, i)    — compiled :key (optional; see design doc §3)
//   opts.patch(handle, d, i)   — extra user patch hook (optional)
//   opts.onWarn(msg)           — duplicate-key warning hook (optional)
//   opts.seed                  — { keys, handles, data } from an external
//                                initial render (optional); when present the
//                                initial reconcile is skipped.
// Updates go through the keyed LIS reconciler; innerHTML is never used on
// update. Returns a handle with update() and destroy().
export function forBlock(c, anchor, deps, items, opts) {
  const home = c.scope;
  const scopes = new Map(); // handle -> scope
  const patches = new Map(); // handle -> patch closure from opts.wire

  const host = {
    insertBefore(h, ref) {
      const p = anchor.parentNode;
      const r = ref === null ? anchor : firstNode(ref);
      for (const n of toNodes(h)) p.insertBefore(n, r);
    },
    remove(h) {
      for (const n of toNodes(h)) n.remove();
      const sc = scopes.get(h);
      if (sc) {
        dropScope(c, sc);
        scopes.delete(h);
      }
      patches.delete(h);
    },
  };

  const compiled = opts.html != null && opts.wire;

  // Build one item in compiled mode: parse its own skeleton copy, wire it.
  const buildOne = (d, i) => {
    const scr = anchor.ownerDocument.createElement("div");
    scr.innerHTML = opts.html;
    const root = scr.childNodes[0];
    const p = opts.wire(root, d, i);
    if (p) patches.set(root, p);
    return root;
  };

  const makeItem = (d, key, i) => {
    const r = inScopeAt(c, home, () =>
      compiled ? buildOne(d, i) : opts.make(d, key, i)
    );
    scopes.set(r.result, r.scope);
    return r.result;
  };

  const state = createForState();
  const ropts = {
    keyOf: opts.keyOf,
    patchItem(h, d, i) {
      const p = patches.get(h);
      if (p) p(d, i);
      const sc = scopes.get(h);
      if (sc) runScope(c, sc);
      if (opts.patch) opts.patch(h, d, i);
    },
    onWarn: opts.onWarn,
  };

  let seeded = false;
  if (opts.seed) {
    seedForState(state, opts.seed.keys, opts.seed.handles, opts.seed.data);
    seeded = true;
  } else if (compiled) {
    // Bulk initial render (design doc §2a): ONE innerHTML parse of every
    // item's skeleton concatenated, then per-item wiring, then seed the
    // reconciler. No later update ever re-runs this.
    const arr = items();
    const n = arr.length;
    if (n > 0) {
      const scr = anchor.ownerDocument.createElement("div");
      let html = "";
      for (let i = 0; i < n; i++) html += opts.html;
      scr.innerHTML = html;
      // Snapshot roots before moving anything (childNodes is live).
      const nodes = new Array(n);
      for (let i = 0; i < n; i++) nodes[i] = scr.childNodes[i];
      const kx = extractKeys(arr, opts.keyOf || ((d) => d), opts.onWarn);
      const p = anchor.parentNode;
      for (let i = 0; i < n; i++) {
        const root = nodes[i];
        const r = inScopeAt(c, home, () => opts.wire(root, arr[i], i));
        if (r.result) patches.set(root, r.result);
        scopes.set(root, r.scope);
        p.insertBefore(root, anchor);
      }
      seedForState(state, kx.keys, nodes, arr);
    }
    seeded = true;
  }

  const run = () => reconcile(state, host, items(), makeItem, ropts);
  const s = bind(c, deps, () => {
    if (seeded) {
      // the initial render already mounted the items; skip the first run
      seeded = false;
      return;
    }
    run();
  });

  return {
    update: run,
    destroy() {
      unbind(c, s);
      reconcile(state, host, [], makeItem, ropts); // removes all + drops scopes
    },
  };
}

// dynamicBlock(c, anchor, deps, factoryOf, props) — dynamic component (`:is`).
// `factoryOf()` returns the current child factory (a `component(...)` result),
// or a falsy value for "render nothing". Whenever the factory identity changes
// (its deps flush), the old child is unmounted and the new one is mounted at
// the same anchor via mountChild, so prop passing and child reactivity keep
// working. `props` is the same shape mountChild takes ({ p: () => e, static });
// it is reused across remounts and re-seeds the fresh child.
//
// Returns a handle: { handle (current mountChild handle or null), update(),
// setProp(name, value) (forwards to the live child), destroy() }.
export function dynamicBlock(c, anchor, deps, factoryOf, props) {
  let cur = undefined; // current factory
  let child = null; // current mountChild handle

  const run = () => {
    const next = factoryOf();
    if (next === cur) return;
    cur = next;
    if (child) {
      child.unmount();
      child = null;
    }
    if (next) child = mountChild(c, anchor, next, props);
  };
  const s = bind(c, deps, run);

  return {
    get handle() {
      return child;
    },
    update: run,
    setProp(name, value) {
      if (child) child.setProp(name, value);
    },
    destroy() {
      unbind(c, s);
      if (child) {
        child.unmount();
        child = null;
      }
    },
  };
}

// teleportBlock(c, anchor, targetOf, build) — teleport/portal.
// `build()` returns the content node or an array of nodes (like an :if branch
// make()). `targetOf()` resolves the mount target: a selector string
// (`document.querySelector(sel)`) or an Element. The content is inserted into
// the target instead of inline at `anchor`; on destroy the nodes are removed.
// A permanent text anchor still marks the inline slot so surrounding layout is
// undisturbed and teardown order stays deterministic.
//
// Content binds are collected in a scope homed at creation, so destroying the
// block tears down every inner bind (no leaks), exactly like ifBlock.
export function teleportBlock(c, anchor, targetOf, build) {
  const home = c.scope;
  const r = inScopeAt(c, home, build);
  const scope = r.scope;
  const nodes = toNodes(r.result);

  const resolveTarget = () => {
    const t = targetOf();
    if (t == null) return null;
    if (typeof t === "string") {
      const doc = anchor.ownerDocument || (typeof document !== "undefined" ? document : null);
      return doc && doc.querySelector ? doc.querySelector(t) : null;
    }
    return t; // an Element
  };

  const target = resolveTarget();
  if (target) for (const n of nodes) target.appendChild(n);

  return {
    nodes,
    destroy() {
      for (const n of nodes) n.remove();
      dropScope(c, scope);
    },
  };
}

// mountChild(c, anchor, childFactory, props) — instantiate a child component
// and insert its root before the anchor (output-design.md §6).
//
// `props` seeds the child once: static props are plain values; reactive props
// are getter functions (`{ p: () => expr }`) invoked once at construction to
// seed the child's reactive prop box. The parent keeps a reactive prop live by
// calling the returned handle's `setProp(name, value)` inside its own bind —
// that writes the child's `_props[name]` box, so the child's own template
// binds react. The two contexts are independent (§6): pushing a prop marks the
// CHILD dirty, a child event marks only the child.
//
// A multi-root child (built by `fragment(...)`) returns an Array of nodes
// carrying `__lunasCtx`; a single-root child returns one node. mountChild
// handles both: it inserts every node of the group before the anchor and
// removes them all on unmount.
//
// Returns a handle: { root, ctx, setProp(name, value), unmount() }.
//
// Lifecycle & DI wiring (additive): the child context is linked to the parent
// `c` via `childCtx.parent` (provide.mjs walks this chain) and registered under
// `c._children` so a parent teardown/mount recurses into it (lifecycle.mjs).
// When the child's insertion point is already live, the child's queued onMount
// callbacks fire immediately; otherwise they stay pending until an ancestor
// `attach()` drains them. `unmount()` fires the child's onDestroy exactly once.
export function mountChild(c, anchor, childFactory, props) {
  const root = childFactory(props);
  const nodes = toNodes(root);
  const p = anchor.parentNode;
  for (const n of nodes) p.insertBefore(n, anchor);
  const childCtx = root && root.__lunasCtx;
  // Guard the `_children` link: only a real context object can carry it. If the
  // caller passes a non-object `c` (a primitive :for item slipping through, a
  // bad handle), we still mount and drive the child — we just skip the parent
  // link rather than throwing "Cannot create property '_children' on number".
  const canLink = childCtx && c != null && (typeof c === "object" || typeof c === "function");
  if (childCtx) {
    if (canLink) {
      childCtx.parent = c;
      (c._children || (c._children = [])).push(childCtx);
    }
    // If the child landed in a live tree, fire its mount hooks now; otherwise a
    // later attach() on an ancestor drains them.
    if (isLive(root)) runMount(childCtx);
  }

  let unmounted = false;
  const unmount = () => {
    if (unmounted) return;
    unmounted = true;
    if (childCtx) {
      runDestroy(childCtx);
      const kids = canLink ? c._children : null;
      if (kids) {
        const k = kids.indexOf(childCtx);
        if (k >= 0) kids.splice(k, 1);
      }
    }
    // Multi-root children remove every node of the group.
    for (const n of nodes) n.remove();
  };

  // Tie the child's teardown to the enclosing control-flow scope (a `:for`/`:if`
  // item), if any: when that item is removed, dropScope runs this and the child
  // is unmounted (onDestroy fires, `_children` link cleared). A top-level mount
  // has no open scope; the caller owns unmount via the returned handle.
  addDisposer(c, unmount);

  return {
    root,
    ctx: childCtx,
    setProp(name, value) {
      const boxes = childCtx && childCtx._props;
      const b = boxes && boxes[name];
      if (b) b.v = value;
    },
    unmount,
  };
}

// slotBlock(childCtx, anchor, factory, fallback, slotPropsOf) — render slot
// content at a `<slot>` anchor inside a CHILD component (output-design.md §6).
//
//   childCtx    — the child component's context (where the `<slot>` lives).
//   anchor      — permanent text anchor marking the slot position in the child.
//   factory     — the parent-provided slot content factory, or undefined/null
//                 when the parent passed no content for this slot. Its shape is
//                 `(slotProps, onCleanup) => nodes` (node or array of nodes):
//                 the PARENT emits it, so it wires against the PARENT's context
//                 and the parent's reactivity drives it. `onCleanup(fn)` lets
//                 the factory register teardown (its parent-scope dropScope) to
//                 run when this child unmounts.
//   fallback    — the child's own fallback factory `() => nodes`, wired in the
//                 CHILD's scope, shown only when `factory` is absent. Optional.
//   slotPropsOf — optional getter returning the scoped-slot props object passed
//                 up to the parent's content (`<slot :item="e"/>`). Called once
//                 at build; reactivity on the props flows through the parent's
//                 own binds inside `factory` when the value is a getter.
//
// Scope ownership (get this right): parent-provided content is wired by the
// parent factory in the PARENT context, so its binds live on the parent and are
// driven by parent state; its teardown is registered via onCleanup and fires on
// the child's onDestroy. Fallback content is the child's own, wired in the child
// context under a scope dropped on the child's destroy. Either way nothing
// leaks and no late write lands after unmount.
export function slotBlock(childCtx, anchor, factory, fallback, slotPropsOf) {
  const slotProps = slotPropsOf ? slotPropsOf() : undefined;
  let nodes = null;

  if (typeof factory === "function") {
    // Parent content: the factory owns its own (parent) scope; it registers
    // teardown through onCleanup, which we tie to the child's destroy.
    const cleanups = [];
    const onCleanup = (fn) => {
      if (typeof fn === "function") cleanups.push(fn);
    };
    const result = factory(slotProps, onCleanup);
    nodes = result == null ? [] : toNodes(result);
    if (cleanups.length) {
      onDestroy(childCtx, () => {
        for (const fn of cleanups) fn();
      });
    }
  } else if (typeof fallback === "function") {
    // Fallback content: the child's own, wired in the child's scope so a child
    // teardown drops it.
    const home = childCtx.scope;
    const r = inScopeAt(childCtx, home, () => fallback(slotProps));
    nodes = r.result == null ? [] : toNodes(r.result);
    onDestroy(childCtx, () => dropScope(childCtx, r.scope));
  }

  if (nodes) {
    const p = anchor.parentNode;
    // Skip null/undefined entries defensively so a factory that returns a
    // sparse group never throws (never-panic).
    for (const n of nodes) if (n != null) p.insertBefore(n, anchor);
  }
  return { nodes: nodes || [] };
}

// slotContent(parentCtx, build) — build the PARENT half of a slot factory
// (output-design.md §6). The parent emits, per slot it fills, a factory of the
// shape `(slotProps, onCleanup) => nodes`; this helper wraps the actual wiring:
//
//   • opens a fresh scope on the PARENT context (homed at the scope open when
//     the parent mounted the child, so nested :for-item slot content tears down
//     with the item), runs `build(slotProps)` to create + wire the content
//     against the parent (its binds register on the parent and react to parent
//     state), and returns the produced nodes;
//   • registers the scope's dropScope through `onCleanup`, so when the child
//     unmounts, the parent-owned binds are unregistered (no leak, no late write).
//
// Emitted usage (parent side):
//   { default: (sp, onCleanup) => slotContent(c, (sp) => { …wire…; return r0.childNodes[0]; }, sp, onCleanup) }
export function slotContent(parentCtx, build, slotProps, onCleanup) {
  const home = parentCtx.scope;
  const r = inScopeAt(parentCtx, home, () => build(slotProps));
  if (typeof onCleanup === "function") {
    onCleanup(() => dropScope(parentCtx, r.scope));
  }
  return r.result;
}
