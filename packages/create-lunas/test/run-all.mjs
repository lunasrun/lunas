// run-all.mjs — test driver: discovers every *.test.mjs in this directory and
// runs each in its own child process. Mirrors packages/lunas/test/run-all.mjs.

import { readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));

const files = readdirSync(here)
  .filter((f) => f.endsWith(".test.mjs"))
  .sort();

if (files.length === 0) {
  console.error("run-all: no *.test.mjs files found in " + here);
  process.exit(1);
}

let failed = 0;

for (const file of files) {
  const full = join(here, file);
  console.log("\n> node test/" + file);
  const res = spawnSync(process.execPath, [full], { stdio: "inherit" });
  if (res.status !== 0) {
    failed++;
    console.error("FAIL  " + file + " (exit " + res.status + ")");
  }
}

console.log("");
if (failed > 0) {
  console.error(failed + "/" + files.length + " test file(s) failed.");
  process.exit(1);
}

console.log("all " + files.length + " test file(s) passed.");
