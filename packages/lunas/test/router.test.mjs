// router.test.mjs — client-side router runtime (createRouter/routerOutlet/
// routerLink) driven with memoryHistory, no real browser.
// Run: node packages/lunas/test/router.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind } from "../src/core.mjs";
import { component } from "../src/dom.mjs";
import { anchorAppend } from "../src/dom.mjs";
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

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// A trivial compiled component: <div>tag</div>, records the props it got.
function makeComponent(tag, sink) {
  return component("div", {}, "", (c, props) => {
    c.root.setAttribute("data-page", tag);
    if (sink) sink.push({ tag, props });
  });
}

// -- pure matching -----------------------------------------------------------

await test("matching: static beats param beats catch-all", async () => {
  // Route objects read back through the store are deep-proxy-wrapped, so we
  // assert on the matched route's stable `path` rather than object identity.
  const routes = [
    { path: "/users/:id", component: makeComponent("param") },
    { path: "/users/me", component: makeComponent("static") },
    { path: "*", component: makeComponent("catch") },
  ];
  const router = createRouter(routes, { history: memoryHistory("/users/me") });
  assert.strictEqual(router.current.matched.path, "/users/me",
    "static /users/me wins over /users/:id");
  await router.push("/users/42");
  assert.strictEqual(router.current.matched.path, "/users/:id",
    "param matches a non-literal segment");
  assert.deepStrictEqual(router.current.params, { id: "42" });
  await router.push("/nope/here");
  assert.strictEqual(router.current.matched.path, "*",
    "catch-all is the fallback");
});

await test("matching: params captured, catch-all captures the rest", async () => {
  const routes = [
    { path: "/a/:x/:y", component: null },
    { path: "/files/*rest", component: null },
  ];
  const router = createRouter(routes, { history: memoryHistory("/a/1/2") });
  assert.deepStrictEqual(router.current.params, { x: "1", y: "2" });
  await router.push("/files/deep/nested/path");
  assert.deepStrictEqual(router.current.params, { rest: "deep/nested/path" });
});

await test("query parsing + trailing-slash normalization", async () => {
  assert.deepStrictEqual(parseQuery("/x?a=1&b=two&flag"), {
    a: "1",
    b: "two",
    flag: "",
  });
  assert.deepStrictEqual(parseQuery("/x?q=a+b%20c"), { q: "a b c" });
  assert.strictEqual(normalizePath("/a/b/"), "/a/b");
  assert.strictEqual(normalizePath("/"), "/");
  assert.strictEqual(normalizePath(""), "/");

  const router = createRouter(
    [{ path: "/search", component: null }],
    { history: memoryHistory("/search/?q=hi&page=2") }
  );
  assert.strictEqual(router.current.path, "/search", "trailing slash normalized");
  assert.deepStrictEqual(router.current.query, { q: "hi", page: "2" });
  assert.strictEqual(router.current.matched.path, "/search",
    "trailing slash still matches the route");
});

// -- navigation --------------------------------------------------------------

await test("push / replace / back over memoryHistory", async () => {
  const hist = memoryHistory("/");
  const router = createRouter([{ path: "*", component: null }], { history: hist });
  await router.push("/a");
  assert.strictEqual(router.current.path, "/a");
  assert.strictEqual(hist.location, "/a");
  await router.push("/b");
  assert.strictEqual(router.current.path, "/b");
  await router.replace("/b2");
  assert.strictEqual(router.current.path, "/b2");
  assert.strictEqual(hist.location, "/b2");
  router.back();
  await tick();
  assert.strictEqual(router.current.path, "/a", "back returns to /a (replace kept one entry)");
});

await test("popstate (history.go) drives the store without re-pushing", async () => {
  const hist = memoryHistory("/one");
  const router = createRouter([{ path: "*", component: null }], { history: hist });
  await router.push("/two");
  router.back();
  await tick();
  assert.strictEqual(router.current.path, "/one");
  router.forward();
  await tick();
  assert.strictEqual(router.current.path, "/two");
});

// -- reactive current route through a component context ----------------------

await test("reactive current-route updates through a component (bind fires on nav)", async () => {
  const hist = memoryHistory("/home");
  const router = createRouter([{ path: "*", component: null }], { history: hist });
  const c = createContext(null);
  router.adopt(c, 0); // adopt route field at reactive index 0

  const seen = [];
  bind(c, [0], () => seen.push(router.current.path));
  assert.deepStrictEqual(seen, ["/home"], "bind ran once immediately");

  await router.push("/about");
  await tick();
  assert.deepStrictEqual(seen, ["/home", "/about"], "bind re-ran on navigation");

  await router.push("/contact");
  await tick();
  assert.deepStrictEqual(seen, ["/home", "/about", "/contact"]);
});

await test("plain subscribe fires synchronously on commit", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/"),
  });
  const seen = [];
  const unsub = router.subscribe((r) => seen.push(r.path));
  await router.push("/x");
  assert.deepStrictEqual(seen, ["/x"]);
  unsub();
  await router.push("/y");
  assert.deepStrictEqual(seen, ["/x"], "no callback after unsubscribe");
});

// -- outlet mount / unmount + params-as-props --------------------------------

await test("outlet mounts the matched component, passing params as props", async () => {
  const hits = [];
  const routes = [
    { path: "/users/:id", component: makeComponent("user", hits) },
    { path: "/about", component: makeComponent("about", hits) },
  ];
  const router = createRouter(routes, { history: memoryHistory("/users/7") });

  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  const outlet = routerOutlet(c, anchor, router);

  // The user page is mounted before the anchor with its id prop.
  assert.strictEqual(parent.childNodes.length, 2, "component + anchor");
  assert.strictEqual(parent.childNodes[0].getAttribute("data-page"), "user");
  assert.strictEqual(hits.length, 1);
  assert.deepStrictEqual(hits[0].props, { id: "7" });

  const firstRoot = parent.childNodes[0];
  await router.push("/about");
  assert.strictEqual(parent.childNodes[0].getAttribute("data-page"), "about",
    "swapped to the about page");
  assert.strictEqual(firstRoot.parentNode, null, "old page unmounted (removed)");
  assert.strictEqual(hits.length, 2);

  outlet.destroy();
  assert.strictEqual(parent.childNodes.length, 1, "destroy unmounts, anchor remains");
});

await test("outlet keeps the same component mounted across param-only changes", async () => {
  const hits = [];
  const routes = [
    { path: "/users/:id", component: makeComponent("user", hits) },
  ];
  const router = createRouter(routes, { history: memoryHistory("/users/1") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  routerOutlet(c, anchor, router);

  const root1 = parent.childNodes[0];
  assert.strictEqual(hits.length, 1);
  await router.push("/users/2"); // same route def, different param
  assert.strictEqual(parent.childNodes[0], root1, "component instance reused");
  assert.strictEqual(hits.length, 1, "no re-mount for a param-only change");
});

await test("outlet 404: catch-all mounts, no match leaves the anchor empty", async () => {
  const hits = [];
  const routes = [
    { path: "/home", component: makeComponent("home", hits) },
    { path: "*", component: makeComponent("notfound", hits) },
  ];
  const router = createRouter(routes, { history: memoryHistory("/does/not/exist") });
  const parent = document.createElement("div");
  const anchor = anchorAppend(parent);
  const c = createContext(parent);
  routerOutlet(c, anchor, router);
  assert.strictEqual(parent.childNodes[0].getAttribute("data-page"), "notfound",
    "catch-all 404 component mounted");

  // A router with NO catch-all: a miss mounts nothing.
  const router2 = createRouter(
    [{ path: "/home", component: makeComponent("home") }],
    { history: memoryHistory("/missing") }
  );
  const p2 = document.createElement("div");
  const a2 = anchorAppend(p2);
  routerOutlet(createContext(p2), a2, router2);
  assert.strictEqual(p2.childNodes.length, 1, "only the anchor, nothing mounted");
  assert.strictEqual(router2.current.matched, null);
});

// -- navigation guards -------------------------------------------------------

await test("beforeEach (sync) cancels navigation when it returns false", async () => {
  let allow = true;
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/start"),
    beforeEach: (to) => allow || to.path === "/start",
  });
  const ok = await router.push("/blocked");
  allow = false;
  const ok2 = await router.push("/also-blocked");
  assert.strictEqual(ok, true);
  assert.strictEqual(router.current.path, "/blocked");
  assert.strictEqual(ok2, false, "guard returned false");
  assert.strictEqual(router.current.path, "/blocked", "route unchanged after cancel");
});

await test("beforeEach (async) supported; false cancels, true commits", async () => {
  const gate = { pass: false };
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/"),
    beforeEach: (to) =>
      new Promise((res) => setTimeout(() => res(gate.pass), 0)),
  });
  const denied = await router.push("/secret");
  assert.strictEqual(denied, false);
  assert.strictEqual(router.current.path, "/");
  gate.pass = true;
  const allowed = await router.push("/secret");
  assert.strictEqual(allowed, true);
  assert.strictEqual(router.current.path, "/secret");
});

await test("beforeEach receives (to, from)", async () => {
  const seen = [];
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/a"),
    beforeEach: (to, from) => {
      seen.push([from.path, to.path]);
      return true;
    },
  });
  await router.push("/b");
  await router.push("/c");
  assert.deepStrictEqual(seen, [["/a", "/b"], ["/b", "/c"]]);
});

// -- links -------------------------------------------------------------------

await test("routerLink: plain click navigates, modified click falls through", async () => {
  const router = createRouter([{ path: "*", component: null }], {
    history: memoryHistory("/"),
  });
  const a = document.createElement("a");
  // The minimal dom-shim omits removeEventListener; add it locally so we can
  // verify routerLink's returned unbind() actually detaches (browsers have it).
  a.removeEventListener = function (ev, fn) {
    const l = this._listeners[ev];
    if (l) {
      const i = l.indexOf(fn);
      if (i >= 0) l.splice(i, 1);
    }
  };
  const unbind = routerLink(a, router, "/target");

  // Plain primary click: navigates, preventDefault called.
  let prevented = false;
  a.dispatch("click"); // dom-shim dispatch sends { type } — button undefined => primary
  await tick();
  assert.strictEqual(router.current.path, "/target");

  // Modified click: falls through (no navigation).
  a.addEventListener("click", () => {});
  // Simulate a meta-click by dispatching a custom event object.
  for (const fn of a._listeners.click) fn({ type: "click", metaKey: true, preventDefault() { prevented = true; } });
  await tick();
  assert.strictEqual(router.current.path, "/target", "meta-click did not navigate");
  assert.strictEqual(prevented, false, "preventDefault not called for modified click");

  unbind();
  a.dispatch("click");
  await tick();
  assert.strictEqual(router.current.path, "/target", "no navigation after unbind");
});

console.log("router.test.mjs: all " + passed + " tests passed");
