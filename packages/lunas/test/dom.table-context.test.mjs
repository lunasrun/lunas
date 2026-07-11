// dom.table-context.test.mjs — the table-context fragment-parsing fix.
//
// A `:for` item / `:if` branch / fragment whose skeleton is a table-context
// element (`<tr>`, `<td>`, `<tbody>`, `<option>`, …) must survive being parsed
// as a detached fragment. Parsing such HTML as the innerHTML of a `<div>` makes
// a real browser DROP the table tags (a `<div>` is not a valid table insertion
// context), so the parsed container is empty and `childNodes[0]` is undefined —
// the crash this fix addresses. parseFragment / fromHTML now parse through a
// `<template>`, whose content fragment is a valid insertion context for any
// element, so the row survives.
//
// The dom-shim models the browser's drop: a `<div>`-context parse of `<tr>`
// drops it, a `<template>`-context parse keeps it. These tests assert both the
// shim's fidelity (a plain `<div>` parse drops table tags) and the runtime fix
// (parseFragment/fromHTML/forBlock keep them).
// Run: node packages/lunas/test/dom.table-context.test.mjs

import assert from "node:assert";
import { installDom } from "./dom-shim.mjs";
import { parseFragment, fromHTML } from "../src/dom.mjs";

installDom();

let passed = 0;
async function test(name, fn) {
  await fn();
  passed++;
  console.log("  ok  " + name);
}

// --- shim fidelity: a <div> parse drops table-context elements ---------------

await test("shim: <tr> assigned as <div> innerHTML is dropped (browser parity)", () => {
  const div = document.createElement("div");
  div.innerHTML = "<tr><td>x</td></tr>";
  // Real browser drops the row (and its subtree) -> empty container.
  assert.strictEqual(div.childNodes.length, 0);
  assert.strictEqual(div.childNodes[0], undefined);
});

await test("shim: <tr> is kept inside a valid table context", () => {
  const table = document.createElement("table");
  table.innerHTML = "<tbody><tr><td>x</td></tr></tbody>";
  const tbody = table.childNodes[0];
  assert.strictEqual(tbody.tag, "tbody");
  assert.strictEqual(tbody.childNodes[0].tag, "tr");
});

// --- shim: <template> content is a permissive parse context ------------------

await test("shim: <template>.content keeps a bare <tr>", () => {
  const t = document.createElement("template");
  assert.ok(t.content, "template exposes .content");
  t.innerHTML = "<tr><td>x</td></tr>";
  // The <template> element itself stays empty; content holds the row.
  assert.strictEqual(t.childNodes.length, 0);
  assert.strictEqual(t.content.childNodes[0].tag, "tr");
  assert.strictEqual(t.content.childNodes[0].childNodes[0].tag, "td");
});

// --- the fix: parseFragment / fromHTML keep table-context content ------------

await test("parseFragment: bare <tr> skeleton survives -> childNodes[0] is the row", () => {
  const scr = parseFragment("<tr><td class=\"c\">a</td></tr>", document);
  const root = scr.childNodes[0];
  assert.ok(root, "root is defined (no table-context drop)");
  assert.strictEqual(root.tag, "tr");
  assert.strictEqual(root.childNodes[0].tag, "td");
  assert.strictEqual(root.childNodes[0].getAttribute("class"), "c");
});

await test("parseFragment: bulk (multi-row) skeleton keeps every row", () => {
  const scr = parseFragment("<tr><td>a</td></tr><tr><td>b</td></tr>", document);
  assert.strictEqual(scr.childNodes.length, 2);
  assert.strictEqual(scr.childNodes[0].tag, "tr");
  assert.strictEqual(scr.childNodes[1].tag, "tr");
});

await test("parseFragment: <option> (select context) survives too", () => {
  const scr = parseFragment("<option value=\"1\">one</option>", document);
  assert.strictEqual(scr.childNodes[0].tag, "option");
  assert.strictEqual(scr.childNodes[0].getAttribute("value"), "1");
});

await test("fromHTML: table-context branch skeleton survives (:if / :for path)", () => {
  const near = document.createElement("table");
  const scr = fromHTML("<tr><td>x</td></tr>", near);
  assert.strictEqual(scr.childNodes[0].tag, "tr");
});

await test("parseFragment: non-table skeleton still parses normally", () => {
  const scr = parseFragment("<li><span>a</span></li>", document);
  assert.strictEqual(scr.childNodes[0].tag, "li");
  assert.strictEqual(scr.childNodes[0].childNodes[0].tag, "span");
});

console.log("dom.table-context.test.mjs: all " + passed + " tests passed");
