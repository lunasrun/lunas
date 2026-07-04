// for_diff.test.mjs — thorough tests for the keyed :for reconciler.
// Run: node runtime-proto/for_diff.test.mjs   (no npm deps, node:assert only)

import assert from "node:assert";
import {
  createForState,
  seedForState,
  reconcile,
  longestIncreasingSubsequence,
} from "./for_diff.mjs";

// -----------------------------------------------------------------------------
// Validating host — a DOM-free doubly-linked list of "nodes" that enforces the
// invariants a real DOM would enforce, so bugs surface as thrown assertions:
//   * a node is never inserted twice (present while re-inserted is a MOVE — ok —
//     but we assert it is a KNOWN node, never an alien one);
//   * a node is never removed while absent;
//   * insertBefore's refNode is null or currently present.
// -----------------------------------------------------------------------------
class Host {
  constructor() {
    this.order = []; // array of node objects, in child order
    this.present = new Set();
    this.everMade = new Set();
    this.opCount = 0; // insert-or-move ops (host mutations that place a node)
    this.moveCount = 0; // placements of an already-present node
    this.insertCount = 0; // placements of a new node
    this.removeCount = 0;
  }

  _indexOf(node) {
    return this.order.indexOf(node);
  }

  insertBefore(node, refNode) {
    this.opCount++;
    assert.ok(this.everMade.has(node), "insertBefore of a node the host never saw made");
    if (refNode !== null) {
      assert.ok(this.present.has(refNode), "insertBefore refNode is not present");
    }
    if (this.present.has(node)) {
      // move: remove from current position first
      this.moveCount++;
      this.order.splice(this._indexOf(node), 1);
    } else {
      this.insertCount++;
      this.present.add(node);
    }
    const at = refNode === null ? this.order.length : this._indexOf(refNode);
    this.order.splice(at, 0, node);
  }

  remove(node) {
    this.removeCount++;
    assert.ok(this.present.has(node), "remove of a node that is absent");
    this.order.splice(this._indexOf(node), 1);
    this.present.delete(node);
  }

  keyOrder() {
    return this.order.map((n) => n.key);
  }

  dataOrder() {
    return this.order.map((n) => n.data);
  }

  resetCounters() {
    this.opCount = 0;
    this.moveCount = 0;
    this.insertCount = 0;
    this.removeCount = 0;
  }
}

// A node factory bound to a host. Each node remembers its key + last data.
function makeHost() {
  const host = new Host();
  const makeItem = (itemData, key) => {
    const node = { key, data: itemData };
    host.everMade.add(node);
    return node;
  };
  const patchItem = (node, itemData) => {
    node.data = itemData;
  };
  return { host, makeItem, patchItem };
}

// Seed a fresh state+host with the initial key list (simulates the bulk
// innerHTML build: all items already placed, in order).
function seed(keys) {
  const { host, makeItem, patchItem } = makeHost();
  const state = createForState();
  const nodes = keys.map((k) => {
    const n = makeItem(k, k);
    host.present.add(n);
    host.order.push(n);
    return n;
  });
  seedForState(state, keys.slice(), nodes, keys.slice());
  return { host, makeItem, patchItem, state };
}

// keyOf that treats a datum as its own key (data === key in these tests).
const idKey = (d) => d;

// Run one transition and assert the host order matches target keys.
function step(ctx, target, opts) {
  ctx.host.resetCounters();
  reconcile(
    ctx.state,
    ctx.host,
    target,
    ctx.makeItem,
    Object.assign({ keyOf: idKey, patchItem: ctx.patchItem }, opts || {})
  );
  assert.deepStrictEqual(
    ctx.host.keyOrder(),
    target,
    "host order != target after reconcile"
  );
  // state must mirror host
  assert.deepStrictEqual(ctx.state.keys, target, "state.keys != target");
  assert.strictEqual(ctx.host.order.length, ctx.host.present.size);
}

let passed = 0;
function test(name, fn) {
  fn();
  passed++;
  console.log("  ok  " + name);
}

console.log("LIS unit tests");
test("LIS basic", () => {
  // values, expect positions of a longest strictly-increasing run
  assert.deepStrictEqual(longestIncreasingSubsequence([2, 3, 1, 5, 6, 4]), [0, 1, 3, 4]);
  assert.deepStrictEqual(longestIncreasingSubsequence([]), []);
  assert.deepStrictEqual(longestIncreasingSubsequence([5]), [0]);
  assert.deepStrictEqual(longestIncreasingSubsequence([3, 2, 1]).length, 1);
});
test("LIS skips NADA holes", () => {
  // -1 entries (new items) never participate
  const r = longestIncreasingSubsequence([0, -1, 1, -1, 2]);
  assert.deepStrictEqual(r, [0, 2, 4]);
});

console.log("Correctness — structural transitions");
test("empty -> N", () => {
  const ctx = seed([]);
  step(ctx, [1, 2, 3, 4]);
  assert.strictEqual(ctx.host.insertCount, 4);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("N -> empty", () => {
  const ctx = seed([1, 2, 3, 4]);
  step(ctx, []);
  assert.strictEqual(ctx.host.removeCount, 4);
});
test("reverse", () => {
  const ctx = seed([1, 2, 3, 4, 5]);
  step(ctx, [5, 4, 3, 2, 1]);
  // reverse of N: at most N-1 moves
  assert.ok(ctx.host.moveCount <= 4, "reverse moves " + ctx.host.moveCount);
});
test("adjacent swap", () => {
  const ctx = seed([1, 2, 3, 4, 5]);
  step(ctx, [1, 3, 2, 4, 5]); // swap 2<->3
  assert.ok(ctx.host.moveCount <= 1, "adjacent swap moves " + ctx.host.moveCount);
});
test("head insert", () => {
  const ctx = seed([2, 3, 4]);
  step(ctx, [1, 2, 3, 4]);
  assert.strictEqual(ctx.host.insertCount, 1);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("prepend 1 to N = 1 insert 0 moves", () => {
  const ctx = seed([1, 2, 3, 4, 5, 6, 7, 8]);
  step(ctx, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
  assert.strictEqual(ctx.host.insertCount, 1);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("tail insert", () => {
  const ctx = seed([1, 2, 3]);
  step(ctx, [1, 2, 3, 4, 5]);
  assert.strictEqual(ctx.host.insertCount, 2);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("middle remove", () => {
  const ctx = seed([1, 2, 3, 4, 5]);
  step(ctx, [1, 2, 4, 5]);
  assert.strictEqual(ctx.host.removeCount, 1);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("mixed insert+remove+move", () => {
  const ctx = seed([1, 2, 3, 4, 5]);
  step(ctx, [4, 1, 6, 2, 5]); // remove 3, add 6, reorder
});
test("full replace, no shared keys", () => {
  const ctx = seed([1, 2, 3]);
  step(ctx, [4, 5, 6]);
  assert.strictEqual(ctx.host.insertCount, 3);
  assert.strictEqual(ctx.host.removeCount, 3);
});
test("sequence of transitions on one state", () => {
  const ctx = seed([1, 2, 3]);
  step(ctx, [3, 2, 1]);
  step(ctx, [3, 2, 1, 4]);
  step(ctx, [4]);
  step(ctx, []);
  step(ctx, [9, 8, 7]);
});

console.log("Duplicate keys — index fallback");
// Under index fallback, items are reused POSITIONALLY and patched with the new
// data. The reused nodes keep their old identity key, so we verify the visible
// DATA order (what the user sees) rather than the internal key, plus the host
// invariants (no double insert / absent remove) which the Host enforces.
test("dup keys do not throw and data order is correct", () => {
  const ctx = seed([1, 2, 3]);
  let warned = 0;
  // target has duplicate key 'x'
  reconcile(ctx.state, ctx.host, ["x", "x", "y"], ctx.makeItem, {
    keyOf: idKey,
    patchItem: ctx.patchItem,
    onWarn: () => warned++,
  });
  assert.ok(warned >= 1, "expected a duplicate-key warning");
  // visible data equals target despite duplicate keys (positional reuse)
  assert.deepStrictEqual(ctx.host.dataOrder(), ["x", "x", "y"]);
  assert.strictEqual(ctx.host.order.length, ctx.host.present.size);
  assert.strictEqual(new Set(ctx.host.order).size, ctx.host.order.length);
});
test("dup keys then recover to unique", () => {
  const ctx = seed(["a", "b"]);
  reconcile(ctx.state, ctx.host, ["d", "d", "d"], ctx.makeItem, {
    keyOf: idKey,
    patchItem: ctx.patchItem,
    onWarn: () => {},
  });
  assert.deepStrictEqual(ctx.host.dataOrder(), ["d", "d", "d"]);
  // recover: unique keys again — data order must match target
  reconcile(ctx.state, ctx.host, ["p", "q", "r"], ctx.makeItem, {
    keyOf: idKey,
    patchItem: ctx.patchItem,
    onWarn: () => {},
  });
  assert.deepStrictEqual(ctx.host.dataOrder(), ["p", "q", "r"]);
  assert.strictEqual(ctx.host.order.length, ctx.host.present.size);
});

console.log("Move-minimality sanity");
test("adjacent swap among N=10 -> <=1 move", () => {
  const base = Array.from({ length: 10 }, (_, i) => i);
  const ctx = seed(base);
  const t = base.slice();
  const tmp = t[4];
  t[4] = t[5];
  t[5] = tmp;
  step(ctx, t);
  assert.ok(ctx.host.moveCount <= 1, "moves=" + ctx.host.moveCount);
});
test("reverse N=10 -> <=9 moves", () => {
  const base = Array.from({ length: 10 }, (_, i) => i);
  const ctx = seed(base);
  step(ctx, base.slice().reverse());
  assert.ok(ctx.host.moveCount <= 9, "moves=" + ctx.host.moveCount);
});
test("prepend to N=50 -> 1 insert 0 moves", () => {
  const base = Array.from({ length: 50 }, (_, i) => i + 1);
  const ctx = seed(base);
  step(ctx, [0].concat(base));
  assert.strictEqual(ctx.host.insertCount, 1);
  assert.strictEqual(ctx.host.moveCount, 0);
});
test("move one item across a stable block -> 1 move", () => {
  // [1..8], move '2' to the end. Only '2' should move.
  const ctx = seed([1, 2, 3, 4, 5, 6, 7, 8]);
  step(ctx, [1, 3, 4, 5, 6, 7, 8, 2]);
  assert.strictEqual(ctx.host.moveCount, 1, "moves=" + ctx.host.moveCount);
});

// -----------------------------------------------------------------------------
// Seeded fuzz — LCG so failures reproduce from the printed seed.
// -----------------------------------------------------------------------------
console.log("Seeded fuzz");
function makeLCG(seed) {
  let s = seed >>> 0;
  return () => {
    // Numerical Recipes LCG
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

test("500 random transitions preserve target order + invariants", () => {
  const seed0 = (process.env.FUZZ_SEED
    ? parseInt(process.env.FUZZ_SEED, 10)
    : (Date.now() ^ 0x9e3779b9)) >>> 0;
  console.log("      fuzz seed = " + seed0);
  const rnd = makeLCG(seed0);
  const randInt = (n) => Math.floor(rnd() * n);

  const ctx = seed([]);
  let keyCounter = 1000; // pool of fresh unique keys

  const ITER = 600; // >= 500 required
  for (let it = 0; it < ITER; it++) {
    // Build a target from the CURRENT keys: keep a random subset, shuffle it,
    // and inject some brand-new unique keys. This exercises remove+move+insert.
    const cur = ctx.state.keys.slice();
    const kept = [];
    for (let i = 0; i < cur.length; i++) {
      if (rnd() < 0.6) kept.push(cur[i]); // ~60% survive
    }
    // shuffle kept (Fisher-Yates)
    for (let i = kept.length - 1; i > 0; i--) {
      const j = randInt(i + 1);
      const t = kept[i];
      kept[i] = kept[j];
      kept[j] = t;
    }
    // inject up to 4 new keys at random positions
    const inserts = randInt(5);
    for (let i = 0; i < inserts; i++) {
      const pos = randInt(kept.length + 1);
      kept.splice(pos, 0, keyCounter++);
    }
    // occasionally collapse to empty to hit that path
    const target = rnd() < 0.05 ? [] : kept;

    ctx.host.resetCounters();
    reconcile(ctx.state, ctx.host, target, ctx.makeItem, {
      keyOf: idKey,
      patchItem: ctx.patchItem,
    });

    assert.deepStrictEqual(
      ctx.host.keyOrder(),
      target,
      "FUZZ mismatch at iter " + it + " seed " + seed0
    );
    assert.strictEqual(
      ctx.host.order.length,
      ctx.host.present.size,
      "FUZZ present/order desync at iter " + it + " seed " + seed0
    );
    // no duplicate nodes present
    assert.strictEqual(
      new Set(ctx.host.order).size,
      ctx.host.order.length,
      "FUZZ duplicate node present at iter " + it + " seed " + seed0
    );
  }
});

test("fuzz move-minimality: pure permutation moves <= n-1 - lis", () => {
  // For a pure reordering (no inserts/removes), the number of moves must equal
  // n - LIS(perm). Verify on random permutations.
  const seedP = 0xC0FFEE;
  const rnd = makeLCG(seedP);
  const randInt = (n) => Math.floor(rnd() * n);
  for (let trial = 0; trial < 100; trial++) {
    const n = 1 + randInt(30);
    const base = Array.from({ length: n }, (_, i) => i);
    const ctx = seed(base);
    // random permutation
    const perm = base.slice();
    for (let i = perm.length - 1; i > 0; i--) {
      const j = randInt(i + 1);
      const t = perm[i];
      perm[i] = perm[j];
      perm[j] = t;
    }
    // expected moves = n - LIS length of (perm as old indices)
    const lisLen = longestIncreasingSubsequence(perm).length;
    ctx.host.resetCounters();
    reconcile(ctx.state, ctx.host, perm, ctx.makeItem, { keyOf: idKey });
    assert.deepStrictEqual(ctx.host.keyOrder(), perm, "perm order");
    assert.strictEqual(
      ctx.host.moveCount,
      n - lisLen,
      "trial " + trial + " n=" + n + " moves=" + ctx.host.moveCount + " expected=" + (n - lisLen)
    );
    assert.strictEqual(ctx.host.insertCount, 0, "no inserts in pure perm");
    assert.strictEqual(ctx.host.removeCount, 0, "no removes in pure perm");
  }
});

console.log("\nAll tests passed: " + passed + " test blocks.");
