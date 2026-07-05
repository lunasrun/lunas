// run-samples.mjs — the SINGLE node process that runs every compiled sample
// case. The Rust runner compiles all cases, writes their emitted modules plus a
// manifest into a temp dir, and spawns this once. Batching keeps hundreds of
// cases fast (no per-case node spawn).
//
// Usage:  node run-samples.mjs <manifest.json>
//
// Manifest shape (written by the Rust runner):
//   {
//     "shimPath": "/abs/path/dom-shim.mjs",
//     "update": false,                       // UPDATE_EXPECTED mode
//     "cases": [
//       {
//         "name": "text/interpolation",
//         "dir": "/abs/path/to/tmp/text__interpolation",
//         "entry": "App.gen.mjs",            // compiled App factory module
//         "props": { ... } | null,
//         "hasSteps": true|false,
//         "expectedHtml": "..." | null,      // stored expected.html (null if absent)
//         "expectedAfterHtml": "..." | null  // stored expected.after.html
//       }, ...
//     ]
//   }
//
// Output: a JSON object on stdout, sandwiched between sentinel lines so any
// stray console output from a case does not corrupt the parse:
//   __LUNAS_RESULTS_BEGIN__
//   { "results": [ { name, status, message, initialHtml, afterHtml }, ... ] }
//   __LUNAS_RESULTS_END__
//
// status: "pass" | "fail" | "error". `initialHtml`/`afterHtml` are the freshly
// captured normalized HTML (used by the Rust side for UPDATE_EXPECTED regen).

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { join } from "node:path";

import { normalizeRoots } from "./normalize.mjs";
import { makeKit, tick } from "./kit.mjs";

const manifestPath = process.argv[2];
if (!manifestPath) {
  console.error("run-samples: missing manifest path argument");
  process.exit(2);
}
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

// Install the shared dom-shim ONCE for the whole batch.
const shim = await import(pathToFileURL(manifest.shimPath).href);
shim.installDom();

// The runtime's real mount path. `attach` fires onMount for a single root;
// `runMount` is used per-node for a multi-root fragment (array of nodes).
const runtime = await import(
  pathToFileURL(join(manifest.runtimeDir, "index.mjs")).href
);
const { attach } = runtime;
const lifecycle = await import(
  pathToFileURL(join(manifest.runtimeDir, "lifecycle.mjs")).href
);
const { runMount } = lifecycle;

// mount(factory, props) — normalize both component() (single node) and
// fragment() (array) into a mounted host, firing onMount via the REAL runtime
// attach path. Returns { roots }.
async function mount(factory, props) {
  // A fresh #portal target per case so teleport cases are isolated.
  const portal = document.createElement("div");
  portal.setAttribute("id", "portal");
  document.body.appendChild(portal);

  const host = document.createElement("div");
  document.body.appendChild(host);
  host._lunasAttached = true; // liveness marker so isConnected works

  const result = factory(props || {});
  const roots = Array.isArray(result) ? result : [result];
  if (Array.isArray(result)) {
    // Fragment: append every node, then fire onMount once on the shared ctx.
    for (const n of roots) {
      host.appendChild(n);
      n._lunasAttached = true;
    }
    const c = result.__lunasCtx;
    if (c) runMount(c);
  } else {
    attach(result, host); // single root: append + runMount
  }
  await tick();
  return { roots, host, portal };
}

const results = [];

for (const c of manifest.cases) {
  const out = {
    name: c.name,
    status: "pass",
    message: "",
    initialHtml: null,
    afterHtml: null,
  };
  try {
    const entryUrl = pathToFileURL(join(c.dir, c.entry)).href;
    const mod = await import(entryUrl);
    const factory = mod.default;
    if (typeof factory !== "function") {
      throw new Error("compiled module has no default-exported factory");
    }

    const { roots } = await mount(factory, c.props);
    const initialHtml = normalizeRoots(roots);
    out.initialHtml = initialHtml;

    // Initial-DOM check (unless updating, where we just capture).
    if (!manifest.update) {
      if (c.expectedHtml === null) {
        throw new Error(
          "expected.html is missing; run `UPDATE_EXPECTED=1` to generate it"
        );
      }
      if (initialHtml !== c.expectedHtml) {
        out.status = "fail";
        out.message =
          "initial DOM mismatch:\n" +
          `  expected: ${JSON.stringify(c.expectedHtml)}\n` +
          `  actual:   ${JSON.stringify(initialHtml)}`;
        results.push(out);
        continue;
      }
    }

    // Interaction steps.
    if (c.hasSteps) {
      const stepsUrl = pathToFileURL(join(c.dir, "steps.mjs")).href;
      const stepsMod = await import(stepsUrl);
      const run = stepsMod.default;
      if (typeof run !== "function") {
        throw new Error("steps.mjs has no default-exported function");
      }
      const kit = makeKit(roots);
      // `mount` lets a step remount or mount an extra factory if it wants.
      kit.mount = mount;
      await run(kit);
      // Capture the post-steps DOM for optional expected.after.html regen.
      out.afterHtml = normalizeRoots(roots);

      if (!manifest.update && c.expectedAfterHtml !== null) {
        if (out.afterHtml !== c.expectedAfterHtml) {
          out.status = "fail";
          out.message =
            "post-steps DOM mismatch (expected.after.html):\n" +
            `  expected: ${JSON.stringify(c.expectedAfterHtml)}\n` +
            `  actual:   ${JSON.stringify(out.afterHtml)}`;
          results.push(out);
          continue;
        }
      }
    }
  } catch (e) {
    out.status = "error";
    out.message = (e && e.stack) || String(e);
  }
  results.push(out);
}

process.stdout.write("__LUNAS_RESULTS_BEGIN__\n");
process.stdout.write(JSON.stringify({ results }));
process.stdout.write("\n__LUNAS_RESULTS_END__\n");
