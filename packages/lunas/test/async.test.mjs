// async.test.mjs — asyncComponent + suspenseBlock against the fake DOM.
// Run: node packages/lunas/test/async.test.mjs
//
// Timing is exercised with controllable deferred promises (resolve when we say)
// and real short timers for delay/timeout semantics. `tick()` drains the
// microtask + flush queue; `after(ms)` waits past a real timer.

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

// A deferred: a promise plus its resolve/reject, so tests control settle timing.
function defer() {
  let resolve, reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

// A trivial child factory producing <tag>text</tag>.
const leaf = (tag, text) => () => {
  const el = document.createElement(tag);
  if (text != null) el.appendChild(document.createTextNode(text));
  return el;
};

// Build a host: a container element with a trailing anchor to mount into.
function host() {
  const c = createContext(document.createElement("div"));
  const container = c.root;
  const anchor = anchorAppend(container);
  return { c, container, anchor };
}

// Serialize a container's children (skips empty-text anchors).
function html(container) {
  return container.childNodes
    .filter((n) => !(n.kind === "text" && n.data === ""))
    .map((n) => n.outerHTML)
    .join("");
}

test("loader resolves → mounts the component", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise);
  mountAsyncChild(c, anchor, factory, {});
  assert.equal(html(container), ""); // nothing yet
  d.resolve(leaf("span", "hi"));
  await tick();
  assert.equal(html(container), "<span>hi</span>");
});

test("default export is unwrapped", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise);
  mountAsyncChild(c, anchor, factory, {});
  d.resolve({ default: leaf("b", "def") }); // ES module shape
  await tick();
  assert.equal(html(container), "<b>def</b>");
});

test("cache: second mount is synchronous", async () => {
  const { c, container, anchor } = host();
  let loads = 0;
  const d = defer();
  const factory = asyncComponent(() => {
    loads++;
    return d.promise;
  });
  mountAsyncChild(c, anchor, factory, {});
  d.resolve(leaf("i", "x"));
  await tick();
  assert.equal(html(container), "<i>x</i>");
  assert.equal(loads, 1);

  // Second mount at a fresh anchor: resolves synchronously from cache.
  const h2 = host();
  mountAsyncChild(h2.c, h2.anchor, factory, {});
  assert.equal(html(h2.container), "<i>x</i>"); // no await → already there
  assert.equal(loads, 1); // loader not called again
});

test("delay: loading not shown before delay, shown after", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    loading: leaf("p", "loading"),
    delay: 40,
  });
  mountAsyncChild(c, anchor, factory, {});
  await tick();
  assert.equal(html(container), ""); // before delay: no loading (avoid flash)
  await after(45);
  assert.equal(html(container), "<p>loading</p>"); // after delay
  d.resolve(leaf("span", "done"));
  await tick();
  assert.equal(html(container), "<span>done</span>");
});

test("delay: fast resolve never shows loading", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    loading: leaf("p", "loading"),
    delay: 40,
  });
  mountAsyncChild(c, anchor, factory, {});
  d.resolve(leaf("span", "fast"));
  await tick();
  assert.equal(html(container), "<span>fast</span>");
  await after(45); // delay timer should have been cleared
  assert.equal(html(container), "<span>fast</span>");
});

test("error component shown on rejection", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    error: leaf("em", "err"),
  });
  mountAsyncChild(c, anchor, factory, {});
  d.reject(new Error("boom"));
  await tick();
  assert.equal(html(container), "<em>err</em>");
});

test("timeout → error component", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    error: leaf("em", "timed-out"),
    timeout: 30,
  });
  mountAsyncChild(c, anchor, factory, {});
  await after(35);
  assert.equal(html(container), "<em>timed-out</em>");
  // A late resolve must not swap the error out.
  d.resolve(leaf("span", "late"));
  await tick();
  assert.equal(html(container), "<em>timed-out</em>");
});

test("suspense: fallback → content when the single async dep resolves", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const child = asyncComponent(() => d.promise);

  suspenseBlock(
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

  assert.equal(html(container), "<div>fallback</div>"); // pending → fallback
  d.resolve(leaf("span", "content"));
  await tick();
  assert.equal(html(container), "<section><span>content</span></section>");
});

test("suspense: counter waits for BOTH async children", async () => {
  const { c, container, anchor } = host();
  const d1 = defer();
  const d2 = defer();
  const a1 = asyncComponent(() => d1.promise);
  const a2 = asyncComponent(() => d2.promise);

  const s = suspenseBlock(
    c,
    anchor,
    (cc) => {
      const wrap = document.createElement("section");
      const an1 = anchorAppend(wrap);
      mountAsyncChild(cc, an1, a1, {});
      const an2 = anchorAppend(wrap);
      mountAsyncChild(cc, an2, a2, {});
      return wrap;
    },
    leaf("div", "wait")
  );

  assert.equal(html(container), "<div>wait</div>");
  d1.resolve(leaf("b", "one"));
  await tick();
  assert.equal(s.isSettled(), false); // one still pending
  assert.equal(html(container), "<div>wait</div>");
  d2.resolve(leaf("i", "two"));
  await tick();
  assert.equal(s.isSettled(), true);
  assert.equal(html(container), "<section><b>one</b><i>two</i></section>");
});

test("suspense: sync-resolved subtree never flashes fallback", async () => {
  const { c, container, anchor } = host();
  const child = asyncComponent(() => leaf("span", "sync")); // already a factory

  suspenseBlock(
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

  // pending was bumped then settled during build; content reveals on afterFlush
  // without the fallback ever entering the DOM.
  await tick();
  assert.equal(html(container), "<section><span>sync</span></section>");
});

test("suspense: content with no async deps reveals immediately", async () => {
  const { c, container, anchor } = host();
  const s = suspenseBlock(
    c,
    anchor,
    () => leaf("main", "static")(),
    leaf("div", "fallback")
  );
  assert.equal(s.isSettled(), true);
  assert.equal(html(container), "<main>static</main>");
});

test("nested boundaries settle independently", async () => {
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
      // outer async dep
      const ao = anchorAppend(wrap);
      mountAsyncChild(cc, ao, outerChild, {});
      // inner boundary with its own async dep + fallback
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

  // Resolve inner first: outer boundary is still pending on outerChild, so the
  // outer fallback stays. The inner boundary settled but its content is inside
  // the still-hidden outer content.
  dInner.resolve(leaf("span", "inner-done"));
  await tick();
  assert.equal(html(container), "<div>outer-fallback</div>");

  // Now resolve outer: outer content (which contains the already-revealed inner
  // article) is shown.
  dOuter.resolve(leaf("b", "outer-done"));
  await tick();
  assert.equal(
    html(container),
    "<section><b>outer-done</b><article><span>inner-done</span></article></section>"
  );
});

test("unmount while pending writes nothing", async () => {
  const { c, container, anchor } = host();
  const d = defer();
  const factory = asyncComponent(() => d.promise, {
    loading: leaf("p", "loading"),
    delay: 0,
  });
  const m = mountAsyncChild(c, anchor, factory, {});
  await tick();
  assert.equal(html(container), "<p>loading</p>"); // delay 0 → loading now
  m.unmount();
  assert.equal(html(container), ""); // gone
  d.resolve(leaf("span", "late"));
  await tick();
  await after(5);
  assert.equal(html(container), ""); // late resolve must not write DOM
});

test("destroy suspense while pending cancels async children", async () => {
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
  assert.equal(html(container), "<div>fallback</div>");
  s.destroy();
  assert.equal(html(container), "");
  d.resolve(leaf("span", "late"));
  await tick();
  assert.equal(html(container), ""); // nothing written after destroy
});
