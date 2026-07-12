// vite-plugin-lunas — compile .lunas / .lun single-file components with Vite.
//
// The plugin runs each .lunas/.lun module through the Lunas compiler (via the
// wasm bindings, or an injected compiler) and hands Vite the emitted ES module,
// which imports its runtime from the `lunas` package.
//
// Options:
//   extensions  string[]  file extensions to handle. Default [".lunas", ".lun"].
//   compiler    object     inject `{ compile(source) -> { code, diagnostics } }`
//                          (tests / custom builds). Skips wasm loading entirely.
//   wasmPkgPath string     path to the `wasm-pack --target nodejs` output dir of
//                          the lunas_wasm crate (or its entry file). Overrides
//                          the LUNAS_WASM_PKG env var and the in-repo default.
//
// Diagnostics: compiler errors abort the transform via `this.error` (with a
// file/line/column + code frame); warnings/hints surface via `this.warn`.
//
// HMR: a change to a .lunas/.lun file triggers a full reload of the modules
// that import it (see `handleHotUpdate`). Finer-grained HMR (patching a live
// component without a reload) is future work.

import { makeCompilerLoader } from "./compiler.mjs";
import { toRollupError } from "./diagnostics.mjs";

const DEFAULT_EXTENSIONS = [".lunas", ".lun"];

// Strip a Vite query suffix (`?import`, `?t=...`) before extension matching.
function cleanId(id) {
  const q = id.indexOf("?");
  return q === -1 ? id : id.slice(0, q);
}

function makeMatcher(extensions) {
  const exts = extensions && extensions.length ? extensions : DEFAULT_EXTENSIONS;
  return (id) => {
    const path = cleanId(id);
    return exts.some((ext) => path.endsWith(ext));
  };
}

export default function lunas(options = {}) {
  const matches = makeMatcher(options.extensions);
  const getCompiler = makeCompilerLoader(options);

  return {
    name: "vite-plugin-lunas",
    enforce: "pre",

    transform(code, id) {
      if (!matches(id)) return null;

      const path = cleanId(id);
      let result;
      try {
        result = getCompiler().compile(code);
      } catch (err) {
        // Loading/invoking the compiler itself failed (e.g. wasm build absent).
        this.error({
          message: err && err.message ? err.message : String(err),
          id: path,
        });
        return null;
      }

      const diagnostics = (result && result.diagnostics) || [];
      const errors = diagnostics.filter((d) => d.severity === "error");
      const nonErrors = diagnostics.filter((d) => d.severity !== "error");

      // Surface warnings/hints (non-fatal).
      for (const d of nonErrors) {
        this.warn(toRollupError(d, path, code));
      }

      if (errors.length > 0 || result == null || result.code == null) {
        // Prefer a real diagnostic; otherwise a generic failure.
        const first = errors[0];
        if (first) {
          this.error(toRollupError(first, path, code));
        } else {
          this.error({
            message: `[vite-plugin-lunas] failed to compile ${path} (no output produced).`,
            id: path,
          });
        }
        return null;
      }

      return {
        code: result.code,
        // No source map yet: the emitted module is generated, not a
        // line-preserving transform of the input. Returning null keeps Vite
        // from fabricating a misleading map.
        map: null,
      };
    },

    handleHotUpdate(ctx) {
      if (!matches(ctx.file)) return;
      // Minimal HMR: full-reload the modules that import this component. Vite
      // reloads the importers when we return the affected module set.
      const mods = ctx.modules;
      if (mods && mods.length) {
        ctx.server.ws.send({ type: "full-reload" });
        return mods;
      }
    },
  };
}

export { makeCompilerLoader, toRollupError };
