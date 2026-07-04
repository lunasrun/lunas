// blocks.mjs — control-flow blocks anchored at permanent text nodes.
// See output-design.md §8 and for-diff-design.md.
//
// Every block collects the binds created inside its content into a scope
// (core.mjs beginScope/endScope) and drops that scope when the content is
// removed, so removed content never receives updates and never leaks.

import { bind, unbind, beginScope, endScope, dropScope } from "./core.mjs";
import { createForState, seedForState, reconcile } from "./for_diff.mjs";

const toNodes = (h) => (Array.isArray(h) ? h : [h]);
const firstNode = (h) => (Array.isArray(h) ? h[0] : h);

// ifBlock(c, anchor, deps, cond, make)
// `make()` returns a node (single-root branch) or an array of nodes
// (multi-root branch — the compiler knows which and emits accordingly).
// The branch is inserted before the permanent anchor when cond() becomes
// truthy and removed (with scope teardown) when it becomes falsy.
// Returns a handle with destroy() for whole-block teardown.
export function ifBlock(c, anchor, deps, cond, make) {
  let nodes = null;
  let scope = null;

  const insert = () => {
    scope = beginScope(c);
    let h;
    try {
      h = make();
    } finally {
      endScope(c);
    }
    nodes = toNodes(h);
    const p = anchor.parentNode;
    for (const n of nodes) p.insertBefore(n, anchor);
  };
  const removeAll = () => {
    for (const n of nodes) n.remove();
    nodes = null;
    dropScope(c, scope);
    scope = null;
  };

  const s = bind(c, deps, () => {
    const want = !!cond();
    if (want === (nodes !== null)) return;
    if (want) insert();
    else removeAll();
  });

  return {
    destroy() {
      unbind(c, s);
      if (nodes !== null) removeAll();
    },
  };
}

// forBlock(c, anchor, deps, items, opts)
//   items — closure returning the current array (read lazily at flush time)
//   opts.make(itemData, key)   — build one item; returns node or node array
//   opts.keyOf(itemData, i)    — compiled :key (optional; see design doc §3)
//   opts.patch(handle, data)   — update an existing item's scope (optional)
//   opts.onWarn(msg)           — duplicate-key warning hook (optional)
//   opts.seed                  — { keys, handles, data } from a bulk
//                                innerHTML initial render (optional); when
//                                present the initial reconcile is skipped
//                                because the items are already mounted.
// Updates go through the keyed LIS reconciler; innerHTML is never used here.
// Returns a handle with destroy().
export function forBlock(c, anchor, deps, items, opts) {
  const scopes = new Map(); // handle -> scope

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
    },
  };

  const makeItem = (itemData, key) => {
    const sc = beginScope(c);
    let h;
    try {
      h = opts.make(itemData, key);
    } finally {
      endScope(c);
    }
    scopes.set(h, sc);
    return h;
  };

  const state = createForState();
  const ropts = {
    keyOf: opts.keyOf,
    patchItem: opts.patch,
    onWarn: opts.onWarn,
  };

  let seeded = false;
  if (opts.seed) {
    seedForState(state, opts.seed.keys, opts.seed.handles, opts.seed.data);
    seeded = true;
  }

  const s = bind(c, deps, () => {
    if (seeded) {
      // bulk initial render already mounted the items; skip the first run
      seeded = false;
      return;
    }
    reconcile(state, host, items(), makeItem, ropts);
  });

  return {
    destroy() {
      unbind(c, s);
      reconcile(state, host, [], makeItem, ropts); // removes all + drops scopes
    },
  };
}

// mountChild(c, anchor, childFactory, props) — instantiate a child component
// and insert its root before the anchor. Returns { root, unmount }.
export function mountChild(c, anchor, childFactory, props) {
  const root = childFactory(props);
  anchor.parentNode.insertBefore(root, anchor);
  return {
    root,
    unmount() {
      root.remove();
    },
  };
}
