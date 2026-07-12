// emits.test.mjs — child → parent events. Run: node packages/lunas/test/emits.test.mjs

import assert from "node:assert";
import { test } from "node:test";
import { installDom } from "./dom-shim.mjs";
import { createContext, bind } from "../src/core.mjs";
import { box } from "../src/boxes.mjs";
import { emit, registerEmits, eventPropName } from "../src/emits.mjs";

installDom();

const tick = () => new Promise((r) => setTimeout(r, 0));

test("eventPropName maps names to on<Name>", () => {
  assert.equal(eventPropName("save"), "onSave");
  assert.equal(eventPropName("close"), "onClose");
  assert.equal(eventPropName("save-all"), "onSaveAll");
  assert.equal(eventPropName("update-model-value"), "onUpdateModelValue");
});

test("emit invokes the parent handler with payload", () => {
  const c = createContext(document.createElement("div"));
  let got = null;
  registerEmits(c, { onSave: (p) => (got = p) });
  const ran = emit(c, "save", { id: 7 });
  assert.equal(ran, true);
  assert.deepEqual(got, { id: 7 });
});

test("emit with no listener is a no-op returning false", () => {
  const c = createContext(document.createElement("div"));
  registerEmits(c, {}); // no handlers
  assert.equal(emit(c, "save", 1), false);
});

test("emit with no props registered (no parent) is a no-op", () => {
  const c = createContext(document.createElement("div"));
  // never called registerEmits
  assert.equal(emit(c, "anything", 1), false);
});

test("child emit does NOT mark the parent dirty by itself", async () => {
  // Parent has a reactive box + a bind reading it. The child's emit handler is
  // a plain function that reads the payload but does not mutate parent state:
  // no parent flush should occur.
  const parent = createContext(document.createElement("div"));
  const v = box(parent, 0, "init");
  let binds = 0;
  bind(parent, [0], () => {
    v.v; // read
    binds++;
  });
  assert.equal(binds, 1); // initial

  const child = createContext(document.createElement("span"));
  registerEmits(child, {
    onPing: () => {
      /* handler that touches nothing reactive on parent */
    },
  });
  emit(child, "ping", {});
  await tick();
  assert.equal(binds, 1); // parent never re-ran

  // But if the handler DOES mutate parent state, the box setter marks parent.
  registerEmits(child, { onPing: () => (v.v = "changed") });
  emit(child, "ping", {});
  await tick();
  assert.equal(binds, 2); // handler decided to update
});

test("emits validation warns on undeclared event but still runs handler", () => {
  const c = createContext(document.createElement("div"));
  let warned = null;
  const orig = console.warn;
  console.warn = (m) => (warned = m);
  try {
    let ran = false;
    registerEmits(c, { onFoo: () => (ran = true) }, ["bar"]); // only "bar" declared
    emit(c, "foo", 1);
    assert.ok(warned && /undeclared/.test(warned));
    assert.equal(ran, true); // still fires
  } finally {
    console.warn = orig;
  }
});
