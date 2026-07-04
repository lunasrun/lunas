// computed.mjs — lazily-evaluated derived values in the adjacency model.
// See output-design.md §4 (compile-time dependency dispatch).
//
// A computed is a derived reactive variable. Like every reactive variable it
// owns an index `i`; parts that read it declare `i` in their deps just like a
// plain box. The compiler supplies both the computed's own index and the set of
// upstream indices it reads (its `deps` mask) — there is NO runtime Proxy
// tracking, consistent with the rest of the runtime.
//
// Laziness: the compute fn runs only when the value is actually read AND an
// upstream dep has changed since the last computation. When an upstream dep
// changes we do not recompute eagerly; we mark the value stale and mark the
// computed's own index dirty so downstream binds are re-queued. Those binds
// pull the fresh value on their next run, which triggers exactly one recompute.

import { bind, markVar } from "./core.mjs";

// computed(c, i, deps, fn) — derived value at reactive index `i` reading the
// upstream reactive indices in `deps`. Returns a box-shaped handle whose `.v`
// getter yields the (memoized) result. Reading `.v` inside a bound updater
// works because that updater declares `i` in its own deps.
//
// The internal bind on `deps` costs one adjacency record per upstream index; it
// does no work beyond flipping the stale flag and re-marking `i`, so a computed
// whose value is never read never recomputes.
export function computed(c, i, deps, fn) {
  let value;
  let stale = true;
  let primed = false; // suppress the bind's initial synchronous run

  // When any upstream dep changes, invalidate and propagate to `i`'s
  // dependents. We do not recompute here — recompute is deferred to the next
  // read (laziness). The initial bind run is skipped: at wiring time `i` has no
  // dependents yet, so marking it would only schedule a wasted flush.
  bind(c, deps, () => {
    if (!primed) {
      primed = true;
      return;
    }
    stale = true;
    markVar(c, i);
  });

  return {
    get v() {
      if (stale) {
        value = fn();
        stale = false;
      }
      return value;
    },
  };
}
