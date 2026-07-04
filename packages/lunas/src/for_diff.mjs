// for_diff.mjs — the Lunas keyed :for reconciler (update phase).
// Same algorithm as the reference implementation in runtime-proto/for_diff.mjs
// (which carries the standalone test suite); only this header differs. Spec:
// crates/lunas_compiler/docs/for-diff-design.md
//
// This is the exact algorithm the real runtime `forBlock` update path will run.
// It is host-abstracted so it can be tested without a DOM: the caller supplies a
// `host` implementing the minimal node-placement interface, plus factory and
// patch callbacks.
//
// Compatibility: pure ES2015+ (arrow fns, const/let, Map, for-of). No BigInt,
// no optional chaining, no spread-into-typed-array. Runs on any ES2015 engine
// with Proxy — the same floor as the rest of the Lunas runtime.
//
// -----------------------------------------------------------------------------
// Host interface (what the DOM binding provides at runtime)
// -----------------------------------------------------------------------------
//
//   host.insertBefore(node, refNode)
//       Move/insert `node` so it sits immediately before `refNode` in the
//       parent. `refNode === null` means "append at the end" (before the
//       permanent `:for` anchor, in the real runtime). `node` may already be
//       in the parent (a move) or brand new (an insert).
//
//   host.remove(node)
//       Detach `node` from the parent.
//
//   makeItem(itemData, key) -> node
//       Build the DOM (or handle) for a new item and return its host node.
//       In the runtime this parses the item's own innerHTML branch, wires its
//       binds, and returns the single-root node or a multi-root handle.
//
//   patchItem(node, itemData)  [optional]
//       Update an existing item's reactive scope with new data (same key,
//       possibly new value/index). No-op if omitted.
//
// The reconciler NEVER calls innerHTML. It only reorders/creates/removes items.
//
// -----------------------------------------------------------------------------
// State object
// -----------------------------------------------------------------------------
//
// `createForState()` returns the mutable state the reconciler threads across
// updates. It holds, in current DOM order:
//   keys:  Array<key>            — key of each mounted item
//   nodes: Array<node>           — host node/handle for each mounted item
//   data:  Array<itemData>       — last data each item was rendered with
//
// The initial bulk-innerHTML render (the fast path, done elsewhere) must call
// `seedForState(state, keys, nodes, data)` once so the reconciler knows the
// starting order. After that, every array change calls `reconcile(...)`.

export function createForState() {
  return { keys: [], nodes: [], data: [] };
}

export function seedForState(state, keys, nodes, data) {
  state.keys = keys.slice();
  state.nodes = nodes.slice();
  state.data = data.slice();
}

// Default key extractor: identity of the datum. The compiler replaces this with
// the compiled `:key` expression, or with index when no key is available.
const defaultKeyOf = (d) => d;

// Sentinel used inside the LIS pass.
const NADA = -1;

// -----------------------------------------------------------------------------
// Duplicate-key detection + index fallback
// -----------------------------------------------------------------------------
//
// Keyed diffing requires keys to be unique. If the caller's key function yields
// a collision within a single update, keyed identity is meaningless for the
// colliding items, so for THAT update we fall back to positional (index) keys
// for the whole new list. This is deterministic and never corrupts order; it
// only loses the "reuse by identity" optimization. A warning is surfaced via
// the optional `onWarn` hook.
//
// Returns { keys, duped }.
function extractKeys(items, keyOf, onWarn) {
  const n = items.length;
  const keys = new Array(n);
  const seen = new Map(); // key -> first index, for the warning
  let duped = false;
  for (let i = 0; i < n; i++) {
    const k = keyOf(items[i], i);
    if (seen.has(k)) {
      duped = true;
      if (onWarn) {
        onWarn(
          "lunas :for — duplicate key " +
            stringifyKey(k) +
            " at positions " +
            seen.get(k) +
            " and " +
            i +
            "; falling back to index keys for this update (move/state reuse disabled)."
        );
      }
      break;
    }
    seen.set(k, i);
    keys[i] = k;
  }
  if (duped) {
    for (let i = 0; i < n; i++) keys[i] = i; // positional keys
  }
  return { keys, duped };
}

function stringifyKey(k) {
  try {
    return typeof k === "string" ? JSON.stringify(k) : String(k);
  } catch (_e) {
    return "<key>";
  }
}

// -----------------------------------------------------------------------------
// reconcile — the update phase
// -----------------------------------------------------------------------------
//
// Given the previous mounted order (in `state`) and the new `items`, mutate the
// host so its children end up exactly in the new key order, using:
//   1. prefix/suffix trimming to skip untouched ends in O(1) each,
//   2. a key->index map over the remaining "middle" of the old list,
//   3. remove for keys gone, create for keys new, patch for keys kept,
//   4. a longest-increasing-subsequence pass so the *minimum* number of kept
//      nodes are moved (nodes on the LIS stay put; everything else is inserted
//      into place).
//
// After it returns, `state` reflects the new order.
//
// opts: { keyOf, patchItem, onWarn } — all optional.
export function reconcile(state, host, items, makeItem, opts) {
  opts = opts || {};
  const keyOf = opts.keyOf || defaultKeyOf;
  const patchItem = opts.patchItem || null;
  const onWarn = opts.onWarn || null;

  const oldKeys = state.keys;
  const oldNodes = state.nodes;
  const oldData = state.data;

  const dup = extractKeys(items, keyOf, onWarn);
  const newKeys = dup.keys;
  // When we fell back to index keys, the OLD keys were built with the same
  // extractor on the previous update. To keep identity comparisons meaningful
  // under fallback we treat old items positionally too (index keys), so kept /
  // moved logic degrades to a stable positional reconcile rather than mixing
  // key spaces. We rebuild oldKeys positionally in that case.
  const effOldKeys =
    dup.duped && oldKeys.length ? oldKeys.map((_k, i) => i) : oldKeys;

  const oldLen = effOldKeys.length;
  const newLen = newKeys.length;

  // Fast total-empty transitions.
  if (newLen === 0) {
    for (let i = 0; i < oldLen; i++) host.remove(oldNodes[i]);
    state.keys = [];
    state.nodes = [];
    state.data = [];
    return;
  }
  if (oldLen === 0) {
    // Everything is new. Append in order (refNode = null => before anchor).
    const nodes = new Array(newLen);
    for (let i = 0; i < newLen; i++) {
      const node = makeItem(items[i], newKeys[i]);
      host.insertBefore(node, null);
      nodes[i] = node;
    }
    state.keys = newKeys;
    state.nodes = nodes;
    state.data = items.slice();
    return;
  }

  // ---- (a) trim common prefix ------------------------------------------------
  let start = 0;
  const minLen = oldLen < newLen ? oldLen : newLen;
  while (
    start < minLen &&
    sameKey(effOldKeys[start], newKeys[start])
  ) {
    if (patchItem) patchItem(oldNodes[start], items[start]);
    start++;
  }

  // ---- trim common suffix ----------------------------------------------------
  let oldEnd = oldLen - 1;
  let newEnd = newLen - 1;
  while (
    oldEnd >= start &&
    newEnd >= start &&
    sameKey(effOldKeys[oldEnd], newKeys[newEnd])
  ) {
    if (patchItem) patchItem(oldNodes[oldEnd], items[newEnd]);
    oldEnd--;
    newEnd--;
  }

  // The final node array we are building (length newLen). Prefix/suffix slots
  // are filled with the retained old nodes; the middle is filled below.
  const resultNodes = new Array(newLen);
  for (let i = 0; i < start; i++) resultNodes[i] = oldNodes[i];
  for (let i = newEnd + 1, j = oldEnd + 1; i < newLen; i++, j++)
    resultNodes[i] = oldNodes[j];

  // ---- pure-insert shortcut: old middle empty -------------------------------
  // Everything between [start..newEnd] is new; insert before the first suffix
  // node (or null if suffix empty).
  if (start > oldEnd) {
    const refNode =
      newEnd + 1 < newLen ? resultNodes[newEnd + 1] : null;
    for (let i = start; i <= newEnd; i++) {
      const node = makeItem(items[i], newKeys[i]);
      host.insertBefore(node, refNode);
      resultNodes[i] = node;
    }
    commit(state, newKeys, resultNodes, items);
    return;
  }

  // ---- pure-remove shortcut: new middle empty -------------------------------
  if (start > newEnd) {
    for (let i = start; i <= oldEnd; i++) host.remove(oldNodes[i]);
    commit(state, newKeys, resultNodes, items);
    return;
  }

  // ---- (b) key map over the old middle --------------------------------------
  // oldKey -> old index, for indices in [start..oldEnd].
  const oldIndexByKey = new Map();
  for (let i = start; i <= oldEnd; i++) oldIndexByKey.set(effOldKeys[i], i);

  const newMidLen = newEnd - start + 1;

  // For each new-middle position, the old index it maps to (or NADA if new).
  // Stored relative so LIS operates on the middle only.
  const newToOld = new Array(newMidLen);
  for (let i = 0; i < newMidLen; i++) newToOld[i] = NADA;

  // Track which old-middle nodes get reused, so leftovers can be removed.
  let patched = 0;
  const oldUsed = new Array(oldEnd - start + 1);
  for (let i = 0; i < oldUsed.length; i++) oldUsed[i] = false;

  for (let ni = start; ni <= newEnd; ni++) {
    const rel = ni - start;
    const k = newKeys[ni];
    const oi = oldIndexByKey.has(k) ? oldIndexByKey.get(k) : NADA;
    if (oi === NADA) {
      // brand-new key; node created in the move/insert pass below.
      newToOld[rel] = NADA;
    } else {
      newToOld[rel] = oi;
      oldUsed[oi - start] = true;
      resultNodes[ni] = oldNodes[oi];
      if (patchItem) patchItem(oldNodes[oi], items[ni]);
      patched++;
    }
  }

  // ---- remove old-middle nodes whose key vanished ---------------------------
  if (patched < oldEnd - start + 1) {
    for (let oi = start; oi <= oldEnd; oi++) {
      if (!oldUsed[oi - start]) host.remove(oldNodes[oi]);
    }
  }

  // ---- (c) LIS over newToOld to minimize moves ------------------------------
  // Compute the longest increasing subsequence of the reused old indices. New
  // items (NADA) are never part of the LIS. Positions on the LIS are already in
  // correct relative order and must NOT be moved; everything else is inserted
  // into place (walking right-to-left so the reference node is already final).
  const lis = longestIncreasingSubsequence(newToOld); // relative indices, ascending
  let lisPtr = lis.length - 1;

  for (let rel = newMidLen - 1; rel >= 0; rel--) {
    const ni = start + rel;
    const k = newKeys[ni];
    // reference node = the already-final node to our right, or the first suffix
    // node, or null (before anchor).
    const refNode = ni + 1 < newLen ? resultNodes[ni + 1] : null;

    if (newToOld[rel] === NADA) {
      // new key -> create + insert
      const node = makeItem(items[ni], k);
      host.insertBefore(node, refNode);
      resultNodes[ni] = node;
    } else if (lisPtr >= 0 && lis[lisPtr] === rel) {
      // stays in place; consume the LIS marker
      lisPtr--;
    } else {
      // reused but out of order -> move into place
      host.insertBefore(resultNodes[ni], refNode);
    }
  }

  commit(state, newKeys, resultNodes, items);
}

function commit(state, keys, nodes, items) {
  state.keys = keys.slice();
  state.nodes = nodes.slice();
  state.data = items.slice();
}

// Key equality. Keys are primitives (string/number/bool) or object identity.
// Use SameValueZero-ish comparison so NaN keys (degenerate) still match
// themselves. Strict === is fine for the common primitive case; we special-case
// NaN to avoid a pathological infinite churn.
function sameKey(a, b) {
  return a === b || (a !== a && b !== b);
}

// -----------------------------------------------------------------------------
// Longest Increasing Subsequence
// -----------------------------------------------------------------------------
// Input: array `arr` of old-index values (ascending == in-order). Entries equal
// to NADA (-1, "new item") are skipped and never included. Output: array of
// POSITIONS into `arr` (ascending) forming a longest strictly-increasing run of
// the non-NADA values. O(n log n) patience-sorting with parent links.
//
// This is the classic Vue 3 formulation, adapted to skip NADA holes.
export function longestIncreasingSubsequence(arr) {
  const n = arr.length;
  const parent = new Array(n).fill(NADA);
  // `tails[len-1]` = position in arr of the smallest tail of an increasing
  // subsequence of length `len`.
  const tails = [];
  for (let i = 0; i < n; i++) {
    const v = arr[i];
    if (v === NADA) continue; // holes never participate

    // binary search: first tail whose value >= v (strictly increasing)
    let lo = 0;
    let hi = tails.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (arr[tails[mid]] < v) lo = mid + 1;
      else hi = mid;
    }
    if (lo > 0) parent[i] = tails[lo - 1];
    if (lo === tails.length) tails.push(i);
    else tails[lo] = i;
  }

  // reconstruct positions by following parent links from the last tail
  const len = tails.length;
  const result = new Array(len);
  let k = len === 0 ? NADA : tails[len - 1];
  for (let idx = len - 1; idx >= 0; idx--) {
    result[idx] = k;
    k = parent[k];
  }
  return result;
}
