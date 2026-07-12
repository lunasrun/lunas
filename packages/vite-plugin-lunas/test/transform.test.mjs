// transform.test.mjs — unit tests for the plugin's transform hook with an
// injected mock compiler (no wasm, no Vite server needed).
// Run: node packages/vite-plugin-lunas/test/transform.test.mjs

import { test } from "node:test";
import assert from "node:assert";

import lunas from "../src/index.mjs";

// A minimal Rollup/Vite plugin-context stand-in. `this.error` throws (as Rollup
// does); `this.warn` records the warning.
function makeCtx() {
  const warnings = [];
  return {
    warnings,
    warn(w) {
      warnings.push(w);
    },
    error(e) {
      const err = new Error(typeof e === "string" ? e : e.message);
      err.payload = e;
      throw err;
    },
  };
}

// Run the plugin's transform hook with a given plugin instance + context.
function runTransform(plugin, ctx, code, id) {
  return plugin.transform.call(ctx, code, id);
}

// A mock compiler that returns fixed output / diagnostics.
function mockCompiler({ code = null, diagnostics = [] } = {}) {
  const calls = [];
  return {
    calls,
    compile(source) {
      calls.push(source);
      return { code, diagnostics };
    },
  };
}

test("ignores non-.lunas files (returns null, compiler not called)", () => {
  const compiler = mockCompiler({ code: "export default 1;" });
  const plugin = lunas({ compiler });
  const ctx = makeCtx();
  assert.strictEqual(runTransform(plugin, ctx, "whatever", "/a/foo.js"), null);
  assert.strictEqual(runTransform(plugin, ctx, "whatever", "/a/foo.ts"), null);
  assert.strictEqual(compiler.calls.length, 0);
});

test("compiles .lunas and passes the emitted code through", () => {
  const emitted = 'import { component } from "lunas";\nexport default 42;';
  const compiler = mockCompiler({ code: emitted });
  const plugin = lunas({ compiler });
  const ctx = makeCtx();
  const out = runTransform(plugin, ctx, "html:\n  <p>hi</p>", "/src/App.lunas");
  assert.ok(out);
  assert.strictEqual(out.code, emitted);
  assert.strictEqual(out.map, null);
  assert.strictEqual(compiler.calls.length, 1);
});

test("compiles .lun files too", () => {
  const compiler = mockCompiler({ code: "export default 1;" });
  const plugin = lunas({ compiler });
  const out = runTransform(plugin, makeCtx(), "x", "/src/Widget.lun");
  assert.ok(out);
  assert.strictEqual(out.code, "export default 1;");
});

test("strips query suffixes when matching ids", () => {
  const compiler = mockCompiler({ code: "export default 1;" });
  const plugin = lunas({ compiler });
  const out = runTransform(
    plugin,
    makeCtx(),
    "x",
    "/src/App.lunas?vue&type=script"
  );
  assert.ok(out, "an id with a query suffix should still be handled");
});

test("custom extensions option is honored", () => {
  const compiler = mockCompiler({ code: "export default 1;" });
  const plugin = lunas({ compiler, extensions: [".lx"] });
  const ctx = makeCtx();
  assert.strictEqual(runTransform(plugin, ctx, "x", "/src/App.lunas"), null);
  assert.ok(runTransform(plugin, ctx, "x", "/src/App.lx"));
});

test("error diagnostics abort the transform via this.error with position", () => {
  const compiler = mockCompiler({
    code: null,
    diagnostics: [
      { message: "unexpected token", severity: "error", start: 6, end: 7 },
    ],
  });
  const plugin = lunas({ compiler });
  const ctx = makeCtx();
  // source: "html:\nX" -> byte 6 is line 2, column 1
  assert.throws(
    () => runTransform(plugin, ctx, "html:\nX", "/src/App.lunas"),
    (err) => {
      assert.strictEqual(err.message, "unexpected token");
      assert.strictEqual(err.payload.loc.line, 2);
      assert.strictEqual(err.payload.loc.column, 1);
      assert.ok(err.payload.frame.includes("^"));
      return true;
    }
  );
});

test("warning diagnostics surface via this.warn but still emit code", () => {
  const compiler = mockCompiler({
    code: "export default 1;",
    diagnostics: [
      { message: "unsupported construct voided", severity: "warning", start: 0, end: 1 },
    ],
  });
  const plugin = lunas({ compiler });
  const ctx = makeCtx();
  const out = runTransform(plugin, ctx, "html:\n  <p></p>", "/src/App.lunas");
  assert.ok(out);
  assert.strictEqual(out.code, "export default 1;");
  assert.strictEqual(ctx.warnings.length, 1);
  assert.strictEqual(ctx.warnings[0].message, "unsupported construct voided");
});

test("null code with no error diagnostics still errors (nothing emitted)", () => {
  const compiler = mockCompiler({ code: null, diagnostics: [] });
  const plugin = lunas({ compiler });
  assert.throws(
    () => runTransform(plugin, makeCtx(), "html:\n  <p></p>", "/src/App.lunas"),
    /no output produced/
  );
});

test("compiler load/throw failures surface as this.error", () => {
  const plugin = lunas({
    compiler: {
      compile() {
        throw new Error("boom loading wasm");
      },
    },
  });
  assert.throws(
    () => runTransform(plugin, makeCtx(), "html:\n  <p></p>", "/src/App.lunas"),
    /boom loading wasm/
  );
});

test("rejects an injected compiler without a compile method", () => {
  assert.throws(() => lunas({ compiler: {} }), /must have a `compile/);
});

test("plugin metadata: name and enforce", () => {
  const plugin = lunas({ compiler: mockCompiler() });
  assert.strictEqual(plugin.name, "vite-plugin-lunas");
  assert.strictEqual(plugin.enforce, "pre");
});
