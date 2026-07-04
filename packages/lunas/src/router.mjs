// router.mjs — client-side router runtime for Lunas-compiled apps.
// See output-design.md §5 (router block) and the old @useRouting/@useAutoRouting
// directives (crates/lunas_parser) whose codegen targets this module.
//
// A router is conceptually a store of the current route: exactly one reactive
// field ("route") holding `{ path, params, query }`. It reuses store.mjs so a
// component adopts the current route at a reactive index the same way it adopts
// any other store field — `useStore`-shaped adoption — and plain consumers
// (tests, devtools, the outlet) can `subscribe` without a component context.
//
// History access is abstracted behind an injectable adapter so the runtime is
// testable without a real browser: `historyAdapter()` wraps the History API in
// the browser, `memoryHistory()` is an in-memory stand-in for tests/SSR.
//
// Dependency-free ES2015 + Proxy (the runtime's compatibility floor). No BigInt.

import { createStore, useStore } from "./store.mjs";
import { mountChild } from "./blocks.mjs";

// ---------------------------------------------------------------------------
// Path parsing & query strings
// ---------------------------------------------------------------------------

// splitPath(p) — the pathname portion of `p`, without its query string, split
// into non-empty segments. Leading/trailing slashes collapse away so "/a/b/"
// and "/a/b" segment identically (trailing-slash normalization, §1).
function splitPath(p) {
  const q = p.indexOf("?");
  const path = q === -1 ? p : p.slice(0, q);
  const out = [];
  for (const seg of path.split("/")) if (seg.length) out.push(decode(seg));
  return out;
}

// parseQuery(p) — the `?a=1&b=2` portion of `p` as a plain object. Repeated
// keys keep the last value (the common case; a router is not a form parser).
// Keys/values are percent-decoded and `+` is treated as a space.
function parseQuery(p) {
  const i = p.indexOf("?");
  const out = {};
  if (i === -1) return out;
  const qs = p.slice(i + 1);
  if (!qs.length) return out;
  for (const pair of qs.split("&")) {
    if (!pair.length) continue;
    const eq = pair.indexOf("=");
    const k = eq === -1 ? pair : pair.slice(0, eq);
    const v = eq === -1 ? "" : pair.slice(eq + 1);
    out[decode(k.replace(/\+/g, " "))] = decode(v.replace(/\+/g, " "));
  }
  return out;
}

function decode(s) {
  try {
    return decodeURIComponent(s);
  } catch (_e) {
    return s; // malformed %-sequence: hand back the raw text rather than throw
  }
}

// normalizePath(p) — a canonical pathname (leading slash, no trailing slash,
// query stripped). "" and "/" both normalize to "/".
function normalizePath(p) {
  const segs = splitPath(p);
  return segs.length ? "/" + segs.map(encodeURIComponent).join("/") : "/";
}

// ---------------------------------------------------------------------------
// Route table + matching (§1)
// ---------------------------------------------------------------------------

// compileRoute(route) — pre-split a route's `path` into a matcher spec once at
// createRouter time. Each segment is classified:
//   { kind: "static", value }  — must equal the incoming segment
//   { kind: "param",  name }   — ":name", captures one segment
//   { kind: "catch",  name }   — "*" or "*name", captures the rest (0+ segs)
// Specificity is scored so a matcher can be ranked: static beats param beats
// catch-all, earlier segments dominate later ones.
function compileRoute(route) {
  const raw = splitPath(route.path);
  const segs = [];
  for (const s of raw) {
    if (s === "*" || s[0] === "*") {
      segs.push({ kind: "catch", name: s.length > 1 ? s.slice(1) : "rest" });
      break; // a catch-all consumes everything after it
    } else if (s[0] === ":") {
      segs.push({ kind: "param", name: s.slice(1) });
    } else {
      segs.push({ kind: "static", value: s });
    }
  }
  return { route, segs };
}

// matchOne(spec, segs) — try to match compiled `spec` against the incoming
// path segments. Returns the captured params object on success, or null.
function matchOne(spec, segs) {
  const params = {};
  const specSegs = spec.segs;
  for (let i = 0; i < specSegs.length; i++) {
    const s = specSegs[i];
    if (s.kind === "catch") {
      // Greedily capture every remaining segment as a single "/"-joined string.
      params[s.name] = segs.slice(i).map(encodeURIComponent).join("/");
      return params;
    }
    if (i >= segs.length) return null; // spec wants more segments than we have
    if (s.kind === "static") {
      if (s.value !== segs[i]) return null;
    } else {
      params[s.name] = segs[i];
    }
  }
  // No catch-all: every incoming segment must have been consumed.
  return segs.length === specSegs.length ? params : null;
}

// rank(spec) — a comparable score for a matching spec: static segments weigh
// most, params less, a catch-all least. Longer static prefixes win. The number
// is only ever compared against other ranks, never interpreted.
function rank(spec) {
  let score = 0;
  for (const s of spec.segs) {
    score *= 8;
    if (s.kind === "static") score += 4;
    else if (s.kind === "param") score += 2;
    else score += 1; // catch-all
  }
  return score;
}

// matchRoute(compiled, path) — best matching route for `path`, or null.
// All matching specs are ranked (static > param > catch-all) and the highest
// wins; ties fall back to declaration order (first wins), matching the
// intuition that earlier routes are more specific by author intent.
function matchRoute(compiled, path) {
  const segs = splitPath(path);
  let best = null;
  let bestRank = -1;
  for (let i = 0; i < compiled.length; i++) {
    const spec = compiled[i];
    const params = matchOne(spec, segs);
    if (params === null) continue;
    const r = rank(spec);
    if (r > bestRank) {
      bestRank = r;
      best = { route: spec.route, params };
    }
  }
  return best;
}

// ---------------------------------------------------------------------------
// History adapters (§2)
// ---------------------------------------------------------------------------

// memoryHistory(initial) — an in-memory History-API stand-in for tests/SSR.
// Keeps its own back-stack; `listen`'s callback is invoked on back()/forward()
// (the popstate analogue) but NOT on push()/replace() (mirrors the browser,
// where pushState does not emit popstate — the router calls its own resolve
// after a programmatic navigation).
export function memoryHistory(initial) {
  const stack = [initial || "/"];
  let idx = 0;
  let onChange = null;
  return {
    get location() {
      return stack[idx];
    },
    push(path) {
      stack.length = idx + 1; // drop any forward entries
      stack.push(path);
      idx = stack.length - 1;
    },
    replace(path) {
      stack[idx] = path;
    },
    go(delta) {
      const next = idx + delta;
      if (next < 0 || next >= stack.length) return;
      idx = next;
      if (onChange) onChange(stack[idx]);
    },
    listen(fn) {
      onChange = fn;
      return () => {
        if (onChange === fn) onChange = null;
      };
    },
  };
}

// historyAdapter(win) — the default adapter wrapping the browser History API.
// `win` defaults to the global `window`; passing one explicitly aids testing
// and future SSR shims. Reads location from `pathname + search`, writes via
// pushState/replaceState, and forwards popstate to `listen`'s callback.
export function historyAdapter(win) {
  const w = win || (typeof window !== "undefined" ? window : null);
  if (!w) {
    throw new Error(
      "historyAdapter: no window available; pass options.history (e.g. memoryHistory()) for non-browser environments"
    );
  }
  const here = () => w.location.pathname + w.location.search;
  return {
    get location() {
      return here();
    },
    push(path) {
      w.history.pushState({}, "", path);
    },
    replace(path) {
      w.history.replaceState({}, "", path);
    },
    go(delta) {
      w.history.go(delta);
    },
    listen(fn) {
      const handler = () => fn(here());
      w.addEventListener("popstate", handler);
      return () => w.removeEventListener("popstate", handler);
    },
  };
}

// ---------------------------------------------------------------------------
// Router (§2, §3, §6)
// ---------------------------------------------------------------------------

// resolve(compiled, path) — the full reactive route object for `path`:
// `{ path, params, query, matched }` where `matched` is the winning route
// definition (or null for a total miss — but a `*` catch-all route, if
// declared, always matches, giving apps a natural 404 slot).
function resolve(compiled, path) {
  const m = matchRoute(compiled, path);
  return {
    path: normalizePath(path),
    params: m ? m.params : {},
    query: parseQuery(path),
    matched: m ? m.route : null,
  };
}

// createRouter(routes, options) — build a router over `routes`
//   routes  = [{ path: "/users/:id", component: UserPage }, …]
//   options = {
//     history?:    an adapter (defaults to historyAdapter() over `window`)
//     beforeEach?: (to, from) => boolean | Promise<boolean>  — false cancels
//   }
//
// The current route lives in a store field named "route", so components adopt
// it via `useStore(c, i, router.store, "route")` (or the `router.adopt` sugar)
// and plain consumers via `router.subscribe(fn)`.
export function createRouter(routes, options) {
  const opts = options || {};
  const history = opts.history || historyAdapter();
  const beforeEach = opts.beforeEach || null;
  const compiled = (routes || []).map(compileRoute);

  const store = createStore({ route: resolve(compiled, history.location) });

  // navigate(path, mode) — the single entry point behind push/replace and the
  // popstate listener. Runs the (optional) async guard first: a falsy result
  // cancels the navigation (the store is left untouched). `mode` is "push",
  // "replace", or "pop" (a browser-driven change already reflected in history,
  // so we only update the store, never call history again).
  function navigate(path, mode) {
    const from = store.get("route");
    const to = resolve(compiled, path);
    const commit = () => {
      if (mode === "push") history.push(path);
      else if (mode === "replace") history.replace(path);
      store.set("route", to);
    };
    if (!beforeEach) {
      commit();
      return Promise.resolve(true);
    }
    return Promise.resolve(beforeEach(to, from)).then((ok) => {
      if (ok === false) return false;
      commit();
      return true;
    });
  }

  // Browser back/forward (or memoryHistory.go) lands here already committed to
  // history; resolve into the store without pushing again.
  const stopListen = history.listen((path) => {
    navigate(path, "pop");
  });

  return {
    // The backing store — components adopt the route field through it.
    store,

    // current — the reactive route object `{ path, params, query, matched }`.
    // Reading it declares no dependency by itself; adoption is what wires
    // reactivity (see `adopt`/`useStore`).
    get current() {
      return store.get("route");
    },

    // push(path) — navigate, pushing a new history entry. Returns a Promise
    // resolving to whether the navigation committed (a guard may cancel it).
    push(path) {
      return navigate(path, "push");
    },

    // replace(path) — navigate, replacing the current history entry.
    replace(path) {
      return navigate(path, "replace");
    },

    // back() — go back one history entry (guard-free, like the browser button).
    back() {
      history.go(-1);
    },

    // forward() — go forward one history entry.
    forward() {
      history.go(1);
    },

    // subscribe(fn) — plain-JS subscription to route changes; `fn(route)` runs
    // synchronously on every committed navigation. Returns an unsubscribe fn.
    subscribe(fn) {
      return store.subscribe("route", fn);
    },

    // adopt(c, i) — sugar over `useStore(c, i, router.store, "route")`: adopt
    // the current route at component context `c`'s reactive index `i`. Returns
    // a detach() (idempotent, scope-aware). This is the compiler-facing hook
    // for a component that reads `router.current` in its template/handlers.
    adopt(c, i) {
      return useStore(c, i, store, "route");
    },

    // destroy() — detach the history listener (for teardown/HMR).
    destroy() {
      stopListen();
    },
  };
}

// ---------------------------------------------------------------------------
// Outlet (§4)
// ---------------------------------------------------------------------------

// routerOutlet(c, anchor, router, options) — mount the matched route's
// component at the text `anchor` (mountChild semantics: inserted before the
// anchor), swapping it out whenever navigation changes which route matches.
//
// The matched route's captured params are passed as props to the component;
// `options.props(route)` can override/augment that mapping (e.g. to inject the
// router itself or the parsed query). Re-mounts only when the *matched route
// definition* changes, not on every param tweak — a `/users/:id` component
// stays mounted across `1 → 2` (its own props reactivity handles the change);
// the outlet only owns mount/unmount.
//
// Teardown: mountChild returns an { unmount } that removes the root node.
// blocks.mjs's mountChild does not itself open a scope, so an outlet-mounted
// component owns its own reactive context (created by `component()`), which is
// dropped along with the removed subtree — nothing to dropScope here. The
// outlet subscribes to the router directly (plain-JS subscribe), so it needs
// no reactive index of its own and returns a handle with destroy().
//
// Intended emitted shape (compiler-facing contract): the `<router-outlet/>`
// element (or the placeholder an @useAutoRouting directive expands to) compiles
// to an anchor + a single routerOutlet call:
//
//   const a = anchorBefore(placeholder);
//   const outlet = routerOutlet(c, a, appRouter);
function routerOutlet(c, anchor, router, options) {
  const opts = options || {};
  const makeProps = opts.props || null;
  let mounted = null; // { root, unmount } from mountChild
  let currentKey = undefined; // stable key of the route def currently mounted

  const propsFor = (route) =>
    makeProps ? makeProps(route) : Object.assign({}, route.params);

  // The route object read from the store is deep-proxy-wrapped (store.mjs), so
  // `route.matched` is a *view* of the route definition, never `===` the
  // original — identity checks won't work. Route `path`s are the unique keys of
  // the route table, so use the matched def's path (or a null sentinel for a
  // 404 miss) as the "did the matched route change?" signal.
  const keyOf = (route) => (route.matched ? route.matched.path : null);

  const render = (route) => {
    const key = keyOf(route);
    // Same route definition still matches: leave the mounted component in
    // place. A distinct key (including null → route, or route → null on a 404
    // miss with no catch-all) swaps.
    if (key === currentKey) return;
    if (mounted) {
      mounted.unmount();
      mounted = null;
    }
    currentKey = key;
    if (route.matched && route.matched.component) {
      mounted = mountChild(c, anchor, route.matched.component, propsFor(route));
    }
  };

  render(router.current);
  const unsub = router.subscribe(render);

  return {
    destroy() {
      unsub();
      if (mounted) {
        mounted.unmount();
        mounted = null;
      }
      currentKey = undefined;
    },
  };
}

export { routerOutlet };

// ---------------------------------------------------------------------------
// Links (§5)
// ---------------------------------------------------------------------------

// routerLink(el, router, path, options) — wire `el`'s click to a client-side
// navigation: preventDefault + router.push(path) (or router.replace when
// `options.replace`). Modified clicks (ctrl/meta/shift/alt) and non-primary
// buttons fall through to the browser so "open in new tab" still works — the
// standard SPA-link contract. Returns an unbind() that removes the listener.
//
// This is the codegen target for `<a :href="/users/1">…</a>` route links: the
// compiler emits `routerLink(aEl, router, "/users/1")` alongside setting the
// element's static `href` (kept for SSR/no-JS and middle-click).
function routerLink(el, router, path, options) {
  const opts = options || {};
  const handler = (e) => {
    // Respect modifier/aux clicks and any handler that already handled it.
    if (
      e.defaultPrevented ||
      (e.button !== undefined && e.button !== 0) ||
      e.metaKey ||
      e.ctrlKey ||
      e.shiftKey ||
      e.altKey
    ) {
      return;
    }
    if (typeof e.preventDefault === "function") e.preventDefault();
    if (opts.replace) router.replace(path);
    else router.push(path);
  };
  el.addEventListener("click", handler);
  return () => el.removeEventListener("click", handler);
}

export { routerLink };

// Pure helpers — useful for compiler tests and advanced use (a router is not
// required to parse a query string or canonicalize a path).
export { parseQuery, normalizePath };
