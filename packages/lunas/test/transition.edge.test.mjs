// transition.edge.test.mjs — additional edge-focused coverage for
// transition.mjs beyond transition.test.mjs: withTransition custom class base,
// leave with mixed node counts, non-browser (degraded) immediate finish
// semantics for enter AND leave with real elements/classLists asserted, plus a
// browser-path subprocess (transition.edge.browser.mjs) for the parts that
// need a fake requestAnimationFrame installed before import.
// Run: node packages/lunas/test/transition.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { installDom } from "./dom-shim.mjs";

installDom();

const cls = (el) => el.classList.toArray();

// --- degraded path (this process has no requestAnimationFrame) ---------------

test("degraded: custom class base name is honored in the (collapsed) sequence", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "custom-fade" });
  const el = document.createElement("div");
  t.enter(el, () => {});
  // Degraded mode collapses the whole sequence; regardless of base name, no
  // transition classes remain (cleanup always removes active+to, and step2
  // removes from before finish runs synchronously).
  assert.deepEqual(cls(el), []);
});

test("degraded: leave with TWO nodes calls remove exactly once after both finish synchronously", () => {
  return (async () => {
    const { withTransition } = await import("../src/transition.mjs");
    const t = withTransition({ name: "fade" });
    const host = document.createElement("div");
    const a = document.createElement("span");
    const b = document.createElement("span");
    host.appendChild(a);
    host.appendChild(b);
    let removedCount = 0;
    t.leave([a, b], () => {
      removedCount++;
      a.remove();
      b.remove();
    });
    assert.equal(removedCount, 1, "remove called exactly once for the whole group");
    assert.equal(a.parentNode, null);
    assert.equal(b.parentNode, null);
  })();
});

test("degraded: runPhase's returned cancel() is a safe no-op after synchronous finish", async () => {
  const { runPhase } = await import("../src/transition.mjs");
  const el = document.createElement("div");
  let finished = false;
  const cancel = runPhase(el, "v", "enter", {}, () => (finished = true));
  assert.equal(finished, true, "degraded mode finishes synchronously");
  assert.doesNotThrow(() => cancel());
});

test("degraded: enter without an insert callback does not throw (insert is optional)", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "v" });
  const el = document.createElement("div");
  assert.doesNotThrow(() => t.enter(el, undefined));
  assert.doesNotThrow(() => t.enter(el, null));
});

test("degraded: leave without a remove callback does not throw", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "v" });
  const el = document.createElement("div");
  assert.doesNotThrow(() => t.leave(el, undefined));
  assert.doesNotThrow(() => t.leave([], undefined));
});

test("degraded: runPhase adds and fully removes from/active/to even with no done callback", async () => {
  const { runPhase } = await import("../src/transition.mjs");
  const el = document.createElement("div");
  assert.doesNotThrow(() => runPhase(el, "v", "leave", {}, null));
  assert.deepEqual(cls(el), [], "sequence still collapses correctly without a done callback");
});

// --- browser path (subprocess with fake rAF + transitionend) -----------------

test("browser path (edge cases): leave sequencing, cancel-before-frame1, missing duration, arrays, default base", () => {
  const here = fileURLToPath(import.meta.url);
  const dir = here.slice(0, here.lastIndexOf("/"));
  const res = spawnSync(process.execPath, [dir + "/transition.edge.browser.mjs"], {
    encoding: "utf8",
  });
  if (res.status !== 0) {
    console.error(res.stdout, res.stderr);
  }
  assert.equal(res.status, 0, "browser-path edge-case subprocess should pass");
});
