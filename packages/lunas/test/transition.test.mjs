// transition.test.mjs — CSS-class enter/leave sequencing + degradation.
// Run: node packages/lunas/test/transition.test.mjs
//
// The transition module reads `requestAnimationFrame` at IMPORT time to pick
// its browser vs. degraded path. So the two paths are exercised in separate
// child processes: this file runs the degraded (non-browser) path directly, and
// spawns a subprocess with a fake rAF installed for the browser path.

import assert from "node:assert";
import { test } from "node:test";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { installDom } from "./dom-shim.mjs";

installDom();

const cls = (el) => el.classList.toArray();

// --- degraded path (this process has no requestAnimationFrame) ---------------

test("degraded: enter runs the class sequence synchronously and finishes", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "fade" });
  const el = document.createElement("div");
  let inserted = false;
  t.enter(el, () => {
    inserted = true;
  });
  assert.equal(inserted, true); // insert ran
  // In degraded mode the whole sequence collapses: from removed, to removed,
  // active removed → element ends with NO transition classes.
  assert.deepEqual(cls(el), []);
});

test("degraded: leave removes the node immediately (no CSS engine)", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "fade" });
  const host = document.createElement("div");
  const el = document.createElement("span");
  host.appendChild(el);
  let removed = false;
  t.leave(el, () => {
    removed = true;
    el.remove();
  });
  assert.equal(removed, true);
  assert.equal(el.parentNode, null);
});

test("degraded: leave with empty node list still calls remove once", async () => {
  const { withTransition } = await import("../src/transition.mjs");
  const t = withTransition({ name: "x" });
  let n = 0;
  t.leave([], () => n++);
  assert.equal(n, 1);
});

// --- browser path (subprocess with fake rAF + transitionend) -----------------

test("browser path: class choreography over frames + transitionend", () => {
  const here = fileURLToPath(import.meta.url);
  const dir = here.slice(0, here.lastIndexOf("/"));
  const res = spawnSync(process.execPath, [dir + "/transition.browser.mjs"], {
    encoding: "utf8",
  });
  if (res.status !== 0) {
    console.error(res.stdout, res.stderr);
  }
  assert.equal(res.status, 0, "browser-path subprocess should pass");
});
