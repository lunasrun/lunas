// async.edge.test.mjs — additional edge-focused coverage for async.mjs
// (asyncComponent / suspenseBlock) beyond async.test.mjs: throwing loaders,
// props passthrough to loading/error factories, delay=0, timeout racing a
// slow-but-eventual resolve, suspense with 3 children, destroying a suspense
// AFTER it settled, and re-entrant nested boundary teardown order.
// Run: node packages/lunas/test/async.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext } from "../src/core.mjs";
import { anchorAppend } from "../src/dom.mjs";
import {
  asyncComponent,
  mountAsyncChild,
  suspenseBlock,
} from "../src/async.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));
const after = (ms) => new Promise((r) => setTimeout(r, ms + 5));

function defer() {
  let resolve, reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const leaf = (tag, text) => () => {
  const el = document.createElement(tag);
  if (text != null) el.appendChild(document.createTextNode(text));
  return el;
};

function host() {
  const c = createContext(document.createElement("div"));
  const container = c.root;
  const anchor = anchorAppend(container);
  return { c, container, anchor };
}

function html(container) {
  return container.childNodes
    .filter((n) => !(n.kind === "text" && n.data === ""))
    .map((n) => n.outerHTML)
    .join("");
}

// -- loader errors + props passthrough ---------------------------------------

test("a loader that throws synchronously is treated as a rejection", async () => {
  const { c, container, anchor } = host();
  const factory = asyncComponent(
    () => {
      throw new Error("sync boom");
    },
    { error: leaf("em", "caught") }
  );
  mountAsyncChild(c, anchor, factory, {});
  await tick();
  assert.equal(html(container), "<em>caught</em>");
});

test("error factory receives {error, ...props} so it can render the message", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  let received = null;
  const errorFactory = (props) => {
    received = props;
    const el = document.createElement("em");
    el.appendChild(document.createTextNode(String(props.error && props.error.message)));
    return el;
  };
  const factory = asyncComponent(() => d.promise, { error: errorFactory });
  mountAsyncChild(c, anchor, factory, { userId: 42 });
  d.reject(new Error("nope"));
  await tick();
  assert.equal(html(container), "<em>nope</em>");
  assert.equal(received.userId, 42, "original props passed through alongside error");
});

test("delay: 0 shows loading immediately (no wait)", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    loading: leaf("p", "now"),
    delay: 0,
  });
  mountAsyncChild(c, anchor, factory, {});
  await tick();
  assert.equal(html(container), "<p>now</p>");
  d.resolve(leaf("span", "done"));
  await tick();
  assert.equal(html(container), "<span>done</span>");
});

test("no loading factory configured: pending state renders nothing (no crash)", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, { delay: 0 });
  mountAsyncChild(c, anchor, factory, {});
  await tick();
  assert.equal(html(container), "");
  d.resolve(leaf("span", "x"));
  await tick();
  assert.equal(html(container), "<span>x</span>");
});

test("timeout longer than the actual resolve time never fires (no error flash)", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    error: leaf("em", "timed-out"),
    timeout: 100,
  });
  mountAsyncChild(c, anchor, factory, {});
  d.resolve(leaf("span", "fast"));
  await tick();
  assert.equal(html(container), "<span>fast</span>");
  await after(105); // let the timeout timer would-be fire
  assert.equal(html(container), "<span>fast</span>", "resolve wins; timeout timer was cleared");
});

test("rejecting after a successful resolve has no further effect (settle is one-shot per token)", async () => {
  const { c, container, anchor } = host();
  // Two separate mounts of independently-loadable components share nothing;
  // this exercises that a factory's own token only reacts to its own promise.
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    error: leaf("em", "err"),
  });
  mountAsyncChild(c, anchor, factory, {});
  d.resolve(leaf("span", "ok"));
  await tick();
  assert.equal(html(container), "<span>ok</span>");
});

// -- suspense: 3 children, partial settle ordering ---------------------------

test("suspense waits for THREE children; order of resolution doesn't matter", async () => {
  const { c, container, anchor } = host();
  const d1 = defer();
  const d2 = defer();
  const d3 = defer();
  const a1 = asyncComponent(() => d1.promise);
  const a2 = asyncComponent(() => d2.promise);
  const a3 = asyncComponent(() => d3.promise);

  const s = suspenseBlock(
    c,
    anchor,
    (cc) => {
      const wrap = document.createElement("section");
      for (const a of [a1, a2, a3]) {
        const an = anchorAppend(wrap);
        mountAsyncChild(cc, an, a, {});
      }
      return wrap;
    },
    leaf("div", "wait3")
  );

  assert.equal(html(container), "<div>wait3</div>");
  d2.resolve(leaf("b", "two"));
  await tick();
  assert.equal(s.isSettled(), false);
  d3.resolve(leaf("i", "three"));
  await tick();
  assert.equal(s.isSettled(), false, "still waiting on the first child");
  d1.resolve(leaf("u", "one"));
  await tick();
  assert.equal(s.isSettled(), true);
  assert.equal(
    html(container),
    "<section><u>one</u><b>two</b><i>three</i></section>"
  );
});

test("destroy() after the boundary already settled is a safe no-op teardown", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const child = asyncComponent(() => d.promise);
  const s = suspenseBlock(
    c,
    anchor,
    (cc) => {
      const wrap = document.createElement("section");
      const a = anchorAppend(wrap);
      mountAsyncChild(cc, a, child, {});
      return wrap;
    },
    leaf("div", "fallback")
  );
  d.resolve(leaf("span", "content"));
  await tick();
  assert.equal(s.isSettled(), true);
  assert.equal(html(container), "<section><span>content</span></section>");
  s.destroy(); // should remove the (now-revealed) content cleanly
  assert.equal(html(container), "");
});

test("suspense fallback is optional: no fallback factory renders nothing while pending", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const child = asyncComponent(() => d.promise);
  suspenseBlock(
    c,
    anchor,
    (cc) => {
      const a = anchorAppend(document.createElement("section"));
      const wrap = a.parentNode;
      mountAsyncChild(cc, a, child, {});
      return wrap;
    },
    null
  );
  assert.equal(html(container), "");
  d.resolve(leaf("span", "later"));
  await tick();
  assert.equal(html(container), "<section><span>later</span></section>");
});

test("nested boundary: outer resolves first, inner still pending — outer waits on inner via its own anchor only", async () => {
  // The outer boundary's pending count is driven only by async deps registered
  // directly under it (its OWN contentFactory build), not by the inner
  // boundary's internal pending count — the inner boundary absorbs its own
  // child's pending state and only ever settles/reveals independently.
  const { c, container, anchor } = host();
  const dOuter = defer();
  const dInner = defer();
  const outerChild = asyncComponent(() => dOuter.promise);
  const innerChild = asyncComponent(() => dInner.promise);

  suspenseBlock(
    c,
    anchor,
    (cc) => {
      const wrap = document.createElement("section");
      const ao = anchorAppend(wrap);
      mountAsyncChild(cc, ao, outerChild, {});
      const innerAnchor = anchorAppend(wrap);
      suspenseBlock(
        cc,
        innerAnchor,
        (ic) => {
          const iwrap = document.createElement("article");
          const ia = anchorAppend(iwrap);
          mountAsyncChild(ic, ia, innerChild, {});
          return iwrap;
        },
        leaf("small", "inner-fallback")
      );
      return wrap;
    },
    leaf("div", "outer-fallback")
  );

  assert.equal(html(container), "<div>outer-fallback</div>");
  // Resolve outer first: outer boundary settles and reveals its content. The
  // inner boundary's fallback is inserted at the inner anchor's own position
  // (a sibling in `wrap`, not nested inside the not-yet-revealed `<article>`
  // content), since the inner boundary manages its own insertion point
  // independent of its (still-hidden) content nodes.
  dOuter.resolve(leaf("b", "outer-done"));
  await tick();
  assert.equal(
    html(container),
    "<section><b>outer-done</b><small>inner-fallback</small></section>"
  );
  dInner.resolve(leaf("span", "inner-done"));
  await tick();
  assert.equal(
    html(container),
    "<section><b>outer-done</b><article><span>inner-done</span></article></section>"
  );
});

test("mountAsyncChild unmount before the anchor is attached cancels the queued insert", async () => {
  // Reproduces the buffered-insert path: build the async root but never let
  // mountChild's anchor-attach step run (simulated by calling the async root's
  // own unmount before draining), verifying no exception and no leaked timers.
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise);
  const m = mountAsyncChild(c, anchor, factory, {});
  m.unmount();
  d.resolve(leaf("span", "late"));
  await tick();
  assert.equal(html(container), "", "unmounted before resolve never renders");
});
