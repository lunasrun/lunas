// async.mjs — async / lazy components + suspense boundaries.
// See output-design.md §5 (runtime API) and §7 (mount lifecycle).
//
// Two cooperating primitives, dependency-free (ES2015 + Promise), tree-shakeable:
//
//   asyncComponent(loader, opts) — turn a `() => import("./Heavy.mjs")` loader
//     into a mountable child factory. Resolves the module (default export or
//     direct factory), caches after first load (later mounts are synchronous),
//     and shows Vue-style loading/error components with a `delay`/`timeout`.
//
//   suspenseBlock(c, anchor, contentFactory, fallbackFactory) — a boundary at a
//     text anchor. It shows `fallback` while any async component under it is
//     pending, then reveals `content` once every pending dep resolves.
//
// ── Boundary-registration mechanism ────────────────────────────────────────
// A suspense boundary publishes itself on the component context as `c._suspense`
// for the duration of its content build. `asyncComponent`, when mounted, reads
// the *current* `c._suspense` and — if present — registers with it: it bumps the
// boundary's pending counter while the module loads and settles it (found / miss)
// once resolved or rejected. Nested boundaries save the enclosing `c._suspense`,
// install themselves, build their content, then restore the parent — so an inner
// boundary owns exactly its own subtree and never leaks pending counts upward.
// This reuses the existing context object; core.mjs / blocks.mjs are untouched.
//
// ── Unmount safety ─────────────────────────────────────────────────────────
// Every async mount owns a live token ({ alive: true }). Unmount flips it false;
// a loader that settles afterwards writes no DOM and settles no boundary. Boundary
// swaps run through afterFlush so sync-resolved loads never flash the fallback.

import { afterFlush } from "./core.mjs";
import { mountChild } from "./blocks.mjs";

// unwrap(mod) — accept an ES module namespace ({ default }), a bare factory, or
// a `{ default }` wrapper object, and return the child-component factory.
function unwrap(mod) {
  if (mod && typeof mod === "object" && "default" in mod) return mod.default;
  return mod;
}

// asyncComponent(loader, opts) — returns a childFactory usable with mountChild.
//
//   loader : () => Promise<Module|Factory> | Module | Factory
//   opts   : {
//     loading? : childFactory shown while pending (only after `delay` ms),
//     error?   : childFactory shown on rejection / timeout,
//     delay?   : ms to wait before showing `loading` (default 200; avoids flash),
//     timeout? : ms after which a still-pending load is treated as an error,
//   }
//
// The returned factory obeys the mountChild contract: called with `props`, it
// returns a root Node immediately (a placeholder anchor + optional loading UI),
// and swaps in the resolved component when the module lands. Once resolved, the
// module is cached, so subsequent mounts build synchronously with no placeholder.
export function asyncComponent(loader, opts) {
  opts = opts || {};
  const delay = opts.delay == null ? 200 : opts.delay;
  const timeout = opts.timeout;
  const loadingFactory = opts.loading || null;
  const errorFactory = opts.error || null;

  // Per-wrapper cache: `resolved` holds the settled child factory; `promise`
  // dedupes concurrent in-flight loads so N mounts share one import().
  let resolved = null;
  let promise = null;

  const load = () => {
    if (resolved) return Promise.resolve(resolved);
    if (promise) return promise;
    let out;
    try {
      out = loader();
    } catch (e) {
      return Promise.reject(e);
    }
    // A synchronous (already-imported / factory) loader still normalizes to a
    // promise so the mount path has one shape.
    promise = Promise.resolve(out).then(
      (mod) => {
        resolved = unwrap(mod);
        promise = null;
        return resolved;
      },
      (err) => {
        promise = null;
        throw err;
      }
    );
    return promise;
  };

  // The returned factory is invoked as `factory(props)` by mountChild (no
  // `this`). The component context is threaded onto `mountAsync._c` by
  // mountAsyncChild right before the call, so the factory can reach
  // `c._suspense` and afterFlush without a `this`-binding contract.
  const mountAsync = function (props) {
    const c = mountAsync._c || null;
    const boundary = c ? c._suspense : null;
    return buildAsyncRoot(c, boundary, props, {
      load,
      resolved: () => resolved,
      delay,
      timeout,
      loadingFactory,
      errorFactory,
    });
  };
  mountAsync._c = null;
  return mountAsync;
}

// buildAsyncRoot — construct the mountable root for one async mount. Returns a
// Node synchronously (placeholder anchor); swaps children in as the load settles.
//
// The root is a permanent empty text node acting as its own anchor; resolved /
// loading / error content is inserted before it and removed on the next swap.
// This keeps a single stable node in the parent's child list (mountChild inserts
// `root`), so unmount()/removal is a single node.remove() on the placeholder plus
// teardown of whatever child is currently shown.
function buildAsyncRoot(c, boundary, props, cfg) {
  const anchor = makeAnchor(c);
  const token = { alive: true };
  let shown = null; // { root, unmount } | null — the currently-mounted child

  // If we have a live suspense boundary, register the pending dep now (before
  // any async work) so a synchronously-resolvable child still counts until it
  // actually mounts — the boundary settles it in `finish`.
  const settled = { done: false };
  if (boundary) boundary.enter();
  const settleBoundary = () => {
    if (boundary && !settled.done) {
      settled.done = true;
      boundary.leave();
    }
  };

  // The anchor is inserted into the parent by mountChild AFTER this function
  // returns, so a synchronous insert here would hit a detached anchor. Buffer
  // any node whose insert races the attach and drain it in `_asyncAttached`
  // (called by mountAsyncChild right after the anchor lands).
  let queued = null;
  const insert = (childRoot) => {
    if (anchor.parentNode) anchor.parentNode.insertBefore(childRoot, anchor);
    else (queued || (queued = [])).push(childRoot);
  };
  const clear = () => {
    if (shown) {
      shown.unmount();
      shown = null;
    }
  };
  const swapTo = (factory) => {
    if (!token.alive) return;
    clear();
    const root = factory(props);
    insert(root);
    shown = { root, unmount: () => root.remove() };
  };

  const cached = cfg.resolved();
  if (cached) {
    // Cache hit: mount synchronously, no placeholder, no flash.
    swapTo(cached);
    settleBoundary();
  } else {
    // Cold load. Optionally show `loading` after `delay` ms (0 → immediately).
    let delayTimer = null;
    let timeoutTimer = null;

    const showLoading = () => {
      if (!token.alive || shown) return;
      if (cfg.loadingFactory) swapTo(cfg.loadingFactory);
    };
    const showError = (err) => {
      if (!token.alive) return;
      if (cfg.errorFactory) {
        clear();
        const root = cfg.errorFactory(Object.assign({ error: err }, props));
        insert(root);
        shown = { root, unmount: () => root.remove() };
      }
    };

    if (cfg.loadingFactory) {
      if (cfg.delay <= 0) showLoading();
      else delayTimer = setTimeout(showLoading, cfg.delay);
    }
    if (cfg.timeout != null) {
      timeoutTimer = setTimeout(() => {
        // Still pending when the timeout fires → treat as an error. `_timedOut`
        // makes the later load resolution a no-op (no late swap-in).
        if (token.alive && !cfg.resolved()) {
          token._timedOut = true;
          showError(new Error("async component: load timed out"));
          settleBoundary();
        }
      }, cfg.timeout);
    }

    const clearTimers = () => {
      if (delayTimer) clearTimeout(delayTimer);
      if (timeoutTimer) clearTimeout(timeoutTimer);
      delayTimer = timeoutTimer = null;
    };

    cfg.load().then(
      (factory) => {
        clearTimers();
        if (!token.alive || token._timedOut) return; // unmounted / already errored
        // Reveal on the flush boundary so a fast (micro-tick) resolve that races
        // a batched update lands with no flicker.
        const reveal = () => {
          if (!token.alive || token._timedOut) return;
          swapTo(factory);
          settleBoundary();
        };
        if (c) afterFlush(c, reveal);
        else reveal();
      },
      (err) => {
        clearTimers();
        if (!token.alive) return;
        showError(err);
        settleBoundary();
      }
    );
  }

  // Drain buffered synchronous inserts once the anchor is attached. Called by
  // mountAsyncChild immediately after the anchor lands in the parent.
  anchor._asyncAttached = () => {
    if (queued) {
      const q = queued;
      queued = null;
      for (const n of q) anchor.parentNode.insertBefore(n, anchor);
    }
  };
  // Expose an unmount hook on the anchor so a mounting parent (or suspense
  // content teardown) can drop this subtree and cancel pending work.
  anchor._asyncUnmount = () => {
    token.alive = false;
    clear();
    queued = null;
    settleBoundary();
    anchor.remove();
  };
  return anchor;
}

// makeAnchor(c) — a permanent empty text node used as the async root's anchor.
// Resolves a document from the context root (works off-DOM / in tests).
function makeAnchor(c) {
  const doc =
    (c && c.root && c.root.ownerDocument) ||
    (typeof document !== "undefined" ? document : null);
  return doc.createTextNode("");
}

// mountAsyncChild(c, anchor, asyncFactory, props) — the mount entry codegen uses
// for an async component. Identical contract to mountChild, but threads the
// component context `c` into the async factory (so it can reach `c._suspense`
// and afterFlush) and returns an unmount that cancels in-flight loads.
export function mountAsyncChild(c, anchor, asyncFactory, props) {
  // Thread the context onto the factory for this mount. Async factories read
  // `_c` at call time; setting it right before the call keeps the closure clean
  // and avoids a `this`-binding contract through mountChild.
  asyncFactory._c = c;
  const m = mountChild(c, anchor, asyncFactory, props);
  asyncFactory._c = null;
  const root = m.root;
  // The async root (an anchor text node) is now attached; drain any synchronous
  // content (cache hit / delay-0 loading) that was buffered during the build.
  if (root._asyncAttached) root._asyncAttached();
  return {
    root,
    unmount() {
      if (root._asyncUnmount) root._asyncUnmount();
      else m.unmount();
    },
  };
}

// suspenseBlock(c, anchor, contentFactory, fallbackFactory)
//
// A boundary anchored at a text node. It builds `content` immediately (so async
// children under it begin loading), but keeps it hidden and shows `fallback`
// while any registered async dep is pending. When the pending counter reaches
// zero the fallback is torn down and the content revealed — batched via
// afterFlush so a fully-synchronous subtree never flashes the fallback.
//
//   contentFactory(c)  — returns a Node (or Node[]); async children inside it
//                        register with this boundary via `c._suspense`.
//   fallbackFactory()  — returns a Node (or Node[]) shown while pending.
//
// Nested boundaries: contentFactory runs with `c._suspense` pointing at THIS
// boundary; an inner suspenseBlock saves/restores that pointer, so async deps
// register with their nearest boundary only.
export function suspenseBlock(c, anchor, contentFactory, fallbackFactory) {
  const toNodes = (h) => (h == null ? [] : Array.isArray(h) ? h : [h]);
  const parent = anchor.parentNode;

  let pending = 0;
  let done = false;
  let alive = true;
  let revealScheduled = false;

  let contentNodes = null; // built once, held (detached) until revealed
  let fallbackNodes = null;

  const removeNodes = (list) => {
    if (list) for (const n of list) if (n.parentNode) n.remove();
  };
  const insertNodes = (list) => {
    if (list) for (const n of list) parent.insertBefore(n, anchor);
  };

  const reveal = () => {
    if (!alive || !done) return;
    removeNodes(fallbackNodes);
    fallbackNodes = null;
    insertNodes(contentNodes);
  };

  const maybeSettle = () => {
    if (pending === 0 && !done) {
      done = true;
      if (revealScheduled) return;
      revealScheduled = true;
      // Batch the swap so a synchronous subtree (pending hit 0 during build)
      // reveals content directly without ever painting the fallback.
      afterFlush(c, () => {
        revealScheduled = false;
        reveal();
      });
    }
  };

  const boundary = {
    enter() {
      pending++;
    },
    leave() {
      if (pending > 0) pending--;
      maybeSettle();
    },
  };

  // Install this boundary as the nearest one for the content build, saving any
  // enclosing boundary so nested suspense restores correctly.
  const prev = c._suspense;
  c._suspense = boundary;
  let built;
  try {
    built = contentFactory(c);
  } finally {
    c._suspense = prev;
  }
  contentNodes = toNodes(built);

  // Decide the initial paint after the content build settled its synchronous
  // deps. If nothing was pending, content is already `done`; reveal directly
  // (no fallback ever mounts). Otherwise mount the fallback now.
  if (done) {
    insertNodes(contentNodes);
  } else if (pending > 0) {
    fallbackNodes = toNodes(fallbackFactory ? fallbackFactory() : null);
    insertNodes(fallbackNodes);
  } else {
    // No async deps registered at all: reveal content immediately.
    done = true;
    insertNodes(contentNodes);
  }

  return {
    // Whether the boundary has revealed its content (settled). Useful for tests.
    isSettled() {
      return done;
    },
    destroy() {
      alive = false;
      // Cancel any pending async children in the (possibly still-hidden) content
      // by removing their anchors — each async root's `_asyncUnmount` flips its
      // token so late loads write nothing.
      cancelAsync(contentNodes);
      removeNodes(fallbackNodes);
      removeNodes(contentNodes);
      contentNodes = fallbackNodes = null;
      c._suspense = prev;
    },
  };
}

// cancelAsync(nodes) — best-effort cancel of async-component roots in a subtree
// list. Async roots are the anchor text nodes carrying `_asyncUnmount`; walking
// the parent list is enough because mountAsyncChild inserts each async root as a
// sibling before the content anchor. We also scan descendants defensively.
function cancelAsync(nodes) {
  if (!nodes) return;
  for (const n of nodes) {
    if (n && n._asyncUnmount) n._asyncUnmount();
    // Content built from an async child may itself hold nested async anchors as
    // sibling text nodes; a shallow descendant scan covers the common shapes.
    if (n && n.childNodes) {
      for (const ch of n.childNodes.slice ? n.childNodes.slice() : n.childNodes) {
        if (ch && ch._asyncUnmount) ch._asyncUnmount();
      }
    }
  }
}
