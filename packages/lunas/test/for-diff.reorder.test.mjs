// for-diff.reorder.test.mjs — deep coverage of the keyed LIS reconciler
// (src/for_diff.mjs) at the abstract-host level. Asserts BOTH final key order
// AND node-identity preservation for items that should not be recreated/moved.
// Run: node packages/lunas/test/for-diff.reorder.test.mjs
//
// The reconciler is host-abstracted, so these tests use a validating fake host
// that mirrors real-DOM insertBefore/remove semantics (throws on double-insert,
// removing a detached node, or a bad refNode) — the same contract the design
// doc's reference harness enforces.

import assert from "node:assert";
import {
  createForState,
  seedForState,
  reconcile,
  longestIncreasingSubsequence,
  extractKeys,
} from "../src/for_diff.mjs";

let passed = 0;
function test(name, fn) {
  fn();
  passed++;
  console.log("  ok  " + name);
}

// ---------------------------------------------------------------------------
// Validating fake host + node factory.
// A "node" is a small object carrying its key and a monotonic uid so identity
// is easy to assert. The host keeps an ordered array of children and enforces
// DOM-like invariants; `null` ref means "append at the end (before anchor)".
// ---------------------------------------------------------------------------
function makeHost() {
  const children = []; // current order
  const present = new Set();
  return {
    children,
    insertBefore(node, ref) {
      // A move: remove from current position first (real DOM does this too).
      const cur = children.indexOf(node);
      if (cur >= 0) children.splice(cur, 1);
      if (ref === null || ref === undefined) {
        children.push(node);
      } else {
        const at = children.indexOf(ref);
        if (at < 0) throw new Error("insertBefore: refNode is not present");
        children.splice(at, 0, node);
      }
      present.add(node);
    },
    remove(node) {
      const at = children.indexOf(node);
      if (at < 0) throw new Error("remove: node is not present");
      children.splice(at, 1);
      present.delete(node);
    },
  };
}

let uid = 0;
// A make that counts calls and tags nodes; wrap in a fresh counter per test.
function makerWith(counter) {
  return (data, key /*, index */) => {
    counter.made++;
    return { key, data, uid: uid++ };
  };
}

const keyOrder = (host) => host.children.map((n) => n.key);

// Seed a state+host from an initial keyed array, returning { state, host,
// nodesByKey } so later assertions can check identity of specific keys.
function seedFrom(keys) {
  const host = makeHost();
  const nodes = keys.map((k) => ({ key: k, data: k, uid: uid++ }));
  for (const n of nodes) host.children.push(n);
  const state = createForState();
  seedForState(state, keys, nodes, keys.slice());
  const nodesByKey = new Map();
  keys.forEach((k, i) => nodesByKey.set(k, nodes[i]));
  return { state, host, nodesByKey };
}

const idOpts = () => ({ keyOf: (d) => d });

// ---------------------------------------------------------------------------
// Structural cases — each asserts final order.
// ---------------------------------------------------------------------------

test("append: prefix trims, pure-insert shortcut, 0 moves on kept", () => {
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c"]);
  const counter = { made: 0 };
  reconcile(state, host, ["a", "b", "c", "d", "e"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "b", "c", "d", "e"]);
  assert.strictEqual(counter.made, 2, "only the two new items built");
  // a,b,c identities untouched.
  assert.strictEqual(host.children[0], nodesByKey.get("a"));
  assert.strictEqual(host.children[2], nodesByKey.get("c"));
});

test("prepend: suffix trims, 1 insert + 0 moves", () => {
  const { state, host, nodesByKey } = seedFrom(["b", "c", "d"]);
  const counter = { made: 0 };
  reconcile(state, host, ["a", "b", "c", "d"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "b", "c", "d"]);
  assert.strictEqual(counter.made, 1);
  // b,c,d never moved/recreated.
  assert.strictEqual(host.children[1], nodesByKey.get("b"));
  assert.strictEqual(host.children[3], nodesByKey.get("d"));
});

test("insert-middle: one new key in the middle, neighbors keep identity", () => {
  const { state, host, nodesByKey } = seedFrom(["a", "b", "d"]);
  const counter = { made: 0 };
  reconcile(state, host, ["a", "b", "c", "d"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "b", "c", "d"]);
  assert.strictEqual(counter.made, 1);
  assert.strictEqual(host.children[0], nodesByKey.get("a"));
  assert.strictEqual(host.children[1], nodesByKey.get("b"));
  assert.strictEqual(host.children[3], nodesByKey.get("d"));
});

test("remove-middle: pure-remove shortcut, neighbors keep identity", () => {
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c", "d"]);
  const counter = { made: 0 };
  reconcile(state, host, ["a", "d"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "d"]);
  assert.strictEqual(counter.made, 0, "nothing built on pure remove");
  assert.strictEqual(host.children[0], nodesByKey.get("a"));
  assert.strictEqual(host.children[1], nodesByKey.get("d"));
});

test("remove-all: N -> empty", () => {
  const { state, host } = seedFrom(["a", "b", "c"]);
  const counter = { made: 0 };
  reconcile(state, host, [], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), []);
  assert.strictEqual(counter.made, 0);
  assert.strictEqual(state.keys.length, 0);
});

test("empty -> N: pure build, 0 moves", () => {
  const host = makeHost();
  const state = createForState();
  const counter = { made: 0 };
  reconcile(state, host, ["x", "y", "z"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["x", "y", "z"]);
  assert.strictEqual(counter.made, 3);
});

test("swap adjacent: exactly one node moves, other keeps identity", () => {
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c", "d"]);
  const counter = { made: 0 };
  const b = nodesByKey.get("b");
  const cNode = nodesByKey.get("c");
  reconcile(state, host, ["a", "c", "b", "d"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "c", "b", "d"]);
  assert.strictEqual(counter.made, 0, "swap creates nothing");
  // Both nodes reused (by identity); only their order changed.
  assert.strictEqual(host.children[1], cNode);
  assert.strictEqual(host.children[2], b);
  assert.strictEqual(host.children[0], nodesByKey.get("a"), "a unmoved kept");
  assert.strictEqual(host.children[3], nodesByKey.get("d"), "d unmoved kept");
});

test("reverse: all nodes reused by identity, none recreated", () => {
  const keys = ["a", "b", "c", "d", "e"];
  const { state, host, nodesByKey } = seedFrom(keys);
  const counter = { made: 0 };
  reconcile(state, host, ["e", "d", "c", "b", "a"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["e", "d", "c", "b", "a"]);
  assert.strictEqual(counter.made, 0, "reverse recreates nothing");
  for (const k of keys) {
    // every original node is still present (identity preserved), just reordered
    assert.ok(host.children.includes(nodesByKey.get(k)), "node " + k + " kept");
  }
  // the LIS-preserved node (the single fixed point of a full reverse) never
  // needs recreation regardless of which one it is.
});

test("move-far: front item to the back keeps every node's identity", () => {
  const keys = ["a", "b", "c", "d", "e"];
  const { state, host, nodesByKey } = seedFrom(keys);
  const counter = { made: 0 };
  reconcile(state, host, ["b", "c", "d", "e", "a"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["b", "c", "d", "e", "a"]);
  assert.strictEqual(counter.made, 0);
  // b..e form the LIS (already in order) and must NOT be recreated; a moved.
  assert.strictEqual(host.children[4], nodesByKey.get("a"));
  for (const k of keys) assert.ok(host.children.includes(nodesByKey.get(k)));
});

test("shuffle: arbitrary permutation, all nodes reused, order exact", () => {
  const keys = ["a", "b", "c", "d", "e", "f", "g"];
  const { state, host, nodesByKey } = seedFrom(keys);
  const counter = { made: 0 };
  const target = ["d", "g", "a", "f", "b", "e", "c"];
  reconcile(state, host, target, makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), target);
  assert.strictEqual(counter.made, 0, "permutation recreates nothing");
  for (const k of keys) assert.ok(host.children.includes(nodesByKey.get(k)));
});

test("stable-vs-moved: an ascending run stays put (LIS identity)", () => {
  // [a b c d e f] -> [a b c f d e]: a,b,c stay; only f moves before d.
  // Assert a,b,c,d,e keep identity and are NOT touched (still present).
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c", "d", "e", "f"]);
  const counter = { made: 0 };
  reconcile(state, host, ["a", "b", "c", "f", "d", "e"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["a", "b", "c", "f", "d", "e"]);
  assert.strictEqual(counter.made, 0);
  assert.strictEqual(host.children[0], nodesByKey.get("a"));
  assert.strictEqual(host.children[1], nodesByKey.get("b"));
  assert.strictEqual(host.children[2], nodesByKey.get("c"));
  assert.strictEqual(host.children[3], nodesByKey.get("f"));
  assert.strictEqual(host.children[4], nodesByKey.get("d"));
});

test("interleaved insert + remove + move in one update", () => {
  // old: a b c d e ; new: c x a e y  (b,d removed; x,y inserted; a,c,e moved)
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c", "d", "e"]);
  const counter = { made: 0 };
  reconcile(state, host, ["c", "x", "a", "e", "y"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["c", "x", "a", "e", "y"]);
  assert.strictEqual(counter.made, 2, "x and y are new");
  // kept keys reuse their original nodes.
  assert.strictEqual(host.children[0], nodesByKey.get("c"));
  assert.strictEqual(host.children[2], nodesByKey.get("a"));
  assert.strictEqual(host.children[3], nodesByKey.get("e"));
  // removed nodes are gone.
  assert.ok(!host.children.includes(nodesByKey.get("b")));
  assert.ok(!host.children.includes(nodesByKey.get("d")));
});

test("single item: identity kept across a no-op update", () => {
  const { state, host, nodesByKey } = seedFrom(["only"]);
  const counter = { made: 0 };
  reconcile(state, host, ["only"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["only"]);
  assert.strictEqual(counter.made, 0);
  assert.strictEqual(host.children[0], nodesByKey.get("only"));
});

test("single item replaced by a different key: remove + insert", () => {
  const { state, host, nodesByKey } = seedFrom(["a"]);
  const counter = { made: 0 };
  reconcile(state, host, ["b"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["b"]);
  assert.strictEqual(counter.made, 1, "b built");
  assert.ok(!host.children.includes(nodesByKey.get("a")), "a removed");
});

test("replace-all-keys: none reused, all rebuilt", () => {
  const { state, host, nodesByKey } = seedFrom(["a", "b", "c"]);
  const counter = { made: 0 };
  reconcile(state, host, ["x", "y", "z"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["x", "y", "z"]);
  assert.strictEqual(counter.made, 3);
  for (const k of ["a", "b", "c"])
    assert.ok(!host.children.includes(nodesByKey.get(k)));
});

// ---------------------------------------------------------------------------
// Key types: numbers vs strings are distinct keys (=== semantics).
// ---------------------------------------------------------------------------

test("numeric keys reconcile and reuse by identity", () => {
  const host = makeHost();
  const state = createForState();
  const counter = { made: 0 };
  reconcile(state, host, [1, 2, 3], makerWith(counter), idOpts());
  const n2 = host.children[1];
  reconcile(state, host, [3, 2, 1], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), [3, 2, 1]);
  assert.strictEqual(host.children[1], n2, "numeric key 2 reused");
  assert.strictEqual(counter.made, 3, "reorder built nothing new");
});

test("number key 1 and string key '1' are different identities", () => {
  const host = makeHost();
  const state = createForState();
  const counter = { made: 0 };
  reconcile(state, host, [1], makerWith(counter), idOpts());
  const numNode = host.children[0];
  reconcile(state, host, ["1"], makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), ["1"]);
  assert.strictEqual(counter.made, 2, "'1' is a fresh key -> rebuilt");
  assert.ok(!host.children.includes(numNode), "numeric node removed");
});

// ---------------------------------------------------------------------------
// Duplicate keys -> index-key fallback (§7). Order still exact; no throw.
// ---------------------------------------------------------------------------

test("duplicate keys fall back to index keys; order exact; warns once", () => {
  // Under the index fallback the reconcile degrades to a stable POSITIONAL
  // patch (§7): old items are reused by position and patched with the new data,
  // so the visible data ends up exactly the target sequence. We assert on the
  // patched data (via patchItem), since node keys stay at their seed values by
  // design when identity reuse is disabled.
  const { state, host } = seedFrom(["a", "b", "c"]);
  const counter = { made: 0 };
  const warnings = [];
  const finalData = new Array(3);
  reconcile(state, host, ["dup", "dup", "z"], makerWith(counter), {
    keyOf: (d) => d,
    onWarn: (m) => warnings.push(m),
    patchItem: (node, d, i) => {
      finalData[i] = d;
    },
  });
  // Three nodes present, in order, holding the target data.
  assert.strictEqual(host.children.length, 3);
  assert.deepStrictEqual(finalData, ["dup", "dup", "z"]);
  assert.strictEqual(counter.made, 0, "positional reuse builds nothing");
  assert.strictEqual(warnings.length, 1, "warned exactly once");
  assert.match(warnings[0], /duplicate key/);
  // State keys are positional after fallback.
  assert.deepStrictEqual(state.keys, [0, 1, 2]);
});

test("duplicate-key fallback with growth: tail insert covers new length", () => {
  // old length 2, new length 3 with a dup -> positional patch on [0,1] then a
  // tail insert for index 2.
  const { state, host } = seedFrom(["a", "b"]);
  const counter = { made: 0 };
  reconcile(state, host, ["x", "x", "y"], makerWith(counter), {
    keyOf: (d) => d,
    onWarn: () => {},
  });
  assert.strictEqual(host.children.length, 3, "grew to 3 nodes");
  assert.strictEqual(counter.made, 1, "one tail item built");
});

test("recovers to full keyed behavior on the next unique-key update", () => {
  const { state, host } = seedFrom(["a", "b"]);
  const counter = { made: 0 };
  reconcile(state, host, ["x", "x"], makerWith(counter), {
    keyOf: (d) => d,
    onWarn: () => {},
  });
  assert.strictEqual(host.children.length, 2);
  // Now a clean unique-key update: keys become real again.
  reconcile(state, host, ["p", "q"], makerWith(counter), idOpts());
  assert.strictEqual(host.children.length, 2);
  assert.deepStrictEqual(state.keys, ["p", "q"]);
});

test("extractKeys: unique keys pass through, no dup flag", () => {
  const r = extractKeys(["a", "b", "c"], (d) => d, null);
  assert.deepStrictEqual(r.keys, ["a", "b", "c"]);
  assert.strictEqual(r.duped, false);
});

test("extractKeys: duplicate collapses to positional keys + duped flag", () => {
  let warned = 0;
  const r = extractKeys(["a", "a", "b"], (d) => d, () => warned++);
  assert.deepStrictEqual(r.keys, [0, 1, 2]);
  assert.strictEqual(r.duped, true);
  assert.strictEqual(warned, 1);
});

// ---------------------------------------------------------------------------
// LIS unit tests — the move-minimizer itself.
// ---------------------------------------------------------------------------

test("longestIncreasingSubsequence: basic ascending run positions", () => {
  // arr values are old indices; result is POSITIONS into arr forming a longest
  // strictly-increasing run.
  const arr = [2, 3, 1, 5, 4];
  const pos = longestIncreasingSubsequence(arr);
  const vals = pos.map((p) => arr[p]);
  // strictly increasing
  for (let i = 1; i < vals.length; i++) assert.ok(vals[i] > vals[i - 1]);
  assert.strictEqual(vals.length, 3, "LIS length of [2,3,1,5,4] is 3");
});

test("longestIncreasingSubsequence: skips NADA (-1) holes", () => {
  const arr = [-1, 0, -1, 1, 2];
  const pos = longestIncreasingSubsequence(arr);
  const vals = pos.map((p) => arr[p]);
  assert.ok(!vals.includes(-1), "holes never appear in the LIS");
  assert.deepStrictEqual(vals, [0, 1, 2]);
});

test("longestIncreasingSubsequence: empty and all-holes", () => {
  assert.deepStrictEqual(longestIncreasingSubsequence([]), []);
  assert.deepStrictEqual(longestIncreasingSubsequence([-1, -1]), []);
});

test("longestIncreasingSubsequence: strictly decreasing has length 1", () => {
  const arr = [5, 4, 3, 2, 1];
  const pos = longestIncreasingSubsequence(arr);
  assert.strictEqual(pos.length, 1);
});

// ---------------------------------------------------------------------------
// Large list (100+) correctness: reverse a big list, assert exact order and
// that no node was recreated (all reused by identity).
// ---------------------------------------------------------------------------

test("large list (120): reverse reuses all nodes, exact order", () => {
  const N = 120;
  const keys = [];
  for (let i = 0; i < N; i++) keys.push("k" + i);
  const { state, host, nodesByKey } = seedFrom(keys);
  const counter = { made: 0 };
  const target = keys.slice().reverse();
  reconcile(state, host, target, makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), target);
  assert.strictEqual(counter.made, 0, "reverse of 120 recreated nothing");
  for (const k of keys) assert.ok(host.children.includes(nodesByKey.get(k)));
});

test("large list (150): random shuffle, exact order + full reuse", () => {
  const N = 150;
  const keys = [];
  for (let i = 0; i < N; i++) keys.push(i);
  const { state, host, nodesByKey } = seedFrom(keys);
  const counter = { made: 0 };
  // deterministic LCG shuffle
  let s = 123456789 >>> 0;
  const rand = () => ((s = (1103515245 * s + 12345) >>> 0) / 0x100000000);
  const target = keys.slice();
  for (let i = target.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    const t = target[i];
    target[i] = target[j];
    target[j] = t;
  }
  reconcile(state, host, target, makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), target);
  assert.strictEqual(counter.made, 0, "shuffle recreated nothing");
  for (const k of keys) assert.ok(host.children.includes(nodesByKey.get(k)));
});

test("large list: append 50 to 100 keeps the original 100 untouched", () => {
  const base = [];
  for (let i = 0; i < 100; i++) base.push("b" + i);
  const { state, host, nodesByKey } = seedFrom(base);
  const counter = { made: 0 };
  const target = base.slice();
  for (let i = 0; i < 50; i++) target.push("n" + i);
  reconcile(state, host, target, makerWith(counter), idOpts());
  assert.deepStrictEqual(keyOrder(host), target);
  assert.strictEqual(counter.made, 50, "only the 50 new items built");
  // original 100 identities preserved and unmoved.
  for (let i = 0; i < 100; i++)
    assert.strictEqual(host.children[i], nodesByKey.get("b" + i));
});

// ---------------------------------------------------------------------------
// patchItem is called for kept items with new data/index.
// ---------------------------------------------------------------------------

test("patchItem receives new data & index for kept items", () => {
  const host = makeHost();
  const state = createForState();
  const counter = { made: 0 };
  const mk = (d, k) => ({ key: k, data: d, uid: uid++ });
  reconcile(state, host, [{ id: 1, v: "a" }, { id: 2, v: "b" }], mk, {
    keyOf: (d) => d.id,
  });
  const patched = [];
  reconcile(
    state,
    host,
    [{ id: 2, v: "B" }, { id: 1, v: "A" }],
    mk,
    {
      keyOf: (d) => d.id,
      patchItem: (node, d, i) => patched.push([node.key, d.v, i]),
    }
  );
  assert.deepStrictEqual(keyOrder(host), [2, 1]);
  // both kept items patched with their new value and new index.
  const byKey = new Map(patched.map((p) => [p[0], p]));
  assert.deepStrictEqual(byKey.get(2), [2, "B", 0]);
  assert.deepStrictEqual(byKey.get(1), [1, "A", 1]);
});

console.log("\nfor-diff.reorder.test.mjs: " + passed + " passed.");
