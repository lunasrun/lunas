# vite-plugin-lunas

[Vite](https://vitejs.dev) plugin that compiles Lunas single-file components
(`.lunas` / `.lun`) into ES modules with the Lunas compiler. The emitted module
imports its runtime from the [`lunas`](https://github.com/lunasrun/lunas/tree/main/packages/lunas)
package.

## Install

```sh
npm install -D vite-plugin-lunas
npm install lunas
```

`vite` is a peer dependency (`>=5`).

## Usage

```js
// vite.config.mjs
import { defineConfig } from "vite";
import lunas from "vite-plugin-lunas";

export default defineConfig({
  plugins: [lunas()],
});
```

Import components as normal modules — their default export is the compiled
component factory:

```js
import { attach } from "lunas";
import App from "./App.lunas";

attach(App(), document.getElementById("app"));
```

## Options

| Option        | Type              | Default             | Description |
| ------------- | ----------------- | ------------------- | ----------- |
| `extensions`  | `string[]`        | `[".lunas", ".lun"]`| File extensions to handle. |
| `compiler`    | `{ compile }`     | —                   | Inject a compiler `{ compile(source) => { code, diagnostics } }`. Skips wasm loading entirely (used by tests / custom builds). |
| `wasmPkgPath` | `string`          | —                   | Path to the `wasm-pack --target nodejs` build of the `lunas_wasm` crate (its `pkg` dir or entry file). Overrides `LUNAS_WASM_PKG` and the in-repo default. |

### Compiler loading

The plugin needs a compiler exposing `compile(source) -> { code, diagnostics }`.

1. If `options.compiler` is given, it is used directly (no wasm is loaded).
2. Otherwise the plugin lazy-loads the `lunas_wasm` wasm-pack (`--target nodejs`)
   build on the first `.lunas` transform, searching in order:
   1. `options.wasmPkgPath`
   2. the `LUNAS_WASM_PKG` environment variable
   3. `crates/lunas_wasm/pkg` (the in-repo dev default)

Build the wasm compiler with:

```sh
cd crates/lunas_wasm
wasm-pack build --target nodejs
```

(A published `@lunas/compiler-wasm`-style package could replace step 2's default
in the future; the option/env override keeps that a drop-in change.)

## Diagnostics

Compiler **errors** abort the build via Rollup's `this.error`, with a
file/line/column and a one-line code frame. **Warnings** and **hints** surface
via `this.warn` and do not stop compilation. Positions are computed from the
compiler's UTF-8 byte offsets.

## HMR

A change to a `.lunas`/`.lun` file triggers a full page reload of the modules
that import it. Finer-grained HMR (patching a live component in place without a
reload) is future work.

## Development

```sh
npm test   # node --test unit suite (mock compiler) + opt-in wasm integration
```

The unit tests inject a mock compiler and need no wasm build. The
`wasm-integration.test.mjs` file builds and runs the real `lunas_wasm` compiler
when `wasm-pack` is available, and skips gracefully otherwise.
