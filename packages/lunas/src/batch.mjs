// batch.mjs — update coalescing and nextTick.
// See output-design.md §4 (microtask flush) and §5.
//
// Coalescing is already the default: markVar schedules a single microtask
// flush, so N synchronous writes across any number of boxes produce one DOM
// update pass. This module exposes that guarantee to user code:
//
//   - nextTick(c)  — a Promise resolved AFTER the pending flush (DOM updated).
//   - batch(c, fn) — run fn, then flush synchronously, so a group of writes is
//                    applied before batch() returns instead of next microtask.

import { flush, afterFlush } from "./core.mjs";

// nextTick(c) — resolves after the next flush completes. If nothing is pending
// the flush is still scheduled, so `await nextTick(c)` always lands after the
// current tick's update pass. Callback ordering matches registration order, so
// two nextTick calls resolve in the order they were made.
export function nextTick(c) {
  return new Promise((resolve) => afterFlush(c, resolve));
}

// batch(c, fn) — run fn (which may write many boxes) and flush synchronously
// afterward, collapsing the whole group into one update pass that has completed
// by the time batch() returns. Useful for handlers that must read the updated
// DOM immediately. Nested batches (on the same context) flush only at the
// outermost call. The microtask that markVar scheduled becomes a cheap no-op
// (empty queue). Depth is tracked per context so batches on different contexts
// don't suppress each other's flush.
export function batch(c, fn) {
  c.batchDepth = (c.batchDepth || 0) + 1;
  try {
    return fn();
  } finally {
    c.batchDepth--;
    if (c.batchDepth === 0) flush(c);
  }
}
