# lunas-app

A minimal [Lunas](https://github.com/lunasrun/lunas) + [Vite](https://vitejs.dev)
starter, scaffolded with `create-lunas`.

## Getting started

```sh
npm install
npm run dev
```

Then open the printed local URL. Edit `src/App.lunas` and the page updates.

## Scripts

- `npm run dev` — start the Vite dev server (HMR: a `.lunas` change reloads).
- `npm run build` — production build into `dist/`.
- `npm run preview` — preview the production build locally.

## Project layout

- `index.html` — the app shell; mounts into `#app`.
- `src/main.mjs` — imports the compiled `App.lunas` and `attach`es it.
- `src/App.lunas` — the root component: reactive state, an event handler, and
  `:if` / `:for` in the template.
- `vite.config.mjs` — wires up `vite-plugin-lunas`.

## Notes

The template pins `lunas` and `vite-plugin-lunas` with normal semver ranges. If
you are developing against a local checkout of the Lunas monorepo instead of the
published packages, point these at the local builds, e.g.:

```jsonc
{
  "dependencies": {
    // "lunas": "file:../lunas/packages/lunas"
  },
  "devDependencies": {
    // "vite-plugin-lunas": "file:../lunas/packages/vite-plugin-lunas"
  }
}
```

The plugin loads the compiler from the `lunas_wasm` wasm-pack build. In the
monorepo it defaults to `crates/lunas_wasm/pkg`; override with the plugin's
`wasmPkgPath` option or the `LUNAS_WASM_PKG` environment variable. See the
`vite-plugin-lunas` README for details.
