// watch.mjs — user-facing watchers over the adjacency model.
// See output-design.md §4–5.
//
// Watchers are just binds with a policy layer. Deps are explicit reactive
// indices (compiler- or user-supplied), never Proxy-tracked. Both variants
// register through `bind`, so they honor the current scope: a watcher created
// inside beginScope/endScope is torn down by dropScope, and the returned
// stop() unbinds it explicitly at any time.

import { bind, unbind } from "./core.mjs";

// watch(c, deps, cb, opts?) — run `cb` after any of `deps` changes.
//
// By default the first (synchronous) run performed by `bind` is suppressed, so
// the callback fires only on subsequent changes — the usual "watch" semantics.
// Pass { immediate: true } to also invoke it once at registration time.
//
// Returns a stop() that unbinds the watcher. The watcher is also collected by
// the current scope, so dropScope tears it down automatically.
export function watch(c, deps, cb, opts) {
  const immediate = opts && opts.immediate;
  let primed = false;
  const s = bind(c, deps, () => {
    if (!primed) {
      primed = true;
      if (immediate) cb();
      return;
    }
    cb();
  });
  return () => unbind(c, s);
}

// watchEffect(c, deps, fn) — run `fn` immediately and again after any of `deps`
// changes. This is `watch` with { immediate: true } but with the effect-style
// name and no distinction between the initial and later runs (the effect always
// runs on registration). Returns a stop().
export function watchEffect(c, deps, fn) {
  const s = bind(c, deps, fn);
  return () => unbind(c, s);
}
