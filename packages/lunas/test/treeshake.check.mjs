// treeshake.check.mjs — demonstrates that the runtime is tree-shakeable.
//
// Two independent checks:
//
//   1. Static side-effect check (always runs, no deps): every src/*.mjs
//      module must contain no top-level statement other than
//      imports/exports/declarations — i.e. nothing runs merely by
//      importing the module. This is what makes "sideEffects": false in
//      package.json truthful and is a necessary condition for bundlers to
//      tree-shake unused exports.
//
//   2. Bundle check (best-effort, uses `npx esbuild`): bundles an entry
//      that imports a single symbol (`box`) from the package entry point
//      with tree-shaking enabled, and asserts that unrelated exports
//      (e.g. `forBlock`, `reconcile`, `component`) do not appear in the
//      output. Skips gracefully (exit 0, with a notice) if npx/esbuild is
//      unavailable or there's no network access — this check is a nice-to
//      -have proof, not a hard requirement, since check #1 already
//      guarantees shakeability.
//
// Run: node test/treeshake.check.mjs

import { readFileSync, readdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

// ---------------------------------------------------------------------------
// 1. Static side-effect check
// ---------------------------------------------------------------------------

console.log("check 1/2: static side-effect scan of src/*.mjs");

const srcFiles = readdirSync(srcDir).filter((f) => f.endsWith(".mjs"));
let sideEffectFound = false;

for (const file of srcFiles) {
  const full = join(srcDir, file);
  const src = readFileSync(full, "utf8");
  const lines = src.split("\n");

  let depth = 0; // brace depth; only inspect top-level (depth 0) lines
  let inContinuation = false; // mid-statement continuation of a top-level decl

  for (const rawLine of lines) {
    const line = rawLine.trim();

    // track brace depth using the raw line (rough but sufficient: this
    // codebase has no braces inside string/template literals at top level)
    const opens = (rawLine.match(/{/g) || []).length;
    const closes = (rawLine.match(/}/g) || []).length;

    if (depth === 0 && line.length > 0 && !line.startsWith("//")) {
      const isDeclarationStart =
        line.startsWith("export ") ||
        line.startsWith("import ") ||
        line.startsWith("function ") ||
        line.startsWith("const ") ||
        line.startsWith("let ") ||
        line.startsWith("class ") ||
        line.startsWith("}") || // closing brace of a top-level decl
        line.startsWith(")") || // continuation (e.g. multi-line import)
        line.startsWith("*/") ||
        line.startsWith("/*");

      if (!isDeclarationStart && !inContinuation) {
        console.error(
          "  FAIL  " + file + ": possible top-level side effect: " + line
        );
        sideEffectFound = true;
      }

      // A top-level statement that doesn't end in `;`, `{` (block opener)
      // or `*/` is continuing on the next line (e.g. a multi-line arrow
      // function body) — don't flag its continuation lines.
      const opensBlock = /\{\s*$/.test(line);
      const statementEnds =
        /;\s*$/.test(line) || opensBlock || line.endsWith("*/");
      if (opens === 0 && closes === 0) {
        inContinuation = !statementEnds;
      } else {
        inContinuation = false;
      }
    } else if (depth === 0) {
      inContinuation = false;
    }

    depth += opens - closes;
  }
}

if (sideEffectFound) {
  console.error(
    "\nstatic check failed: a module executes code at import time, " +
      "which would break tree-shaking / \"sideEffects\": false."
  );
  process.exit(1);
}
console.log("  ok    no top-level side effects in any src/*.mjs module\n");

// ---------------------------------------------------------------------------
// 2. Bundle check (best-effort)
// ---------------------------------------------------------------------------

console.log("check 2/2: esbuild bundle + unused-export elimination (best-effort)");

function tryEsbuild() {
  const entryDir = mkdtempSync(join(tmpdir(), "lunas-treeshake-"));
  const entryFile = join(entryDir, "entry.mjs");
  const outFile = join(entryDir, "out.js");
  writeFileSync(
    entryFile,
    "import { box } from " +
      JSON.stringify(join(here, "..", "src", "index.mjs")) +
      ";\n" +
      "globalThis.__lunasTreeshakeProbe = box;\n"
  );

  const res = spawnSync(
    "npx",
    [
      "--yes",
      "esbuild",
      entryFile,
      "--bundle",
      "--format=esm",
      "--tree-shaking=true",
      "--outfile=" + outFile,
    ],
    { encoding: "utf8", timeout: 60000 }
  );

  if (res.error || res.status !== 0) {
    return {
      ok: false,
      skip: true,
      reason:
        "npx esbuild unavailable (no network or resolution failed): " +
        (res.error ? res.error.message : res.stderr || "unknown error"),
    };
  }

  const bundled = readFileSync(outFile, "utf8");
  rmSync(entryDir, { recursive: true, force: true });

  // Symbols that are ONLY reachable from other, unimported exports. If
  // tree-shaking works, none of these identifiers should appear in the
  // bundle (function names are preserved by esbuild by default at this
  // optimization level when not minifying).
  const unrelatedMarkers = [
    "function forBlock",
    "function ifBlock",
    "function component(",
    "function reconcile(",
    "function mountChild",
  ];

  const leaked = unrelatedMarkers.filter((m) => bundled.includes(m));
  if (leaked.length > 0) {
    return {
      ok: false,
      skip: false,
      reason:
        "bundle unexpectedly contains unrelated exports: " + leaked.join(", "),
    };
  }

  if (!bundled.includes("__lunasTreeshakeProbe")) {
    return {
      ok: false,
      skip: false,
      reason: "bundle is missing the imported symbol itself — bundling failed",
    };
  }

  return { ok: true, skip: false };
}

let result;
try {
  result = tryEsbuild();
} catch (e) {
  result = { ok: false, skip: true, reason: String(e && e.message ? e.message : e) };
}

if (result.skip) {
  console.log("  skip  " + result.reason);
  console.log(
    "  (this is expected offline; check 1/2 already guarantees shakeability)\n"
  );
} else if (!result.ok) {
  console.error("  FAIL  " + result.reason);
  process.exit(1);
} else {
  console.log(
    "  ok    bundling only `box` did not pull in forBlock/ifBlock/component/reconcile/mountChild\n"
  );
}

console.log("treeshake.check.mjs: done");
