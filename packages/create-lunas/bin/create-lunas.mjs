#!/usr/bin/env node
// create-lunas — scaffold a new Lunas + Vite project.
//
// Usage:
//   npm create lunas@latest my-app
//   npm create lunas@latest            # prompts for a project name
//
// Plain node ESM, no dependencies. Prompting falls back to a default when
// stdin is not a TTY (e.g. CI / piped input).

import { createInterface } from "node:readline";
import { resolve } from "node:path";

import { scaffold, normalizeName } from "../src/scaffold.mjs";

const DEFAULT_NAME = "lunas-app";

async function prompt(question, fallback) {
  if (!process.stdin.isTTY) return fallback;
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  try {
    const answer = await new Promise((res) => rl.question(question, res));
    const trimmed = answer.trim();
    return trimmed || fallback;
  } finally {
    rl.close();
  }
}

async function main() {
  // First non-flag CLI arg is the project directory/name.
  const arg = process.argv.slice(2).find((a) => !a.startsWith("-"));
  const projectName =
    arg || (await prompt(`Project name: (${DEFAULT_NAME}) `, DEFAULT_NAME));

  const targetDir = resolve(process.cwd(), projectName);
  const pkgName = normalizeName(projectName);

  let files;
  try {
    files = scaffold({ targetDir, projectName });
  } catch (err) {
    console.error("\n✖ " + (err && err.message ? err.message : String(err)));
    process.exit(1);
  }

  console.log(`\n✔ Scaffolded ${pkgName} into ${targetDir}`);
  console.log(`  ${files.length} files written.\n`);
  console.log("Next steps:");
  console.log(`  cd ${projectName}`);
  console.log("  npm install");
  console.log("  npm run dev\n");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
