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
import { parseFragment } from "./dom.mjs";

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
    const scr = parseFragment(opts.html, anchor.ownerDocument);
    const root = scr.childNodes[0];
    const p = opts.wire(root, d, i);
    if (p) patches.set(root, p);
    return root;
  };

  // Build one item in mount mode (`:for` directly on a component tag): no static
  // item HTML — `opts.mount(d, key, i)` mounts the child and returns
  // `{ node, patch }`, where `node` is the child root (single node or multi-root
  // group) used as the reconciler handle, and `patch(d, i)` updates the item's
  // data cell so a re-run drives the child's props with the new item value.
  const mountOne = (d, key, i) => {
    const r = opts.mount(d, key, i);
    const node = r && r.node !== undefined ? r.node : r;
    if (r && r.patch) patches.set(node, r.patch);
    return node;
  };

  const makeItem = (d, key, i) => {
    const r = inScopeAt(c, home, () =>
      compiled ? buildOne(d, i) : opts.mount ? mountOne(d, key, i) : opts.make(d, key, i)
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

  // Fine-grained item-field updates (output-design.md §8, for-diff-design.md §6):
  // when the `:for` source is a single deepBox (opts.box), opting into element
  // tracking lets us tell a pure field mutation (`rows[i].label = x`) apart from
  // a structural change. On a field-only flush we patch just the touched items —
  // no extractKeys / LIS / whole-list patch — falling back to a full reconcile
  // whenever anything structural changed (reassign / length / reorder). This is
  // the only behavioral change; correctness is preserved by always reconciling
  // on structural change. Element tracking must be enabled BEFORE the initial
  // `items()` read so `state.data` already holds trackable element proxies.
  const fineBox = opts.box && typeof opts.box.observeElems === "function" ? opts.box : null;
  let rawToHandle = null; // raw element -> handle, rebuilt after each reconcile
  if (fineBox) {
    fineBox.observeElems();
    rawToHandle = new Map();
  }
  // In fine mode the reconciler iterates the RAW array (no per-element proxy
  // reads on the hot path); field-write detection still fires when user code
  // mutates through `box.v`. Outside fine mode, use the compiled `items()`.
  const readItems = fineBox ? () => fineBox._raw() : items;

  // bulkRender(arr) — mount `arr` from an EMPTY list via ONE innerHTML parse of
  // every item's skeleton concatenated (design doc §2a), then per-item wiring,
  // then seed the reconciler. This is only valid when the current mounted list
  // is empty: it appends before the anchor and OVERWRITES the reconciler state,
  // which is exactly the reconciler's own empty->N branch — minus N separate
  // scratch-div innerHTML parses (one parse for the whole list instead of N).
  // Used both for the initial render and as a `run()` fast path whenever the
  // list transitions empty -> non-empty (create / clear-then-fill). Assumes
  // `compiled` (opts.html + opts.wire); callers guard on it.
  const bulkRender = (arr) => {
    const n = arr.length;
    if (n === 0) {
      seedForState(state, [], [], arr);
      return;
    }
    let html = "";
    for (let i = 0; i < n; i++) html += opts.html;
    const scr = parseFragment(html, anchor.ownerDocument);
    // Snapshot roots before moving anything (childNodes is live).
    const nodes = new Array(n);
    for (let i = 0; i < n; i++) nodes[i] = scr.childNodes[i];
    const kx = extractKeys(arr, opts.keyOf || ((d) => d), opts.onWarn);
    const p = anchor.parentNode;
    // Inline inScopeAt's scope bracketing so the hot per-item loop doesn't
    // allocate a fresh `() => opts.wire(...)` closure per row (N closures for an
    // N-item create). Save/restore the open scope once around the whole loop and
    // call opts.wire directly; each item still gets its own scope homed at `home`
    // (identical teardown semantics to inScopeAt).
    const wire = opts.wire;
    const prevScope = c.scope;
    for (let i = 0; i < n; i++) {
      const root = nodes[i];
      c.scope = home;
      const scope = beginScope(c);
      let patch;
      try {
        patch = wire(root, arr[i], i);
      } finally {
        endScope(c);
      }
      if (patch) patches.set(root, patch);
      scopes.set(root, scope);
      p.insertBefore(root, anchor);
    }
    c.scope = prevScope;
    seedForState(state, kx.keys, nodes, arr);
  };

  let seeded = false;
  if (opts.seed) {
    seedForState(state, opts.seed.keys, opts.seed.handles, opts.seed.data);
    seeded = true;
  } else if (compiled) {
    // Bulk initial render. No later update ever re-runs this.
    bulkRender(readItems());
    seeded = true;
  }

  // On a flush, an empty->non-empty transition can skip the host-abstracted
  // reconciler (which mounts N items via N separate scratch-div innerHTML
  // parses in its all-new branch) and use the single-parse bulk path instead.
  // Only valid when the current list is empty AND in compiled mode; otherwise
  // fall back to the reconciler. Correctness: bulkRender seeds exactly the state
  // the reconciler's empty->N branch would.
  const run = () => {
    const arr = readItems();
    if (compiled && state.keys.length === 0 && arr.length > 0) {
      bulkRender(arr);
      return;
    }
    reconcile(state, host, arr, makeItem, ropts);
  };

  // Seed the raw->handle map from the initial render's state (if any). In fine
  // mode state.data already holds raw elements (readItems yields raw).
  if (fineBox) {
    for (let k = 0; k < state.data.length; k++) {
      rawToHandle.set(state.data[k], state.nodes[k]);
    }
  }

  // When the initial render already ran (seeded), incremental tracking is live
  // from the start; otherwise the first fire renders via a full reconcile.
  let fineInited = seeded;
  const fineRun = () => {
    // First fire without a seeded initial render (e.g. make-mode): render via a
    // full reconcile so the list actually mounts, then track incrementally.
    if (!fineInited) {
      fineInited = true;
      fineBox._clear();
      run();
      rebuildRawMap();
      return;
    }
    // A structural change (or a datum with no live handle) forces a full
    // reconcile; that rebuilds rawToHandle from the fresh state below.
    if (fineBox._struct) {
      fineBox._clear();
      run();
      rebuildRawMap();
      return;
    }
    // Snapshot the dirty elements before clearing (`_clear` empties the live
    // Set, which is the same object).
    const dirty = fineBox._elems && fineBox._elems.size ? Array.from(fineBox._elems) : null;
    fineBox._clear();
    if (!dirty) return;
    let missed = false;
    // Patch only the touched items. Index is looked up from state so patchItem
    // gets the right position (patchItem re-runs the item scope with new data).
    const idxOf = indexMap();
    for (let k = 0; k < dirty.length; k++) {
      const h = rawToHandle.get(dirty[k]);
      const pos = h !== undefined ? idxOf.get(h) : undefined;
      if (h === undefined || pos === undefined) {
        missed = true;
        continue;
      }
      ropts.patchItem(h, state.data[pos], pos);
    }
    if (missed) {
      // A field write landed on an element we don't track (shouldn't happen for
      // in-place edits of mounted items, but never miss an update): reconcile.
      run();
      rebuildRawMap();
    }
  };

  function indexMap() {
    const m = new Map();
    const nodes = state.nodes;
    for (let k = 0; k < nodes.length; k++) m.set(nodes[k], k);
    return m;
  }
  function rebuildRawMap() {
    rawToHandle.clear();
    const data = state.data; // raw elements in fine mode
    const nodes = state.nodes;
    for (let k = 0; k < data.length; k++) rawToHandle.set(data[k], nodes[k]);
  }

  const s = bind(c, deps, () => {
    if (seeded) {
      // the initial render already mounted the items; skip the first run
      seeded = false;
      return;
    }
    if (fineBox) fineRun();
    else run();
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
//
// The teleported nodes live under an external target, NOT under `anchor`'s
// parent — so a plain :for/:if item removal (which only walks its own subtree
// and drops its own scope) never touches them, and neither does an owning
// component's own unmount (mountChild's unmount() only fires onDestroy
// callbacks + removes the component's OWN root's nodes — it never sees the
// teleport's target-side nodes). Both would leak the teleported content
// forever without explicit teardown wiring. So destroy() is registered two
// ways, mirroring mountChild's own dual wiring (addDisposer + onDestroy):
//   - addDisposer(c, destroy) — runs when the enclosing :if/:for item's scope
//     is dropped (the teleport call site sits inside that item's content).
//   - onDestroy(c, destroy) — runs when the OWNING component's context itself
//     is torn down (mountChild(...).unmount() -> runDestroy(childCtx)), which
//     covers a top-level teleport call (no scope open, addDisposer is a
//     no-op) whose owning component unmounts directly.
// destroy() guards against running twice since both paths can fire (e.g. a
// component with a top-level teleport nested inside a removed :for item hits
// dropScope's disposer first; onDestroy would otherwise double-fire when the
// child context's own destroy also runs as part of the same teardown).
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

  let destroyed = false;
  const destroy = () => {
    if (destroyed) return;
    destroyed = true;
    for (const n of nodes) n.remove();
    dropScope(c, scope);
  };

  addDisposer(c, destroy);
  onDestroy(c, destroy);

  return {
    nodes,
    destroy,
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
