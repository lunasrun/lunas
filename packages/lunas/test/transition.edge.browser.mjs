// transition.edge.browser.mjs — the browser-path half of
// transition.edge.test.mjs. Spawned as a child process so we can install a
// fake requestAnimationFrame BEFORE importing transition.mjs (read at import
// time). Exits 0 on success, throws (nonzero) on failure. Not a *.test.mjs
// file, so run-all skips it; transition.edge.test.mjs invokes it.

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";

installDom();

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
function settleFrames() {
  let guard = 0;
  while (rafQueue.length && guard++ < 10) drainRaf();
}

const { withTransition, runPhase } = await import("../src/transition.mjs");

const cls = (el) => el.classList.toArray().sort();

// --- runPhase leave: full class sequence (from/active -> active/to -> gone) --
{
  const el = document.createElement("div");
  let finished = false;
  runPhase(el, "fade", "leave", { duration: 10000 }, () => (finished = true));
  assert.deepEqual(cls(el), ["fade-leave-active", "fade-leave-from"]);
  settleFrames();
  assert.deepEqual(cls(el), ["fade-leave-active", "fade-leave-to"]);
  el.dispatch("transitionend");
  assert.equal(finished, true);
  assert.deepEqual(cls(el), []);
}

// --- cancel() before frame 1 stops the sequence and cleans up classes --------
{
  const el = document.createElement("div");
  let finished = false;
  const cancel = runPhase(el, "v", "enter", { duration: 10000 }, () => (finished = true));
  assert.deepEqual(cls(el), ["v-enter-active", "v-enter-from"]);
  cancel();
  assert.equal(finished, false, "cancel does not itself call done()");
  // cleanup() only ever removes `-active`/`-to` (mirroring the natural finish
  // path); `-from` was added at frame 0 and step2 (which swaps it for `-to`)
  // never ran since we cancelled before frame 1, so `-from` is left behind.
  assert.deepEqual(cls(el), ["v-enter-from"], "cancel cleans up active/to, leaves -from since step2 never ran");
  // A transitionend arriving after cancel must not re-trigger finish — cleanup()
  // already removed the transitionend listener.
  el.dispatch("transitionend");
  assert.equal(finished, false);
}

// --- multiple calls to the returned cancel() are safe (idempotent) ----------
{
  const el = document.createElement("div");
  const cancel = runPhase(el, "v", "enter", {}, () => {});
  cancel();
  assert.doesNotThrow(() => cancel());
}

// --- no duration specified: fallback timer defaults to 0ms -------------------
{
  const el = document.createElement("div");
  let finished = false;
  runPhase(el, "v", "enter", {}, () => (finished = true)); // no opts.duration
  settleFrames();
  await new Promise((r) => setTimeout(r, 5));
  assert.equal(finished, true, "missing duration still arms a 0ms fallback");
}

// --- withTransition.enter with a NODE ARRAY runs each node's own choreography ---
// (duration kept short and the phase settled+finished so no fallback timer is
// left dangling for a later settleFrames() call in this shared-rafQueue script
// to accidentally arm and never clear — see the finish() below.)
{
  const t = withTransition({ name: "grp", duration: 5 });
  const a = document.createElement("div");
  const b = document.createElement("div");
  t.enter([a, b], () => {});
  assert.deepEqual(cls(a), ["grp-enter-active", "grp-enter-from"]);
  assert.deepEqual(cls(b), ["grp-enter-active", "grp-enter-from"]);
  settleFrames();
  a.dispatch("transitionend");
  b.dispatch("transitionend");
}

// --- withTransition default class base is "v" when no name given ------------
{
  const t = withTransition({ duration: 5 });
  const el = document.createElement("div");
  t.enter(el, () => {});
  assert.deepEqual(cls(el), ["v-enter-active", "v-enter-from"]);
  settleFrames();
  el.dispatch("transitionend");
}

// --- withTransition.leave with a single node still waits for its finish -----
{
  const t = withTransition({ name: "x", duration: 5 });
  const el = document.createElement("div");
  let removed = false;
  t.leave(el, () => (removed = true)); // single node, not an array
  settleFrames();
  assert.equal(removed, false);
  el.dispatch("transitionend");
  assert.equal(removed, true);
}

console.log("transition.edge.browser.mjs: all assertions passed");
