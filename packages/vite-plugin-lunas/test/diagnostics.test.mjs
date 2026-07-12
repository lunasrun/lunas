// diagnostics.test.mjs — byte-offset → line/column mapping and code frames.
// Run: node packages/vite-plugin-lunas/test/diagnostics.test.mjs

import { test } from "node:test";
import assert from "node:assert";

import { byteOffsetToPosition, codeFrame, toRollupError } from "../src/diagnostics.mjs";

test("offset 0 is line 1, column 1", () => {
  const p = byteOffsetToPosition("hello", 0);
  assert.deepStrictEqual(p, { line: 1, column: 1, character: 0 });
});

test("maps an offset on the second line", () => {
  const src = "html:\nX";
  const p = byteOffsetToPosition(src, 6); // just after the newline
  assert.strictEqual(p.line, 2);
  assert.strictEqual(p.column, 1);
});

test("column counts characters within a line", () => {
  const src = "abcde";
  assert.strictEqual(byteOffsetToPosition(src, 3).column, 4);
});

test("multi-byte UTF-8 offsets account for byte length", () => {
  // "あ" is 3 UTF-8 bytes. A diagnostic at byte 3 lands after it (column 2).
  const src = "あx";
  const p = byteOffsetToPosition(src, 3);
  assert.strictEqual(p.line, 1);
  assert.strictEqual(p.column, 2);
});

test("codeFrame shows the offending line and a caret", () => {
  const frame = codeFrame("html:\n  <bad", 2, 3);
  assert.ok(frame.includes("  <bad"));
  assert.ok(frame.includes("^"));
});

test("toRollupError builds a loc + frame", () => {
  const src = "html:\n  <p>${x}</p>";
  const e = toRollupError(
    { message: "x is not reactive", severity: "warning", start: 8, end: 9 },
    "/src/App.lunas",
    src
  );
  assert.strictEqual(e.message, "x is not reactive");
  assert.strictEqual(e.id, "/src/App.lunas");
  assert.strictEqual(e.loc.line, 2);
  assert.ok(e.frame.includes("^"));
});
