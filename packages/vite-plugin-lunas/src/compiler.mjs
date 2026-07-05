// compiler.mjs — resolves the compiler the plugin uses.
//
// The plugin needs an object with a `compile(source) -> { code, diagnostics }`
// method. Two ways to provide it:
//
//   1. Inject one via `options.compiler` (used by tests and by anyone wiring a
//      custom / published compiler build). Takes priority — no wasm is loaded.
//
//   2. Default: lazy-load the `wasm-pack --target nodejs` build of the
//      `lunas_wasm` crate. We look for it, in order:
//        a. `options.wasmPkgPath`   — explicit path to the pkg dir or its entry
//        b. `process.env.LUNAS_WASM_PKG`
//        c. the in-repo build at `crates/lunas_wasm/pkg` (dev default)
//
// Loading is lazy (first `.lunas` transform) and cached, so a project that
// injects a compiler never touches the filesystem for wasm.

import { createRequire } from "node:module";
import { existsSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, isAbsolute, join, resolve } from "node:path";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));

// crates/lunas_wasm/pkg relative to this file
// (packages/vite-plugin-lunas/src -> repo root -> crates/lunas_wasm/pkg).
const REPO_DEFAULT_PKG = resolve(here, "..", "..", "..", "crates", "lunas_wasm", "pkg");

// Candidate entry-file names inside a wasm-pack `--target nodejs` output dir.
const ENTRY_CANDIDATES = ["lunas_wasm.js", "lunas_wasm.cjs"];

// Resolve a user-supplied path (dir or file) to a require-able entry file.
function resolveEntry(p) {
  const abs = isAbsolute(p) ? p : resolve(process.cwd(), p);
  if (!existsSync(abs)) return null;
  if (statSync(abs).isDirectory()) {
    for (const name of ENTRY_CANDIDATES) {
      const candidate = join(abs, name);
      if (existsSync(candidate)) return candidate;
    }
    return null;
  }
  return abs;
}

function firstExistingEntry(candidates) {
  for (const c of candidates) {
    if (!c) continue;
    const entry = resolveEntry(c);
    if (entry) return entry;
  }
  return null;
}

// Build a compiler backed by the wasm-pack nodejs build. Throws a helpful error
// if the pkg cannot be found or loaded.
function loadWasmCompiler(wasmPkgPath) {
  const entry = firstExistingEntry([
    wasmPkgPath,
    process.env.LUNAS_WASM_PKG,
    REPO_DEFAULT_PKG,
  ]);

  if (!entry) {
    throw new Error(
      "[vite-plugin-lunas] could not find the lunas_wasm build. Build it with " +
        "`wasm-pack build --target nodejs` in crates/lunas_wasm, then point the " +
        "plugin at it via the `wasmPkgPath` option or the LUNAS_WASM_PKG env var. " +
        "(Looked at: " +
        [wasmPkgPath, process.env.LUNAS_WASM_PKG, REPO_DEFAULT_PKG]
          .filter(Boolean)
          .join(", ") +
        ")"
    );
  }

  let mod;
  try {
    mod = require(entry);
  } catch (err) {
    throw new Error(
      "[vite-plugin-lunas] failed to load the lunas_wasm build at " +
        entry +
        ": " +
        (err && err.message ? err.message : String(err))
    );
  }

  if (typeof mod.compile !== "function") {
    throw new Error(
      "[vite-plugin-lunas] the module at " +
        entry +
        " does not export a `compile` function."
    );
  }
  return { compile: (source) => mod.compile(source) };
}

// Returns a getter that resolves the compiler once and caches it.
// `options.compiler` short-circuits everything (no lazy loading).
export function makeCompilerLoader(options) {
  const injected = options && options.compiler;
  if (injected) {
    if (typeof injected.compile !== "function") {
      throw new Error(
        "[vite-plugin-lunas] `options.compiler` must have a `compile(source)` method."
      );
    }
    return () => injected;
  }

  let cached = null;
  const wasmPkgPath = options && options.wasmPkgPath;
  return () => {
    if (!cached) cached = loadWasmCompiler(wasmPkgPath);
    return cached;
  };
}
