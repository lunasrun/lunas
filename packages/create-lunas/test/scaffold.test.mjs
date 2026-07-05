// scaffold.test.mjs — scaffold into a temp dir; assert the file set and the
// package-name rewrite. Run: node packages/create-lunas/test/scaffold.test.mjs

import { test } from "node:test";
import assert from "node:assert";
import { mkdtempSync, readFileSync, writeFileSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  scaffold,
  normalizeName,
  isEmptyDir,
  listFiles,
  TEMPLATE_DIR,
} from "../src/scaffold.mjs";

function tmp() {
  return mkdtempSync(join(tmpdir(), "create-lunas-"));
}

test("normalizeName sanitizes to a valid npm name", () => {
  assert.strictEqual(normalizeName("My App"), "my-app");
  assert.strictEqual(normalizeName("  Foo/Bar  "), "foo-bar");
  assert.strictEqual(normalizeName("Ok-Name_1.2"), "ok-name_1.2");
  assert.strictEqual(normalizeName(""), "lunas-app");
  assert.strictEqual(normalizeName("!!!"), "lunas-app");
});

test("scaffold writes the expected file set", () => {
  const base = tmp();
  const target = join(base, "my-app");
  const files = scaffold({ targetDir: target, projectName: "My App" });

  // Compare against the template's own file set (name rewrite doesn't add/remove
  // files), so this test stays correct if the template grows.
  const expected = listFiles(TEMPLATE_DIR);
  assert.deepStrictEqual(files, expected);

  // Spot-check the essential files are present.
  for (const f of ["package.json", "index.html", "vite.config.mjs", "src/main.mjs", "src/App.lunas"]) {
    assert.ok(files.includes(f), `expected ${f} in scaffold`);
  }
  rmSync(base, { recursive: true, force: true });
});

test("scaffold rewrites the package name", () => {
  const base = tmp();
  const target = join(base, "cool-thing");
  scaffold({ targetDir: target, projectName: "Cool Thing" });
  const pkg = JSON.parse(readFileSync(join(target, "package.json"), "utf8"));
  assert.strictEqual(pkg.name, "cool-thing");
  // Deps are preserved from the template.
  assert.ok(pkg.dependencies.lunas);
  assert.ok(pkg.devDependencies["vite-plugin-lunas"]);
  assert.ok(pkg.devDependencies.vite);
  rmSync(base, { recursive: true, force: true });
});

test("template deps use real semver ranges, not file: paths", () => {
  const pkg = JSON.parse(readFileSync(join(TEMPLATE_DIR, "package.json"), "utf8"));
  const all = { ...pkg.dependencies, ...pkg.devDependencies };
  for (const [name, range] of Object.entries(all)) {
    assert.ok(!range.startsWith("file:"), `${name} should not use a file: path`);
  }
});

test("scaffold refuses a non-empty target dir", () => {
  const base = tmp();
  const target = join(base, "occupied");
  mkdirSync(target, { recursive: true });
  writeFileSync(join(target, "keep.txt"), "hi");
  assert.throws(
    () => scaffold({ targetDir: target, projectName: "x" }),
    /not empty/
  );
  rmSync(base, { recursive: true, force: true });
});

test("isEmptyDir: missing and empty are ok, populated is not", () => {
  const base = tmp();
  assert.strictEqual(isEmptyDir(join(base, "nope")), true);
  const empty = join(base, "empty");
  mkdirSync(empty);
  assert.strictEqual(isEmptyDir(empty), true);
  writeFileSync(join(empty, "f"), "x");
  assert.strictEqual(isEmptyDir(empty), false);
  rmSync(base, { recursive: true, force: true });
});
