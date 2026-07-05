// lifecycle.mjs — component lifecycle hooks (c-lifecycle).
// See output-design.md §5 (runtime API) and §7 (mount lifecycle).
//
// Four hooks, all keyed on the component context `c`:
//
//   onMount(c, fn)     — after the component's root is attached to a live tree.
//   onDestroy(c, fn)   — when the component's scope is torn down (unmount).
//   onUpdate(c, fn)    — after each flush of `c` that actually ran updates.
//   onActivated(c, fn) / onDeactivated(c, fn) — keep-alive (keepalive.mjs) only.
//
// ── The attach contract (§7) ────────────────────────────────────────────────
// `component()` returns a DETACHED root; the CALLER attaches it. So onMount
// callbacks cannot fire at construction — they are queued on the context
// (`c._mountQ`) and drained the moment the root becomes part of a live tree:
//
//   • `attach(root, host)` — the top-level mount helper: appends `root` to
//     `host`, then drains the queue (recursively, so nested children mounted
//     during setup also fire). Use this to mount a root component.
//   • `mountChild` (blocks.mjs) — when it inserts a child whose insertion point
//     is already connected, it drains the child's queue immediately; otherwise
//     the child stays pending until an ancestor `attach` drains it.
//
// Liveness is detected via `Node.isConnected` with a walk-to-root fallback for
// DOM shims that don't implement it (the test fake DOM has no `isConnected`).
//
// Destroy runs exactly once per context: `runDestroy(c)` is idempotent (it
// nulls the queue after firing). Every unmount path funnels through it —
// mountChild.unmount, forBlock/ifBlock item teardown that owns a child, and
// keep-alive final eviction.

// ── queue accessors (kept off `core.mjs` so the core stays minimal) ──────────
// Each queue is a plain array lazily created on the context under a private
// field. onUpdate additionally installs a post-flush hook via a wrapper the
// core's flush loop already drains (`c.post`); to fire *every* flush (not just
// once) we re-arm it from inside itself.

/** onMount(c, fn) — run `fn` after this component's root attaches to the DOM.
 *  If the root is already connected (mounted synchronously into a live tree),
 *  `fn` runs on the next microtask so ordering matches the deferred case. */
export function onMount(c, fn) {
  if (typeof fn !== "function") return;
  if (c._mounted) {
    // Already attached — schedule on a microtask so callers registering after
    // attach still observe a live, painted tree.
    queueMicrotask(fn);
    return;
  }
  (c._mountQ || (c._mountQ = [])).push(fn);
}

/** onDestroy(c, fn) — run `fn` when this component is torn down (once). */
export function onDestroy(c, fn) {
  if (typeof fn !== "function") return;
  (c._destroyQ || (c._destroyQ = [])).push(fn);
}

/** onUpdate(c, fn) — run `fn` after each flush of `c` that ran updates.
 *  Wires the core's per-context `onUpdate` post-update hook (core.mjs flush). */
export function onUpdate(c, fn) {
  if (typeof fn !== "function") return;
  (c._updateQ || (c._updateQ = [])).push(fn);
  if (!c.onUpdate) c.onUpdate = () => fireUpdate(c);
}

/** onActivated(c, fn) — keep-alive: run `fn` each time the component is
 *  (re)activated from the cache. Fires immediately on first activation. */
export function onActivated(c, fn) {
  if (typeof fn !== "function") return;
  (c._activateQ || (c._activateQ = [])).push(fn);
}

/** onDeactivated(c, fn) — keep-alive: run `fn` each time the component is
 *  deactivated (cached, not destroyed). */
export function onDeactivated(c, fn) {
  if (typeof fn !== "function") return;
  (c._deactivateQ || (c._deactivateQ = [])).push(fn);
}

// ── liveness ────────────────────────────────────────────────────────────────
// Prefer the real `isConnected`; fall back to walking parentNode to a node
// whose ownerDocument is (or contains) it. The fake DOM has neither a document
// element nor isConnected, so we treat "has a parent chain terminating in a
// node with no parent that is not the scratch owner" — in practice the runtime
// only ever calls this after inserting into the mount host, and tests drive
// attach() explicitly, so a null-safe walk is enough.
export function isLive(node) {
  if (!node) return false;
  if (typeof node.isConnected === "boolean") return node.isConnected;
  // Shim fallback: connected iff a marked root ancestor is reachable.
  let n = node;
  while (n) {
    if (n._lunasAttached) return true;
    n = n.parentNode;
  }
  return false;
}

// ── draining ─────────────────────────────────────────────────────────────────
// runMount(c) — mark the context mounted and fire+clear its queued onMount
// callbacks. Idempotent: a second call is a no-op. Recurses into child
// contexts registered on `c._children` (mountChild links them) so a single
// top-level attach fires the whole freshly-built subtree in child-before-... no:
// parent registers children as it mounts them, and children are attached (their
// nodes inserted) before the parent finishes, so we fire children first to match
// "mounted" meaning "my subtree is in the DOM".
export function runMount(c) {
  if (!c || c._mounted) return;
  c._mounted = true;
  const kids = c._children;
  if (kids) for (const k of kids) runMount(k);
  const q = c._mountQ;
  if (q) {
    c._mountQ = null;
    for (const fn of q) fn();
  }
}

// runDestroy(c) — fire+clear queued onDestroy callbacks exactly once, then
// recurse into child contexts so a parent teardown tears children down too.
export function runDestroy(c) {
  if (!c || c._destroyed) return;
  c._destroyed = true;
  const kids = c._children;
  if (kids) for (const k of kids) runDestroy(k);
  const q = c._destroyQ;
  if (q) {
    c._destroyQ = null;
    for (const fn of q) fn();
  }
}

// runActivate(c) / runDeactivate(c) — keep-alive activation hooks. Not
// idempotent by design: a keep-alive component activates/deactivates many times.
export function runActivate(c) {
  if (!c) return;
  const q = c._activateQ;
  if (q) for (const fn of q.slice()) fn();
}
export function runDeactivate(c) {
  if (!c) return;
  const q = c._deactivateQ;
  if (q) for (const fn of q.slice()) fn();
}

// fireUpdate(c) — run queued onUpdate callbacks. Called by the flush post-hook
// installed on demand (see armUpdate). Kept separate so keep-alive/tests can
// invoke it deterministically.
export function fireUpdate(c) {
  const q = c._updateQ;
  if (q) for (const fn of q.slice()) fn();
}

// ── attach — the top-level mount entry (§7) ──────────────────────────────────
// attach(root, host) — append a detached component root to a live host and fire
// the whole subtree's onMount callbacks. `root.__lunasCtx` is the component's
// context (set by component()). Returns `root`.
export function attach(root, host) {
  host.appendChild(root);
  root._lunasAttached = true; // liveness marker for the shim fallback
  const c = root && root.__lunasCtx;
  if (c) runMount(c);
  return root;
}
