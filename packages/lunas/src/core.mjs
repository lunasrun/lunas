// core.mjs — Lunas reactive core: compile-time adjacency dispatch.
// See crates/lunas_compiler/docs/output-design.md §4–5.
//
// Plain ESM, ES2015+ (Proxy floor is set elsewhere, in boxes.mjs). No BigInt.
//
// A component context `c` is:
//   { root, deps: [], queue: [], pending: false, scope: null }
// where deps[i] is the adjacency list of bind records that read reactive
// variable i. `scope` is the currently-open collection scope used by control
// flow blocks so their inner binds can be unregistered on teardown (see
// for-diff-design.md §6).

export function createContext(root) {
  // `parent` links a child component context to the one that mounted it
  // (additive; set by mountChild) so provide/inject (provide.mjs) can walk the
  // chain. `onUpdate` (lifecycle.mjs) is an optional per-context hook the flush
  // loop invokes after an update pass actually ran. All new fields default to
  // null so contexts stay cheap and existing paths are untouched.
  return {
    root,
    deps: [],
    queue: [],
    pending: false,
    scope: null,
    post: null,
    parent: null,
    onUpdate: null,
  };
}

// bind(c, deps, fn) — register an update function that reads the reactive
// variable indices in `deps`. Runs fn once immediately (correct first paint,
// no flush needed). Returns the bind record (needed for unbind).
export function bind(c, deps, fn) {
  const s = { fn, q: false, alive: true, deps };
  fn();
  for (const i of deps) (c.deps[i] || (c.deps[i] = [])).push(s);
  if (c.scope) c.scope.subs.push(s);
  return s;
}

// markVar(c, i) — reactive variable i changed: enqueue its dependents
// (deduplicated via the per-record q flag) and schedule a microtask flush.
export function markVar(c, i) {
  const ds = c.deps[i];
  if (ds) {
    for (const s of ds) {
      if (s.alive && !s.q) {
        s.q = true;
        c.queue.push(s);
      }
    }
  }
  if (!c.pending) {
    c.pending = true;
    queueMicrotask(() => flush(c));
  }
}

// flush(c) — run every queued update once. Only affected parts run.
// After the update pass, drain any post-flush callbacks registered via
// `afterFlush` (used by nextTick): they observe the DOM already updated.
export function flush(c) {
  c.pending = false;
  const q = c.queue;
  c.queue = [];
  const ran = q.length > 0;
  for (const s of q) {
    s.q = false;
    if (s.alive) s.fn();
  }
  // onUpdate (lifecycle.mjs) — a lightweight per-context post-update hook that
  // fires only when an update pass actually ran, distinct from the one-shot
  // `post` callbacks (afterFlush/nextTick) drained below. Additive: null unless
  // a component registered onUpdate.
  if (ran && c.onUpdate) c.onUpdate();
  const post = c.post;
  if (post) {
    c.post = null;
    for (const cb of post) cb();
  }
}

// afterFlush(c, cb) — run `cb` once, after the next flush completes (i.e. after
// the DOM update pass). If a flush is already pending the callback rides that
// one; otherwise a flush is scheduled so the callback still fires this tick.
// This is the primitive behind nextTick(); it keeps flush the single place that
// knows when updates have landed.
export function afterFlush(c, cb) {
  (c.post || (c.post = [])).push(cb);
  if (!c.pending) {
    c.pending = true;
    queueMicrotask(() => flush(c));
  }
}

// unbind(c, s) — permanently unregister a bind record. Safe to call while a
// flush containing `s` is pending (the dead record is skipped at flush).
export function unbind(c, s) {
  s.alive = false;
  for (const i of s.deps) {
    const ds = c.deps[i];
    if (ds) {
      const k = ds.indexOf(s);
      if (k >= 0) ds.splice(k, 1);
    }
  }
}

// ---------------------------------------------------------------------------
// Scopes — teardown bookkeeping for control-flow blocks.
// beginScope/endScope bracket a block's `make()`; every bind created inside
// (including nested blocks' binds, via nested scopes) is collected so
// dropScope can unregister the whole subtree when the block content is
// removed. Registration cost is one array push per bind.
// ---------------------------------------------------------------------------

export function beginScope(c) {
  const scope = { subs: [], children: [], parent: c.scope };
  if (c.scope) c.scope.children.push(scope);
  c.scope = scope;
  return scope;
}

export function endScope(c) {
  c.scope = c.scope ? c.scope.parent : null;
}

// runScope(c, scope) — re-run every live bind registered in `scope` and its
// child scopes (nested blocks' content). Used by forBlock's patch path: after
// an item's data cell is updated, re-running the item's scope refreshes every
// item-local bind — including nested ifBlock/forBlock binds, which re-evaluate
// their condition/reconcile with the fresh data (for-diff-design.md §6).
// Scopes dropped mid-walk are safe: dropScope empties their arrays and marks
// their records dead, so they no-op here.
export function runScope(c, scope) {
  // Snapshot first: a sub may create child scopes (a branch shown by this very
  // walk) whose binds already ran at creation, or drop existing ones (their
  // arrays are emptied by dropScope, so recursing into them is a no-op).
  const subs = scope.subs.slice();
  const children = scope.children.slice();
  for (const s of subs) if (s.alive) s.fn();
  for (const ch of children) runScope(c, ch);
}

export function dropScope(c, scope) {
  for (const s of scope.subs) unbind(c, s);
  for (const ch of scope.children) {
    ch.parent = null; // avoid double-splice below
    dropScope(c, ch);
  }
  scope.subs.length = 0;
  scope.children.length = 0;
  if (scope.parent) {
    const sib = scope.parent.children;
    const k = sib.indexOf(scope);
    if (k >= 0) sib.splice(k, 1);
    scope.parent = null;
  }
}
