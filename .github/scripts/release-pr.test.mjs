// Dependency-free unit tests for the release-PR body generator.
// Run: node .github/scripts/release-pr.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  buildBody,
  parseCheckedNumbers,
  prNumberFromSubject,
} from "./release-pr.mjs";

const FIXTURE = [
  { number: 142, login: "alice", labels: ["feature"] },
  { number: 133, login: "bob", labels: ["fix"] },
  { number: 140, login: "carol", labels: ["refactor"] },
  { number: 138, login: "dave", labels: ["version"] },
  { number: 135, login: "erin", labels: ["chore"] },
  { number: 136, login: "frank", labels: ["documentation"] }, // -> Others
  { number: 137, login: "grace", labels: [] }, // -> Others
];

test("buildBody categorizes into all sections, sorted ascending", () => {
  const body = buildBody(FIXTURE, { date: "2026-07-05" });
  const expected = [
    "🚀 Release (2026-07-05 - beta)",
    "## 🛠 Fixes",
    "- [ ] #133 @bob",
    "## ✨ New Features",
    "- [ ] #142 @alice",
    "## 🔨 Refactoring",
    "- [ ] #140 @carol",
    "## 📌 Version Updates",
    "- [ ] #138 @dave",
    "## 🔧 Chore",
    "- [ ] #135 @erin",
    "## 👻 Others",
    "- [ ] #136 @frank",
    "- [ ] #137 @grace",
    "",
  ].join("\n");
  assert.equal(body, expected);
});

test("empty categories still render their heading (matches ERB)", () => {
  const body = buildBody([{ number: 1, login: "x", labels: ["feature"] }], {
    date: "2026-01-01",
  });
  assert.match(body, /## 🛠 Fixes\n## ✨ New Features\n- \[ \] #1 @x/);
  assert.match(body, /## 👻 Others\n$/);
});

test("parseCheckedNumbers extracts checked PR numbers", () => {
  const body = [
    "🚀 Release (2026-07-05 - beta)",
    "## ✨ New Features",
    "- [x] #142 @alice",
    "- [ ] #133 @bob",
    "- [X] #140 @carol",
  ].join("\n");
  const checked = parseCheckedNumbers(body);
  assert.deepEqual([...checked].sort((a, b) => a - b), [140, 142]);
});

test("buildBody preserves checked checkboxes", () => {
  const checked = parseCheckedNumbers("- [x] #142 @alice\n- [ ] #133 @bob");
  const body = buildBody(FIXTURE, { date: "2026-07-05", checked });
  assert.match(body, /- \[x\] #142 @alice/);
  assert.match(body, /- \[ \] #133 @bob/);
});

test("parseCheckedNumbers on empty/undefined body is empty", () => {
  assert.equal(parseCheckedNumbers("").size, 0);
  assert.equal(parseCheckedNumbers(undefined).size, 0);
});

test("prNumberFromSubject handles squash and merge subjects", () => {
  assert.equal(
    prNumberFromSubject("feat(runtime): async components + suspense (#142)"),
    142,
  );
  assert.equal(
    prNumberFromSubject("Merge pull request #124 from lunasrun/rewrite"),
    124,
  );
  assert.equal(prNumberFromSubject("chore: no pr number here"), null);
  assert.equal(prNumberFromSubject(""), null);
});

test("PR with multiple category labels appears in each matching section", () => {
  const body = buildBody([{ number: 5, login: "z", labels: ["fix", "chore"] }], {
    date: "2026-07-05",
  });
  const fixes = body.indexOf("## 🛠 Fixes");
  const chore = body.indexOf("## 🔧 Chore");
  assert.ok(body.slice(fixes, chore).includes("- [ ] #5 @z"));
  assert.ok(body.slice(chore).includes("- [ ] #5 @z"));
  // Not in Others.
  assert.ok(!body.slice(body.indexOf("## 👻 Others")).includes("#5"));
});
