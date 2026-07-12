// transition.mjs — CSS-class enter/leave transitions (c-transition).
// See output-design.md §5 (runtime API).
//
// Vue-style class choreography around a block's insert/remove. For a transition
// named `n` an element gets, on ENTER:
//
//   frame 0 (before paint):  + n-enter-from   + n-enter-active
//   frame 1 (next raf):      − n-enter-from   + n-enter-to
//   transitionend / timeout: − n-enter-active − n-enter-to
//
// and symmetrically on LEAVE (`n-leave-from/-active/-to`), with the node removed
// only after the leave transition finishes. This lets CSS drive the animation
// while the runtime just toggles classes and awaits `transitionend`.
//
// ── Environment degradation ──────────────────────────────────────────────────
// There is no CSS engine outside a browser (and the test DOM has none), so when
// `requestAnimationFrame` is unavailable the transition degrades to an immediate
// insert/remove — but the class SEQUENCE still runs synchronously so the logic
// is testable: classes are added and removed in order, and `done` is called
// right away. In a browser the frames and `transitionend` (with a `duration`
// timeout fallback) drive the real timing.

const nextFrame =
  typeof requestAnimationFrame === "function"
    ? (fn) => requestAnimationFrame(() => requestAnimationFrame(fn))
    : null;

function addClass(el, cls) {
  if (el && el.classList) el.classList.add(cls);
}
function removeClass(el, cls) {
  if (el && el.classList) el.classList.remove(cls);
}

// runPhase(el, base, phase, opts, done) — perform one enter/leave choreography
// on a single element. `phase` is "enter" | "leave". Calls `done()` when the
// phase completes (after transitionend/timeout in a browser, synchronously when
// degraded). Returns a `cancel()` that stops timers/listeners.
export function runPhase(el, base, phase, opts, done) {
  opts = opts || {};
  const from = base + "-" + phase + "-from";
  const active = base + "-" + phase + "-active";
  const to = base + "-" + phase + "-to";
  let finished = false;
  let timer = null;
  let removeListener = null;

  const cleanup = () => {
    removeClass(el, active);
    removeClass(el, to);
    if (timer != null) clearTimeout(timer);
    if (removeListener) removeListener();
  };
  const finish = () => {
    if (finished) return;
    finished = true;
    cleanup();
    if (done) done();
  };

  // frame 0: starting classes.
  addClass(el, from);
  addClass(el, active);

  const step2 = () => {
    // frame 1: swap -from → -to to kick the transition.
    removeClass(el, from);
    addClass(el, to);
  };

  if (!nextFrame) {
    // Degraded (non-browser): run the whole sequence synchronously so class
    // ordering is observable, then finish immediately.
    step2();
    finish();
    return () => {};
  }

  // Browser path: advance a frame, then await transitionend with a timeout
  // fallback (some elements never fire it — e.g. display:none, 0-duration).
  nextFrame(() => {
    if (finished) return;
    step2();
    if (el && el.addEventListener) {
      const onEnd = (ev) => {
        if (ev && ev.target !== el) return; // ignore bubbled child transitions
        finish();
      };
      el.addEventListener("transitionend", onEnd);
      removeListener = () => el.removeEventListener("transitionend", onEnd);
    }
    const dur = opts.duration;
    // Always arm a fallback timer so a missing transitionend can't hang teardown.
    timer = setTimeout(finish, dur != null ? dur : 0);
  });

  return () => {
    if (!finished) {
      finished = true;
      cleanup();
    }
  };
}

// withTransition(opts) — build a transition controller composing with block
// insert/remove. `opts.name` is the class base (default "v"); `opts.duration`
// is the leave/enter timeout fallback in ms.
//
// Returns { enter(nodes, insert), leave(nodes, remove) }:
//   enter(nodes, insert) — call `insert()` to mount the nodes, then choreograph
//                          the enter classes on each root node.
//   leave(nodes, remove) — choreograph the leave classes, then call `remove()`
//                          once every node's leave phase has finished.
//
// This is the composable primitive ifBlock/forBlock/mountChild wrappers use:
// pass the block's own insert/remove closures through it.
export function withTransition(opts) {
  opts = opts || {};
  const base = opts.name || "v";

  const toList = (n) => (n == null ? [] : Array.isArray(n) ? n : [n]);

  return {
    enter(nodes, insert) {
      if (insert) insert();
      const list = toList(nodes);
      for (const el of list) runPhase(el, base, "enter", opts, null);
    },
    leave(nodes, remove) {
      const list = toList(nodes);
      if (list.length === 0) {
        if (remove) remove();
        return;
      }
      let pending = list.length;
      const one = () => {
        if (--pending === 0 && remove) remove();
      };
      for (const el of list) runPhase(el, base, "leave", opts, one);
    },
  };
}
