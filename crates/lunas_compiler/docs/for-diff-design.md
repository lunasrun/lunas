# Keyed `:for` reconciliation design

This document specifies the update-path algorithm for `forBlock`, closing the
first item under "Open / deferred" in `output-design.md` ("Keyed-diff algorithm
for `:for` updates"). The **initial** render strategy is already settled there
(§8: one bulk `innerHTML` of all items concatenated); this document specifies
what happens on every subsequent change to the iterated array.

A DOM-free reference implementation lives in `runtime-proto/for_diff.mjs` with
its test suite in `runtime-proto/for_diff.test.mjs` (run with plain
`node runtime-proto/for_diff.test.mjs`). The algorithm there is **exactly** what
the real runtime `forBlock` update path executes; the only difference in the
runtime is that the abstract host is the real DOM (`parent.insertBefore`,
`node.remove()`) and `makeItem` builds the item from its compiled template.

Compatibility floor: **ES2015 + Proxy**, same as the rest of the runtime.
No `BigInt` anywhere (see `output-design.md` §4).

---

## 1. Runtime surface

```js
forBlock(c, anchor, deps, items, make /*, opts */)
```

- `c` — the component instance (for registering the block's own `bind` and for
  per-item scope bookkeeping, §6).
- `anchor` — the **permanent text node** that marks the end of the list slot
  (created at wiring time per `output-design.md` §8). All items live
  immediately before it; "append" means `insertBefore(node, anchor)`. Because
  the anchor never moves, the slot position is always known even when the list
  is empty.
- `deps` — the compile-time dependency indices of the iterated expression
  (adjacency dispatch; the reconcile function is enqueued only when one of
  these variables was written).
- `items` — a closure returning the **current** array (`() => history.v`).
  Reading it lazily at flush time means one flush sees the final state of any
  number of synchronous mutations.
- `make` — the compiled per-item factory: `make(itemData, index)` builds the
  item's DOM and wires its item-local binds, returning the item handle (§4).
- The compiler additionally passes (when present in the template):
  - `keyOf(itemData, index)` — compiled from the `:key` expression (§3),
  - `patch(handle, itemData, index)` — updates an existing item's local scope.

Internally `forBlock` is:

```js
export function forBlock(c, anchor, deps, items, make, opts) {
  const state = createForState();          // keys / handles / data, in DOM order
  initialRender(state, anchor, items(), make, opts);   // phase (a): bulk innerHTML
  bind(c, deps, () => reconcile(state, host(anchor), items(), make, opts)); // phase (b)
}
```

(The `bind`'s initial run is skipped for the reconcile path because
`initialRender` already produced the first paint; the generator emits the
seed + bind pair accordingly.)

---

## 2. Phases

### (a) Initial render — bulk `innerHTML` + per-item wiring

Benchmark-locked (see `output-design.md` §2): the fastest way to materialize N
items is **one** native parse.

1. Concatenate the static HTML of every item into a single string (item
   template string × N with per-item static interpolations resolved), assign it
   to a detached scratch element via `innerHTML` — one parse, comment-free,
   whitespace-free.
2. Walk the parsed children once, slicing them into per-item handles (§4):
   single-root items consume exactly one child each; multi-root items consume a
   compile-time-known count (or are delimited by start anchors emitted as text
   nodes at wiring time — never comments, which would kill the fast-path
   parser).
3. For each item, run the per-item wiring: positional-nav refs, `on(...)`
   listeners, item-local `bind`s, nested `ifBlock`/`forBlock` (all off-DOM).
4. Seed the reconciler state: `keys[i]`, `handles[i]`, `data[i]` in order.
5. Move all children before the permanent anchor in one `insertBefore` of a
   fragment (or during the single attach of the whole component if the initial
   render happens at mount).

This phase runs exactly once. **No later update ever re-runs it.**

### (b) Update — keyed diff (the subject of this document)

Re-`innerHTML`-ing the list on update is forbidden (`output-design.md` §8): it
would destroy element state, listeners, focus, selection, and any nested
component instances. Instead, `reconcile(state, host, newItems, make, opts)`
performs the minimal set of targeted mutations:

1. **Extract keys** for the new array (§3). Detect duplicates → fallback (§7).
2. **Empty fast paths.** `new = ∅` → remove everything (and tear down scopes,
   §6). `old = ∅` → build every item and append before the anchor, in order.
3. **Prefix trim.** Walk from the front while `oldKey[i] === newKey[i]`,
   patching data in place. Skips the untouched head in O(prefix).
4. **Suffix trim.** Same from the back. After trimming, only the changed
   "middle" window remains. Pure appends, pure prepends, and pure tail/head
   removals never reach the expensive machinery:
   - old middle empty → **pure insert** of the new middle before the first
     suffix node (or the anchor);
   - new middle empty → **pure remove** of the old middle.
5. **Key map.** Build `Map(oldKey → oldIndex)` over the old middle. For each
   new-middle position record `newToOld[i]` = matching old index, or −1 if the
   key is new. Matched items are **patched** with their new data/index; old
   items whose key vanished are **removed** (scope teardown, §6).
6. **LIS move minimization.** Compute the longest increasing subsequence of the
   non-(−1) values of `newToOld` (O(m log m) patience sorting, positions of the
   run returned). Old indices that appear in increasing order are already
   correctly ordered relative to each other — those nodes are **not touched**.
   Walk the new middle **right-to-left**; for each position the reference node
   is the already-final node to its right (or the first suffix node, or the
   anchor):
   - `newToOld[i] === −1` → `make` the item, `insertBefore(node, ref)`;
   - position is on the LIS → skip (zero DOM ops);
   - otherwise → `insertBefore(existingNode, ref)` (one move).

   Right-to-left order guarantees the reference node is always already in its
   final position, so each item needs at most one placement op.
7. Commit the new `keys`/`handles`/`data` arrays as the state for next time.

This is the Vue 3 / ivi family algorithm, adapted to Lunas's permanent-anchor
slot model and adjacency-dispatch reactivity.

### (c) Per-item reactive scope

See §6.

---

## 3. Key extraction

| Template | Key function | Semantics |
|---|---|---|
| `:for="item of items" :key="item.id"` | compiled `(item, i) => item.id` | full keyed identity: reuse, patch, and **move** items by key |
| `:for` without `:key` | `(item, i) => item` (identity of the datum) when items are objects; effectively index for primitives that repeat | reuse by value/reference identity; **moves are best-effort** |
| fallback (duplicates detected, §7) | `(item, i) => i` | positional reuse only; no move semantics |

- The `:key` expression is compiled like any other item-scope expression (it
  can reference the loop variable and the index binding from the `:for`
  header). It is evaluated once per item per update — never stored on the DOM.
- Keys should be primitives (string/number/bool) or stable object references.
  Comparison is `===` with a NaN-equals-NaN carve-out so a degenerate NaN key
  cannot cause infinite churn.
- **No `:key` ⇒ documented warning in the docs/lint layer**: without an
  explicit key, item identity follows datum identity, so replacing an object
  with an equal-but-not-identical object is a remove+insert (state such as
  focus or an `<input>`'s value is **not** carried across), and duplicate
  primitive values trigger the index fallback. The linter should suggest
  `:key` whenever the item template contains stateful elements (inputs,
  components, nested control flow).

---

## 4. Item representation (compile-time specialization)

The reconciler operates on opaque **handles**; what a handle is, is chosen
**per template at compile time** — the same philosophy as the rest of the
output design (`output-design.md` §8 "compiler picks"): the runtime never
inspects the template shape at runtime, because the compiler already knows it.

| Template shape | Handle | Cost |
|---|---|---|
| **Single root, fixed content** (`<li>…</li>`) | the root node itself | cheapest: `insertBefore(h, ref)` / `h.remove()` are one DOM op; zero bookkeeping |
| **Multi-root** (`:for` on a fragment of siblings) or **variable content** (item root is itself an `:if` chain that can swap its top-level node) | either (i) a per-item **node array** when the count is compile-time-fixed, or (ii) a **start/end anchor pair** (text nodes) when it isn't | move = insert the group before ref (last-to-first, or `range`-extract); remove = drop the group |

The generator emits one of two tiny host adapters per `:for` site:

```js
// single-root adapter (common case)
host = { insertBefore: (h, ref) => parent.insertBefore(h, ref || anchor),
         remove:       (h)      => h.remove() };

// group adapter (multi-root / variable-content)
host = { insertBefore: (h, ref) => { const r = ref ? first(ref) : anchor;
                                     for (const n of h.nodes) parent.insertBefore(n, r); },
         remove:       (h)      => { for (const n of h.nodes) n.remove(); } };
```

Because the adapter is selected at compile time there is **no per-operation
branch** ("is this item multi-root?") in the hot loop, and the single-root
common case pays nothing for the general case's existence. This is exactly the
compile-time-specialization bet Lunas makes everywhere else: ship the smallest
code that the *specific* template needs, not a general interpreter.

The reference implementation (`runtime-proto/for_diff.mjs`) is written against
the abstract `{ insertBefore, remove }` host precisely so both adapters (and
the test harness's validating fake) run the identical reconcile code.

---

## 5. Edge cases — exact behavior

| Case | Behavior | Ops |
|---|---|---|
| **∅ → N** | fast path: `make` each item, append before anchor in order | N inserts, 0 moves |
| **N → ∅** | fast path: remove every handle, tear down every item scope | N removes |
| **full reverse** | prefix/suffix trims match nothing; key map matches all; LIS length is 1, so all but one node move | ≤ N−1 moves, 0 inserts/removes (tested) |
| **adjacent swap** | trims isolate the 2-item middle; LIS keeps one; the other moves | ≤ 1 move (tested) |
| **prepend 1** | suffix trim matches everything; pure-insert shortcut | 1 insert, 0 moves (tested) |
| **append K** | prefix trim matches everything; pure-insert shortcut | K inserts, 0 moves |
| **middle remove** | trims isolate it; pure-remove shortcut | 1 remove, 0 moves (tested) |
| **moves mixed with inserts+removes** | key map partitions kept/new/gone; removes issued before the placement walk (shrinks the working set and frees scopes early); LIS over kept only | removes + inserts + (kept − LIS) moves (fuzz-tested, 500+ seeded transitions) |
| **duplicate keys** | warn once via the runtime warning hook; **whole update falls back to index keys** (§7); order is still exactly the target order; identity reuse and move semantics are disabled for that update only | deterministic; never throws, never corrupts order (tested) |
| **keyed item containing nested `:if` / `:for`** | the nested blocks live in the item's scope; a **move never touches them** (moving a subtree preserves all descendants, listeners, and anchors — the nested blocks' anchors travel with the item); a **remove** tears down the item scope, which unregisters the nested blocks' binds recursively (§6); patching only re-evaluates the item-local binds, and the nested blocks re-render only if their own deps changed | move = 1 op regardless of nesting depth |
| **same array identity, mutated in place** (`arr.push` via `deepBox`) | `items()` re-reads the proxy target; the diff is computed from content, not array identity — in-place mutation is the normal case, not an exception | as above |
| **NaN keys** | NaN matches NaN (SameValueZero-style), preventing remove+insert churn | — |

---

## 6. Per-item reactive scope

Each item owns binds (`${item.name}`, item-level `:attr`, nested blocks). With
adjacency dispatch, a bind is a `{fn, q}` record pushed onto `c.deps[i]` for
each dep index `i` — so removal must be able to *unregister* them, or removed
items would keep firing (and leak).

Design: `make` runs with an **item scope** object collected on the handle:

```js
// emitted inside make():
const scope = beginScope(c);          // pushes a collector
bind(c, [1], () => { ... });          // bind() also records itself in the open scope
ifBlock(c, ...); forBlock(c, ...);    // nested blocks record their scopes too
endScope(c);
handle.scope = scope;                 // { subs: [...], children: [...] }
```

- `bind` appends its record to the currently-open scope (a one-field push; no
  measurable setup cost, consistent with "near-zero reactivity setup").
- **Remove**: `dropScope(c, handle.scope)` — for every record, splice it out of
  each `c.deps[i]` list it was pushed to (each record keeps its dep indices),
  then recurse into child scopes (nested `:if`/`:for` items). A queued-but-
  dropped record is skipped at flush via its `q`/alive flag, so removal during
  a pending flush is safe.
  - **Child components in an item**: a `<Child/>` mounted inside the item body
    calls `mountChild(c, …)` with the enclosing component context `c` (not the
    item datum). Because a scope is open when the item is built, `mountChild`
    registers the child's `unmount` as a **disposer** on that item scope
    (`scope.disposers`); `dropScope` runs disposers after unbinding, so removing
    the item unmounts the child — its `onDestroy` fires and it is unlinked from
    `c._children`. Without this, removed items would leak child contexts on the
    parent's `_children` list even though their DOM was gone. mountChild also
    hardens the link write: it never sets `_children` on a non-object `c`, so a
    primitive slipping through can never throw "Cannot create property
    '_children' on number" (never-panic).
- **Move**: no scope work at all — registration is per-component, not
  per-position.
- **Patch**: the item's data cell is a per-item slot (`handle.data` or an
  item-local box when the item template reads it reactively); `patch` writes
  the new datum/index into that slot and runs the item's binds that read it.
  The compiler knows exactly which item-locals each bind reads, so patch is a
  direct call list, not a broadcast.

Index bindings (`const [i, v] of …`) make every item after an insertion point
index-dirty; the compiler only wires index-reactive binds when the template
actually reads the index, so templates that ignore the index pay nothing on
reorder.

---

## 7. Duplicate keys — defined fallback

Keyed reconciliation is meaningless if two items claim the same identity. The
defined behavior (implemented and tested):

1. During key extraction, the first collision aborts keyed mode **for this
   update**: a warning is emitted (`onWarn` hook → `console.warn` in dev
   builds, stripped in production) naming the key and both positions.
2. The entire new list is re-keyed **by index**, and the old list is treated
   positionally as well (so the two key spaces are never mixed). The reconcile
   then degrades to a stable positional patch: item `i` old is reused as item
   `i` new and patched; length differences become tail inserts/removes.
3. Consequences (documented, deterministic): no state-preserving moves for
   that update; visible output is always exactly the target order; nothing
   throws; the next update with unique keys resumes full keyed behavior
   (positional keys never match real keys, so stale identities cannot be
   reused incorrectly).

This mirrors what Vue does (warn + degraded behavior) but with a precisely
specified degradation instead of undefined "patched in place" semantics.

---

## 8. Complexity & alternatives

Let `n` = old length, `m` = new length, `k` = size of the changed middle after
prefix/suffix trimming, `moves` = k − LIS(k).

**This algorithm:** O(n + m) to trim/map/patch, plus O(k log k) for the LIS.
DOM ops: exactly `removes + inserts + (kept − LIS)` placements — provably
minimal placements for the kept set, since any correct reordering must move
every kept node not on some longest increasing subsequence. Memory: O(k) for
the map and index arrays (plain `number` arrays — no BigInt, no width limit).

| Strategy | Time | DOM ops on reorder | State preserved? | Verdict for Lunas |
|---|---|---|---|---|
| **Full rebuild** (re-`innerHTML` the list) | O(m) string + 1 parse | destroys and recreates everything | **No** — loses focus, selection, listeners, input values, nested component state | Forbidden on update (`output-design.md` §8). It *is* the right call for the initial render, where there is no state to lose and the native parser wins — hence the split: innerHTML first paint, diff afterwards. |
| **Non-keyed positional patch** | O(m) | 0 moves, but patches everything after any shift | No (state sticks to positions, not items) | Only as the duplicate-key fallback (§7). |
| **React-style two-pointer + key map, no LIS** | O(n + m) | up to `k` moves — e.g. `[1,2,…,N] → [2,…,N,1]` does N−1 moves where LIS does 1 | Yes | Simpler, but pathological on common patterns (move-one-to-front, sort by a column). The LIS pass costs only O(k log k) CPU on the already-small middle, and each avoided move is an `insertBefore` — layout-invalidating and far more expensive than the comparison it saves. |
| **Vue 3 / ivi keyed diff (trim + map + LIS)** — chosen | O(n + m + k log k) | `k − LIS` moves (minimal placements) | Yes | Best DOM-op economy; trims make the common cases (append, prepend, single edit) O(1)-ish beyond the scan; battle-tested family. |
| **Longest-common-subsequence / edit distance** | O(n·m) | minimal in theory | Yes | Quadratic; strictly dominated by key-map+LIS when keys are unique. |

Why the asymmetry between phases is coherent rather than contradictory:
*initial render is construction* (no prior state exists, so the bulk native
parse is pure win — benchmark-locked in `output-design.md` §2), while *update
is preservation* (state exists precisely in the DOM we would destroy). The two
paths optimize for different invariants, and the reconciler state seeded by
phase (a) is the handoff between them.

---

## 9. Reference implementation & tests

- `runtime-proto/for_diff.mjs` — `createForState`, `seedForState`,
  `reconcile`, `longestIncreasingSubsequence`. Host-abstracted
  (`{ insertBefore(node, refNode), remove(node) }`, `refNode === null` ⇒
  before-anchor), pure ES2015+.
- `runtime-proto/for_diff.test.mjs` — `node runtime-proto/for_diff.test.mjs`:
  - structural correctness (∅→N, N→∅, reverse, swaps, head/tail inserts,
    middle removes, mixed, multi-step sequences) against a validating fake
    host that throws on double-insert / absent-remove / bad refNode;
  - seeded-LCG fuzz (600 transitions/run ≥ 500 required; seed printed and
    overridable via `FUZZ_SEED`);
  - move-minimality: adjacent swap ≤ 1 move, reverse ≤ N−1 moves, prepend =
    1 insert + 0 moves, plus 100 random pure permutations asserting
    `moves === n − LIS(perm)` exactly;
  - duplicate-key fallback exercised (warns, correct visible order, invariants
    hold, recovers on the next unique-key update).
