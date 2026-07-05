// emits.edge.test.mjs — additional edge-focused coverage for emits.mjs beyond
// emits.test.mjs: kebab-case edge cases, payload passthrough shapes, multiple
// emits on one child, re-registering emits, and the "child emit never marks
// parent dirty" contract exercised with markVar-driven reactivity more deeply.
// Run: node packages/lunas/test/emits.edge.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { emit, registerEmits, eventPropName } from "../src/emits.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));

// -- eventPropName edge cases -------------------------------------------------

test("eventPropName: single-letter and already-capitalized names", () => {
  assert.equal(eventPropName("a"), "onA");
  // "Close" (already capitalized) -> charAt(0).toUpperCase() is a no-op.
  assert.equal(eventPropName("Close"), "onClose");
});

test("eventPropName: multiple consecutive hyphens collapse per hyphen-letter pair", () => {
  assert.equal(eventPropName("a-b-c-d"), "onABCD");
});

test("eventPropName: trailing hyphen leaves nothing to capitalize after it", () => {
  // "save-" -> replace(/-([a-z])/g,...) finds no letter after the trailing '-',
  // so it's left as-is; "on" + "Save-" charAt(0) uppercased.
  assert.equal(eventPropName("save-"), "onSave-");
});

// -- payload passthrough shapes -----------------------------------------------

test("emit passes through primitive, object, array, and undefined payloads unchanged", () => {
  const c = createContext(document.createElement("div"));
  const got = [];
  registerEmits(c, {
    onA: (p) => got.push(p),
  });
  emit(c, "a", 42);
  emit(c, "a", "str");
  emit(c, "a", { x: 1 });
  emit(c, "a", [1, 2, 3]);
  emit(c, "a", undefined);
  emit(c, "a", null);
  assert.deepEqual(got, [42, "str", { x: 1 }, [1, 2, 3], undefined, null]);
});

test("emit with zero-arg payload (undefined) still invokes the handler", () => {
  const c = createContext(document.createElement("div"));
  let called = false;
  let receivedArgCount = -1;
  registerEmits(c, {
    onPing: function () {
      called = true;
      receivedArgCount = arguments.length;
    },
  });
  emit(c, "ping");
  assert.equal(called, true);
  assert.equal(receivedArgCount, 1, "emit always calls handler with exactly one arg (payload, possibly undefined)");
});

// -- multiple distinct events on one child ------------------------------------

test("a child can emit several distinct event names, each routed independently", () => {
  const c = createContext(document.createElement("div"));
  const calls = [];
  registerEmits(c, {
    onSave: (p) => calls.push(["save", p]),
    onCancel: (p) => calls.push(["cancel", p]),
  });
  emit(c, "save", 1);
  emit(c, "cancel", 2);
  emit(c, "save", 3);
  assert.deepEqual(calls, [
    ["save", 1],
    ["cancel", 2],
    ["save", 3],
  ]);
});

test("emitting a name with no matching on<Name> prop is a no-op even when other handlers exist", () => {
  const c = createContext(document.createElement("div"));
  let saveCalled = false;
  registerEmits(c, { onSave: () => (saveCalled = true) });
  const ran = emit(c, "delete", {});
  assert.equal(ran, false);
  assert.equal(saveCalled, false);
});

// -- re-registering emits (e.g. prop update mid-life) -------------------------

test("calling registerEmits again replaces the prior handler set entirely", () => {
  const c = createContext(document.createElement("div"));
  let firstCalled = false;
  let secondCalled = false;
  registerEmits(c, { onSave: () => (firstCalled = true) });
  registerEmits(c, { onSave: () => (secondCalled = true) }); // fresh props object
  emit(c, "save", {});
  assert.equal(firstCalled, false, "old handler set discarded");
  assert.equal(secondCalled, true);
});

test("registerEmits with declared list allows the declared event silently (no warning)", () => {
  const c = createContext(document.createElement("div"));
  let warned = false;
  const orig = console.warn;
  console.warn = () => (warned = true);
  try {
    registerEmits(c, { onSave: () => {} }, ["save", "cancel"]);
    emit(c, "save", 1);
    assert.equal(warned, false);
  } finally {
    console.warn = orig;
  }
});

// -- emit does not mark the parent dirty: deeper multi-bind exercise ---------

test("emit handler mutating a DIFFERENT parent box than the one a bind reads leaves that bind untouched", async () => {
  const parent = createContext(document.createElement("div"));
  const watched = box(parent, 0, "a");
  const unrelated = box(parent, 1, "z");
  let runs = 0;
  bind(parent, [0], () => {
    watched.v;
    runs++;
  });
  runs = 0;

  const child = createContext(document.createElement("span"));
  registerEmits(child, {
    onPing: () => {
      unrelated.v = "changed"; // marks index 1, not 0
    },
  });
  emit(child, "ping", {});
  await tick();
  assert.equal(runs, 0, "bind on index 0 unaffected by a write to index 1");
});

test("emit synchronously invokes the handler; any parent mark is still batched to a microtask", async () => {
  const parent = createContext(document.createElement("div"));
  const v = box(parent, 0, 0);
  const order = [];
  bind(parent, [0], () => {
    order.push("bind:" + v.v);
  });
  order.length = 0;

  const child = createContext(document.createElement("span"));
  registerEmits(child, {
    onBump: () => {
      v.v = v.v + 1;
      order.push("handler-done");
    },
  });
  emit(child, "bump", {});
  order.push("after-emit-call");
  assert.deepEqual(order, ["handler-done", "after-emit-call"], "flush hasn't run yet — it's a microtask");
  await tick();
  assert.deepEqual(order, ["handler-done", "after-emit-call", "bind:1"]);
});

test("emit returns false and does not throw when c is null/undefined", () => {
  assert.equal(emit(null, "x", 1), false);
  assert.equal(emit(undefined, "x", 1), false);
});
