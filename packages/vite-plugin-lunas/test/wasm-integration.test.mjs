// wasm-integration.test.mjs — opt-in end-to-end test: build the real
// lunas_wasm compiler (if wasm-pack is available), then run a fixture .lunas
// through the plugin's default (wasm-backed) path.
//
// Skips gracefully — never fails — when:
//   - wasm-pack is not installed, or the pkg build is absent, or
//   - the local node cannot load the wasm module (older node without
//     reference-types support; CI's node 22 handles it).
//
// Run: node packages/vite-plugin-lunas/test/wasm-integration.test.mjs

import { test } from "node:test";
import assert from "node:assert";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

import lunas from "../src/index.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..", "..");
const wasmCrate = resolve(repoRoot, "crates", "lunas_wasm");
const pkgEntry = resolve(wasmCrate, "pkg", "lunas_wasm.js");
const templateApp = resolve(
  repoRoot,
  "packages",
  "create-lunas",
  "template",
  "src",
  "App.lunas"
);

function hasWasmPack() {
  const r = spawnSync("wasm-pack", ["--version"], { encoding: "utf8" });
  return r.status === 0;
}

// Ensure the pkg exists; build it once if wasm-pack is available.
function ensurePkg() {
  if (existsSync(pkgEntry)) return true;
  if (!hasWasmPack()) return false;
  const r = spawnSync("wasm-pack", ["build", "--target", "nodejs"], {
    cwd: wasmCrate,
    stdio: "inherit",
  });
  return r.status === 0 && existsSync(pkgEntry);
}

test("real wasm compiler compiles the scaffold template via the plugin", async (t) => {
  if (!ensurePkg()) {
    t.skip("lunas_wasm pkg unavailable (wasm-pack not installed / build failed)");
    return;
  }

  // The default plugin path lazy-loads crates/lunas_wasm/pkg. Guard the load:
  // an older node may reject the wasm (reference-types), which is an
  // environment limitation, not a plugin bug — skip in that case.
  let plugin;
  try {
    // Touch the module once to surface a load error before running transform.
    require(pkgEntry);
    plugin = lunas();
  } catch (err) {
    t.skip("node cannot load the wasm module: " + (err && err.message));
    return;
  }

  const source = readFileSync(templateApp, "utf8");
  const ctx = {
    warn() {},
    error(e) {
      throw new Error(typeof e === "string" ? e : e.message);
    },
  };

  const out = plugin.transform.call(ctx, source, templateApp);
  assert.ok(out, "transform should return output");
  assert.ok(typeof out.code === "string" && out.code.length > 0);
  assert.ok(
    out.code.includes('from "lunas"'),
    "emitted module should import the lunas runtime"
  );
});
