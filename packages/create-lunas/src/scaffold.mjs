// scaffold.mjs — copy the bundled template/ into a target directory and rewrite
// the package name. Pure filesystem logic, no prompting, so it is unit-testable.
//
// No dependencies beyond node builtins.

import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
export const TEMPLATE_DIR = resolve(here, "..", "template");

// npm package names: lowercase, no spaces, url-safe. Good enough for scaffolding.
export function normalizeName(raw) {
  return String(raw)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-~._]+/g, "-")
    .replace(/^[-_.]+|[-_.]+$/g, "") || "lunas-app";
}

// Is `dir` a safe scaffold target? Missing or empty is fine; a non-empty
// existing directory is refused (we never overwrite user files).
export function isEmptyDir(dir) {
  if (!existsSync(dir)) return true;
  return readdirSync(dir).length === 0;
}

// Scaffold the template into `targetDir`, rewriting the project's package name.
// Returns the list of files written (relative paths), sorted.
export function scaffold({ targetDir, projectName }) {
  const dir = resolve(targetDir);
  if (!isEmptyDir(dir)) {
    throw new Error(
      `target directory is not empty: ${dir}\n` +
        "refusing to overwrite existing files."
    );
  }
  const name = normalizeName(projectName);

  mkdirSync(dir, { recursive: true });
  // Copy the whole template tree.
  cpSync(TEMPLATE_DIR, dir, { recursive: true });

  // Rewrite the package name in package.json.
  const pkgPath = join(dir, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  pkg.name = name;
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

  return listFiles(dir);
}

// Recursively list files under `dir`, as sorted paths relative to `dir`.
export function listFiles(dir) {
  const out = [];
  const root = resolve(dir);
  const walk = (cur, prefix) => {
    for (const entry of readdirSync(cur, { withFileTypes: true })) {
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(join(cur, entry.name), rel);
      } else {
        out.push(rel);
      }
    }
  };
  walk(root, "");
  return out.sort();
}
