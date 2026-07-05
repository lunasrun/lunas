// router.edge.test.mjs — additional edge-focused coverage for router.mjs:
// nested/param/catch-all precedence details, query-string edge cases, more
// navigation sequencing, guard redirects, outlet remount rules, and link
// fallthrough behavior not already covered by router.test.mjs.
// Run: node packages/lunas/test/router.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind } from "../src/core.mjs";
import { component, anchorAppend } from "../src/dom.mjs";
import {
  createRouter,
  memoryHistory,
  routerOutlet,
  routerLink,
  parseQuery,
  normalizePath,
} from "../src/router.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));

function makeComponent(tag, sink) {
  return component("div", {}, "", (c, props) => {
    c.root.setAttribute("data-page", tag);
    if (sink) sink.push({ tag, props });
  });
}

// -- matching precedence, deeper nesting -------------------------------------

test("nested static beats nested param at every depth", () => {
  const routes = [
    { path: "/a/:x/b", component: makeComponent("param") },
    { path: "/a/lit/b", component: makeComponent("static") },
  ];
  const router = createRouter(routes, { history: memoryHistory("/a/lit/b") });
  assert.strictEqual(router.current.matched.path, "/a/lit/b");
});

test("catch-all only wins when nothing more specific matches", async () => {
  const routes = [
    { path: "/a/:x", component: makeComponent("param") },
    { path: "/*rest", component: makeComponent("catch") },
  ];
  const router = createRouter(routes, { history: memoryHistory("/a/1") });
  assert.strictEqual(router.current.matched.path, "/a/:x");
  await router.push("/a/1/2/3");
  assert.strictEqual(router.current.matched.path, "/*rest");
});

test("declaration order breaks ties between equally-ranked routes", () => {
  const routes = [
    { path: "/:a/:b", component: makeComponent("first") },
    { path: "/:x/:y", component: makeComponent("second") },
  ];
  const router = createRouter(routes, { history: memoryHistory("/1/2") });
  assert.strictEqual(
    router.current.matched.path,
    "/:a/:b",
    "first declared route wins a tie"
  );
});

test("segment count must match exactly when there's no catch-all", () => {
  const routes = [{ path: "/a/:x", component: makeComponent("p") }];
  const router = createRouter(routes, { history: memoryHistory("/a/1/2") });
  assert.strictEqual(
    router.current.matched,
    null,
    "extra trailing segment does not match a fixed-length spec"
  );
});

// -- trailing slash + root ----------------------------------------------------

test("trailing slash on root path normalizes to /", () => {
  const router = createRouter(
    [{ path: "/", component: makeComponent("home") }],
    { history: memoryHistory("/") }
  );
  assert.strictEqual(router.current.path, "/");
  assert.strictEqual(router.current.matched.path, "/");
});

test("multiple trailing slashes collapse the same as one", () => {
  const router = createRouter(
    [{ path: "/a/b", component: makeComponent("ab") }],
    { history: memoryHistory("/a/b///") }
  );
  assert.strictEqual(router.current.path, "/a/b");
  assert.strictEqual(router.current.matched.path, "/a/b");
});

// -- query parsing edge cases -------------------------------------------------

test("query parse: empty query string yields an empty object", () => {
  assert.deepStrictEqual(parseQuery("/x"), {});
  assert.deepStrictEqual(parseQuery("/x?"), {});
});

test("query parse: repeated keys keep the LAST value", () => {
  assert.deepStrictEqual(parseQuery("/x?a=1&a=2&a=3"), { a: "3" });
});

test("query parse: percent-encoded keys and values decode correctly", () => {
  assert.deepStrictEqual(parseQuery("/x?na%20me=%E2%98%83"), {
    "na me": "☃",
  });
});

test("query parse: malformed percent-sequence falls back to raw text", () => {
  // decode() catches the URIError and returns the raw (still-encoded) string.
  assert.deepStrictEqual(parseQuery("/x?bad=%E0%A4%A"), { bad: "%E0%A4%A" });
});

test("query parse: value-less key (no '=') yields empty string", () => {
  assert.deepStrictEqual(parseQuery("/x?flag"), { flag: "" });
  assert.deepStrictEqual(parseQuery("/x?flag&a=1"), { flag: "", a: "1" });
});

test("normalizePath: query string is stripped", () => {
  assert.strictEqual(normalizePath("/a/b?x=1"), "/a/b");
});

// -- navigation: push/replace/back/forward stack semantics -------------------

test("push after back drops the forward stack (browser semantics)", async () => {
  const hist = memoryHistory("/a");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
  });
  await router.push("/b");
  await router.push("/c");
  router.back(); // -> /b
  await tick();
  assert.strictEqual(router.current.path, "/b");
  await router.push("/d"); // drop the "/c" forward entry
  assert.strictEqual(router.current.path, "/d");
  router.forward(); // nothing beyond /d
  await tick();
  assert.strictEqual(
    router.current.path,
    "/d",
    "forward stack cleared by the intervening push"
  );
});

test("forward() at the end of history is a no-op", async () => {
  const hist = memoryHistory("/a");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
  });
  await router.push("/b");
  router.forward();
  await tick();
  assert.strictEqual(router.current.path, "/b", "already at the end");
});

test("back() at the start of history is a no-op", async () => {
  const hist = memoryHistory("/only");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
  });
  router.back();
  await tick();
  assert.strictEqual(router.current.path, "/only");
});

// -- guards: cancel (sync + async), redirect-in-guard ------------------------

test("beforeEach returning a rejected promise propagates (never silently commits)", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/start"),
    beforeEach: () => Promise.reject(new Error("guard blew up")),
  });
  await assert.rejects(() => router.push("/blocked"), /guard blew up/);
  assert.strictEqual(
    router.current.path,
    "/start",
    "route unchanged when the guard's promise rejects"
  );
});

test("beforeEach can redirect by calling push() itself inside the guard", async () => {
  const seen = [];
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/start"),
    beforeEach: (to) => {
      seen.push(to.path);
      if (to.path === "/private") {
        // Deny this navigation, then redirect to /login. This inner push runs
        // its own beforeEach pass too, but /login isn't guarded away.
        router.push("/login");
        return false;
      }
      return true;
    },
  });
  const ok = await router.push("/private");
  assert.strictEqual(ok, false, "the original navigation reports cancelled");
  await tick();
  assert.strictEqual(
    router.current.path,
    "/login",
    "the guard's own redirect commits"
  );
});

test("guard cancel leaves history untouched (no orphan entry)", async () => {
  const hist = memoryHistory("/start");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
    beforeEach: (to) => to.path !== "/blocked",
  });
  await router.push("/blocked");
  assert.strictEqual(hist.location, "/start", "history.push never ran");
});

// -- outlet: remount only on route-def change --------------------------------

test("outlet remounts when navigating between two distinct route defs, even with overlapping params", async () => {
  const hits = [];
  const routes = [
    { path: "/users/:id", component: makeComponent("user", hits) },
    { path: "/posts/:id", component: makeComponent("post", hits) },
  ];
  const router = createRouter(routes, { history: memoryHistory("/users/1") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  routerOutlet(c, anchor, router);

  const firstRoot = parent.childNodes[0];
  await router.push("/posts/1"); // same param value, different route def
  assert.notStrictEqual(
    parent.childNodes[0],
    firstRoot,
    "distinct route def remounts even though the param value is identical"
  );
  assert.strictEqual(hits.length, 2);
});

test("outlet 404 -> match -> 404 toggles mount/unmount without a catch-all", async () => {
  const hits = [];
  const routes = [{ path: "/home", component: makeComponent("home", hits) }];
  const router = createRouter(routes, { history: memoryHistory("/home") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  routerOutlet(c, anchor, router);
  assert.strictEqual(parent.childNodes.length, 2, "home mounted + anchor");

  await router.push("/missing");
  assert.strictEqual(
    parent.childNodes.length,
    1,
    "no match unmounts, leaving only the anchor"
  );

  await router.push("/home");
  assert.strictEqual(parent.childNodes.length, 2, "remounted on match again");
  assert.strictEqual(hits.length, 2, "mounted twice total (not on the miss)");
});

test("outlet with options.props overrides the default params-as-props mapping", () => {
  const hits = [];
  const routes = [{ path: "/users/:id", component: makeComponent("u", hits) }];
  const router = createRouter(routes, { history: memoryHistory("/users/9") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  routerOutlet(c, anchor, router, {
    props: (route) => ({ userId: route.params.id, extra: true }),
  });
  assert.deepStrictEqual(hits[0].props, { userId: "9", extra: true });
});

test("destroy() before any navigation still leaves the anchor and no listener", async () => {
  const hits = [];
  const routes = [{ path: "/home", component: makeComponent("home", hits) }];
  const router = createRouter(routes, { history: memoryHistory("/home") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  const outlet = routerOutlet(c, anchor, router);
  outlet.destroy();
  assert.strictEqual(parent.childNodes.length, 1, "unmounted, anchor remains");
  // A navigation after destroy must not resurrect the outlet.
  await router.push("/home");
  assert.strictEqual(parent.childNodes.length, 1, "outlet stays torn down");
});

// -- links: modified-click / non-primary-button fallthrough ------------------

test("routerLink: non-primary mouse button falls through", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/"),
  });
  const a = document.createElement("a");
  routerLink(a, router, "/target");
  for (const fn of a._listeners.click.slice()) {
    fn({ type: "click", button: 1, preventDefault() {} }); // middle click
  }
  await tick();
  assert.strictEqual(
    router.current.path,
    "/",
    "auxiliary button click does not navigate"
  );
});

test("routerLink: an already-defaultPrevented event is left alone", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/"),
  });
  const a = document.createElement("a");
  routerLink(a, router, "/target");
  let calledPreventDefault = false;
  for (const fn of a._listeners.click.slice()) {
    fn({
      type: "click",
      defaultPrevented: true,
      preventDefault() {
        calledPreventDefault = true;
      },
    });
  }
  await tick();
  assert.strictEqual(router.current.path, "/");
  assert.strictEqual(calledPreventDefault, false);
});

test("routerLink: options.replace uses history.replace instead of push", async () => {
  const hist = memoryHistory("/start");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
  });
  await router.push("/mid"); // so we have a back-entry to prove replace didn't add one
  const a = document.createElement("a");
  routerLink(a, router, "/final", { replace: true });
  for (const fn of a._listeners.click.slice()) {
    fn({ type: "click", preventDefault() {} });
  }
  await tick();
  assert.strictEqual(router.current.path, "/final");
  router.back();
  await tick();
  assert.strictEqual(
    router.current.path,
    "/start",
    "replace did not push a new history entry for /final"
  );
});

// -- reactive current-route wired through a plain store-adopting context ----

test("two components adopting the same router route independently at different indices", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/one"),
  });
  const c1 = createContext(null);
  const c2 = createContext(null);
  router.adopt(c1, 2);
  router.adopt(c2, 5);
  const seen1 = [];
  const seen2 = [];
  bind(c1, [2], () => seen1.push(router.current.path));
  bind(c2, [5], () => seen2.push(router.current.path));

  await router.push("/two");
  await tick();
  assert.deepStrictEqual(seen1, ["/one", "/two"]);
  assert.deepStrictEqual(seen2, ["/one", "/two"]);
});

test("router.destroy() detaches the history listener (pop no longer updates the store)", async () => {
  const hist = memoryHistory("/a");
  const router = createRouter([{ path: "*", component: null }], {
    history: hist,
  });
  await router.push("/b");
  router.destroy();
  router.back();
  await tick();
  assert.strictEqual(
    router.current.path,
    "/b",
    "history moved but the router no longer listens"
  );
  assert.strictEqual(hist.location, "/a", "history itself still moved back");
});
