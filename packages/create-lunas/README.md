# create-lunas

Scaffold a new [Lunas](https://github.com/lunasrun/lunas) + [Vite](https://vitejs.dev)
project.

## Usage

```sh
npm create lunas@latest my-app
# or
npm create lunas@latest        # prompts for a project name
```

Then:

```sh
cd my-app
npm install
npm run dev
```

## What you get

A minimal Vite app wired up with `vite-plugin-lunas`:

- `index.html` + `src/main.mjs` — mounts the app into `#app` via `attach`.
- `src/App.lunas` — a root component demonstrating reactive state, an event
  handler, and `:if` / `:for` in the template.
- `vite.config.mjs` — the Lunas Vite plugin.

The scaffolder copies the bundled `template/` into your target directory
(refusing to overwrite a non-empty one) and rewrites the project's package name.

## Development

```sh
npm test   # scaffolds into a temp dir and asserts the file set + name rewrite
```

The template's `App.lunas` is also compiled by the Rust test suite
(`crates/lunas_compiler/tests/scaffold_template.rs`) to guarantee it stays valid
for the current compiler.
