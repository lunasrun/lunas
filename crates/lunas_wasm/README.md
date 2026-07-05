# lunas_wasm

Thin [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) bindings over
the Lunas compiler. It exposes one function — `compile(source)` — that turns a
`.lunas`/`.lun` source string into a compiled ES module plus diagnostics, from
JavaScript.

This crate is **bindings only**: all logic lives in `lunas_compiler`. It exists
so a bundler plugin (`packages/vite-plugin-lunas`) or a browser playground can
run the real compiler without a native toolchain.

## API

```js
import { compile, version } from "lunas_wasm";

const { code, diagnostics } = compile(source);
// code:        string | null   — the emitted ES module (null on failure)
// diagnostics: Array<{ message, severity, start, end }>
//   severity:  "error" | "warning" | "hint"
//   start/end: UTF-8 byte offsets into `source`
```

`compile` never throws for compiler problems — they come back as
`diagnostics`. `code` is `null` when there is an error (or nothing to emit).

## Building

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and the
`wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).

Build from **this crate's directory** (`crates/lunas_wasm`):

```sh
# Node target (what the Vite plugin loads by default) → ./pkg
wasm-pack build --target nodejs

# Browser/bundler target (for a playground or the `web` build) → ./pkg-web
wasm-pack build --target web --out-dir pkg-web
```

Each build produces a JS wrapper + `lunas_wasm_bg.wasm` in the out dir. The
Vite plugin looks for the `nodejs` build at `crates/lunas_wasm/pkg` by default;
override the location with the plugin's `wasmPkgPath` option or the
`LUNAS_WASM_PKG` environment variable (see `packages/vite-plugin-lunas`).

`pkg/` and `pkg-web/` are build artifacts and are git-ignored.

## Notes

- The `rlib` crate type is kept alongside `cdylib` so the host
  `cargo test --workspace` can exercise the pure-Rust adapter logic (diagnostic
  flattening, error passthrough) without a wasm runtime.
- Spans are UTF-8 byte offsets, matching the compiler. A JS consumer that needs
  line/column (e.g. for editor squiggles) builds its own line index over the
  original source.
