// transition.browser.mjs — the browser-path half of transition.test.mjs.
// Spawned as a child process so we can install a fake requestAnimationFrame
// BEFORE importing transition.mjs (which reads rAF at import time). Exits 0 on
// success, throws (nonzero) on failure. Not a *.test.mjs file, so run-all skips
// it; the parent transition.test.mjs invokes it.

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";

installDom();

// Fake rAF: queue callbacks; `drainFrames(n)` runs n frames. transition.mjs
// double-rafs (rAF inside rAF), so one logical "next frame" = draining twice.
let rafQueue = [];
globalThis.requestAnimationFrame = (fn) => {
  rafQueue.push(fn);
  return rafQueue.length;
};
function drainRaf() {
  const q = rafQueue;
  rafQueue = [];
  for (const fn of q) fn();
}
// Drain until the double-raf resolves (queue empties or a bounded number of hops).
function settleFrames() {
  let guard = 0;
  while (rafQueue.length && guard++ < 10) drainRaf();
}

const { withTransition, runPhase } = await import("../src/transition.mjs");

const cls = (el) => el.classList.toArray().sort();

// --- runPhase enter: frame 0 classes, frame 1 swap, transitionend finish -----
{
  const el = document.createElement("div");
  let finished = false;
  runPhase(el, "fade", "enter", { duration: 10000 }, () => (finished = true));
  // frame 0 (synchronous): from + active present, to absent.
  assert.deepEqual(cls(el), ["fade-enter-active", "fade-enter-from"]);
  assert.equal(finished, false);

  settleFrames(); // advance the double-raf → step2 swaps from→to
  assert.deepEqual(cls(el), ["fade-enter-active", "fade-enter-to"]);
  assert.equal(finished, false);

  // transitionend on the element itself completes the phase.
  el.dispatch("transitionend");
  assert.equal(finished, true);
  // cleanup removed active + to.
  assert.deepEqual(cls(el), []);
}

// --- runPhase ignores transitionend bubbled from a child --------------------
{
  const el = document.createElement("div");
  const child = document.createElement("span");
  el.appendChild(child);
  let finished = false;
  runPhase(el, "v", "leave", { duration: 10000 }, () => (finished = true));
  settleFrames();
  // event target = child, not el → ignored.
  el.dispatch("transitionend", { target: child });
  assert.equal(finished, false);
  el.dispatch("transitionend"); // target = el → finishes
  assert.equal(finished, true);
}

// --- timeout fallback finishes when transitionend never fires ---------------
{
  const el = document.createElement("div");
  let finished = false;
  // duration 0 → setTimeout(…, 0). Await a macrotask.
  runPhase(el, "v", "enter", { duration: 0 }, () => (finished = true));
  settleFrames();
  await new Promise((r) => setTimeout(r, 5));
  assert.equal(finished, true);
}

// --- withTransition.leave waits for ALL nodes before removing ---------------
{
  const t = withTransition({ name: "slide", duration: 10000 });
  const a = document.createElement("div");
  const b = document.createElement("div");
  let removed = false;
  t.leave([a, b], () => (removed = true));
  settleFrames();
  a.dispatch("transitionend");
  assert.equal(removed, false); // b still pending
  b.dispatch("transitionend");
  assert.equal(removed, true);
}

// --- withTransition.enter inserts then animates -----------------------------
{
  const t = withTransition({ name: "pop", duration: 10000 });
  const el = document.createElement("div");
  let inserted = false;
  t.enter(el, () => (inserted = true));
  assert.equal(inserted, true);
  // frame 0 classes present.
  assert.deepEqual(cls(el), ["pop-enter-active", "pop-enter-from"]);
}

console.log("transition.browser.mjs: all assertions passed");
