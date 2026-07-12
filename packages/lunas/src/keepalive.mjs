// keepalive.mjs — component instance caching (c-keepalive).
// See output-design.md §5 (runtime API).
//
// A keep-alive wraps a swappable child slot (e.g. a dynamic `:is` or a routed
// outlet) so switching AWAY from a component DEACTIVATES it — detaches its nodes
// but keeps its context, reactive state, and DOM subtree alive — instead of
// destroying it. Switching BACK ACTIVATES the cached instance: its nodes are
// re-attached with no rebuild, preserving scroll, form state, and reactive vars.
//
// Cache policy: keyed LRU with an optional `max`. When the cache overflows the
// least-recently-used instance is truly EVICTED — that's the only path (besides
// `destroy()`) that fires the instance's onDestroy. Deactivation never destroys.
//
// Lifecycle integration (lifecycle.mjs): activation fires `onActivated`,
// deactivation fires `onDeactivated`, first mount also fires `onMount`, and real
// eviction/teardown fires `onDestroy`.

import { mountChild } from "./blocks.mjs";
import {
  runActivate,
  runDeactivate,
  runDestroy,
  isLive,
  runMount,
} from "./lifecycle.mjs";

// keepAlive(opts) — opts: { max? } (LRU capacity; unbounded when omitted).
// Returns a controller:
//   show(c, anchor, key, factory, props) — ensure the instance for `key` is the
//       one mounted before `anchor`, activating a cached instance or mounting a
//       fresh one; deactivates the previously-shown instance. Returns the
//       instance's mountChild handle (extended with `.key`).
//   destroy() — evict and destroy every cached instance.
export function keepAlive(opts) {
  opts = opts || {};
  const max = opts.max == null ? Infinity : opts.max;

  // key → { handle, nodes, active }. Insertion order in the Map is the LRU
  // order (least-recent first); re-inserting on access moves an entry to the end.
  const cache = new Map();
  let current = null; // the key currently attached, or null.

  // Detach an instance's nodes without destroying it. Snapshots the node list
  // (a component root is one node; multi-root handled by scanning siblings is
  // out of scope — the child root is a single node per mountChild contract).
  const deactivate = (entry) => {
    if (!entry.active) return;
    entry.active = false;
    const root = entry.handle.root;
    // Snapshot so re-attach restores the same node.
    entry.nodes = [root];
    root.remove();
    runDeactivate(entry.handle.ctx);
  };

  // Re-attach a cached instance before `anchor` (no rebuild).
  const activate = (entry, anchor) => {
    if (entry.active) return;
    entry.active = true;
    const p = anchor.parentNode;
    for (const n of entry.nodes) p.insertBefore(n, anchor);
    runActivate(entry.handle.ctx);
  };

  // Truly evict: destroy the instance and drop it from the cache.
  const evict = (key) => {
    const entry = cache.get(key);
    if (!entry) return;
    cache.delete(key);
    if (entry.active && entry.handle.root.parentNode) entry.handle.root.remove();
    runDestroy(entry.handle.ctx);
  };

  // Enforce LRU capacity, never evicting the just-touched `keepKey`.
  const trim = (keepKey) => {
    while (cache.size > max) {
      // First key in iteration order = least recently used.
      let victim = null;
      for (const k of cache.keys()) {
        if (k !== keepKey) {
          victim = k;
          break;
        }
      }
      if (victim == null) break;
      evict(victim);
    }
  };

  return {
    show(c, anchor, key, factory, props) {
      // Deactivate the outgoing instance (if switching keys).
      if (current !== null && current !== key) {
        const prev = cache.get(current);
        if (prev) deactivate(prev);
      }

      let entry = cache.get(key);
      if (entry) {
        // Cache hit: move to MRU position, re-attach if needed.
        cache.delete(key);
        cache.set(key, entry);
        activate(entry, anchor);
      } else {
        // Miss: mount a fresh instance. mountChild links parent/child, inserts
        // the root, and fires onMount when live.
        const handle = mountChild(c, anchor, factory, props);
        entry = { handle, nodes: [handle.root], active: true };
        cache.set(key, entry);
        // A fresh instance is considered "activated" on first show too.
        runActivate(handle.ctx);
        // If mountChild couldn't fire onMount (detached at mount time) but the
        // slot is now live, this is still deferred to an ancestor attach().
        if (isLive(handle.root)) runMount(handle.ctx);
        entry.handle.key = key;
      }
      current = key;
      trim(key);
      return entry.handle;
    },

    // has(key) — whether an instance is currently cached (test/introspection).
    has(key) {
      return cache.has(key);
    },
    // size — number of cached instances.
    get size() {
      return cache.size;
    },

    destroy() {
      for (const key of Array.from(cache.keys())) evict(key);
      current = null;
    },
  };
}
