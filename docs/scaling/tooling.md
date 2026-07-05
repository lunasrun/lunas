# Tooling

Lunas's build toolchain has three pieces:

- **`create-lunas`** — the scaffolding CLI that generates a new project.
- **`vite-plugin-lunas`** — the Vite plugin that compiles `.lunas` / `.lun`
  single-file components.
- **`lunas_wasm`** — the WebAssembly binding that lets JavaScript run the real
  Rust compiler.

## Scaffolding a project: `create-lunas`

```sh
npm create lunas@latest my-app
# or, to be prompted for a name:
npm create lunas@latest
```

Then:

```sh
cd my-app
npm install
npm run dev
```

You get a minimal Vite app already wired up with `vite-plugin-lunas`:

- `index.html` + `src/main.mjs` — mounts the app into `#app` via `attach`.
- `src/App.lunas` — a root component demonstrating reactive state, an event
  handler, and `:if` / `:for` in the template.
- `vite.config.mjs` — the Lunas Vite plugin.

The scaffolder copies its bundled `template/` into your target directory
(**refusing to overwrite a non-empty one**) and rewrites the project's package
name. The template's `App.lunas` is compiled by the Rust test suite on every
build, so it's guaranteed to stay valid for the current compiler.

## Compiling components: `vite-plugin-lunas`

### Setup

```sh
npm install -D vite-plugin-lunas
npm install lunas
```

`vite` is a peer dependency (`>=5`).

```js
// vite.config.mjs
import { defineConfig } from "vite";
import lunas from "vite-plugin-lunas";

export default defineConfig({
  plugins: [lunas()],
});
```

Import components like any other module — the default export is the compiled
component factory:

```js
import { attach } from "lunas";
import App from "./App.lunas";

attach(App(), document.getElementById("app"));
```

The emitted module imports its runtime from the `lunas` package, so the runtime
is shared and tree-shaken normally by Vite/Rollup.

### Options

| Option | Type | Default | Description |
|---|---|---|---|
| `extensions` | `string[]` | `[".lunas", ".lun"]` | File extensions the plugin handles. |
| `compiler` | `{ compile }` | — | Inject a compiler `{ compile(source) => { code, diagnostics } }`. Skips wasm loading entirely (used by tests / custom builds). |
| `wasmPkgPath` | `string` | — | Path to a `wasm-pack --target nodejs` build of `lunas_wasm` (its `pkg` dir or entry file). Overrides `LUNAS_WASM_PKG` and the in-repo default. |

### How the compiler is loaded

The plugin needs a compiler exposing `compile(source) -> { code, diagnostics }`.
It resolves one in this order:

1. If `options.compiler` is given, it's used directly (**no wasm is loaded**).
2. Otherwise the plugin **lazy-loads** the `lunas_wasm` wasm-pack
   (`--target nodejs`) build on the first `.lunas` transform, searching:
   1. `options.wasmPkgPath`
   2. the `LUNAS_WASM_PKG` environment variable
   3. `crates/lunas_wasm/pkg` (the in-repo dev default)

Build the wasm compiler with:

```sh
cd crates/lunas_wasm
wasm-pack build --target nodejs
```

### Diagnostics

Compiler **errors** abort the build via Rollup's `this.error`, with a
file/line/column and a one-line code frame. **Warnings** and **hints** surface
via `this.warn` and do not stop compilation. Positions are computed from the
compiler's UTF-8 byte offsets.

### HMR behavior — honest scope

A change to a `.lunas` / `.lun` file triggers a **full page reload** of the
modules that import it.

> **Finer-grained HMR** — patching a live component in place without a reload,
> preserving its state — is **future work**. Today, editing a component reloads
> the page; your app's runtime state is lost on each edit. This is functional but
> coarser than the module-hot-swap you may be used to from mature frameworks.

## The compiler binding: `lunas_wasm`

`lunas_wasm` is a thin `wasm-bindgen` binding over the Lunas compiler
(`lunas_compiler`). It's **bindings only** — all logic lives in the compiler
crate. It exists so a bundler plugin or a browser playground can run the real
compiler with no native toolchain.

```js
import { compile, version } from "lunas_wasm";

const { code, diagnostics } = compile(source);
// code:        string | null   — the emitted ES module (null on failure)
// diagnostics: Array<{ message, severity, start, end }>
//   severity:  "error" | "warning" | "hint"
//   start/end: UTF-8 byte offsets into `source`
```

`compile` **never throws** for compiler problems — they come back as
`diagnostics`. `code` is `null` when there's an error (or nothing to emit).

### Building the wasm

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and the
`wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).
Build from `crates/lunas_wasm`:

```sh
# Node target (what the Vite plugin loads by default) → ./pkg
wasm-pack build --target nodejs

# Browser/bundler target (playground / web build) → ./pkg-web
wasm-pack build --target web --out-dir pkg-web
```

Each build produces a JS wrapper + `lunas_wasm_bg.wasm`. `pkg/` and `pkg-web/`
are build artifacts and are git-ignored.

## The build workflow at a glance

```
.lunas source
   │  (vite-plugin-lunas transform)
   ▼
lunas_wasm.compile(source)  →  { code, diagnostics }
   │  errors → this.error (abort);  warnings/hints → this.warn
   ▼
ES module (imports runtime from `lunas`)
   │  (Vite / Rollup bundle + tree-shake)
   ▼
your app bundle
```

1. `create-lunas` scaffolds the project (once).
2. `npm run dev` / `npm run build` runs Vite; `vite-plugin-lunas` intercepts
   `.lunas` / `.lun` imports.
3. The plugin calls `lunas_wasm.compile` (or your injected compiler), turning each
   component into an ES module.
4. Vite bundles the modules with the tree-shakeable `lunas` runtime.

## Gotchas

- **The wasm build must exist** (or a compiler must be injected) before the first
  `.lunas` transform. In a fresh checkout, run `wasm-pack build --target nodejs`
  in `crates/lunas_wasm`, or point `wasmPkgPath` / `LUNAS_WASM_PKG` at a build.
- **Diagnostics are byte offsets**, not line/column. The Vite plugin computes
  line/column for its code frames; a raw `lunas_wasm` consumer builds its own line
  index if it needs editor squiggles.
- **HMR is a full reload today** — don't expect in-place state preservation on
  edit (see above).
- **`extensions` is configurable** — if you use only `.lunas`, you can narrow it,
  but the default already covers both `.lunas` and `.lun`.

## See also

- [Scoped CSS](./scoped-css.md) — note the CSS codegen wiring is a pending
  integration task.
- [SSR](./ssr.md) — the build is client-only today; SSR is deferred.
- `packages/vite-plugin-lunas/README.md`, `packages/create-lunas/README.md`,
  `crates/lunas_wasm/README.md` — the authoritative package references.
