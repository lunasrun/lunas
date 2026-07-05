# Sample-directory E2E test harness

This is the Svelte-style sample runner for Lunas: **one directory = one test
case**. It is the foundation the mass-production test suite (1000+ cases) is
built on. Adding a case is just creating a directory with a `.lunas` component
and (optionally) an expected-HTML file and an interaction script — no Rust.

- Cases live under `samples/<category>/<case>/`.
- The Rust runner is `crates/lunas_compiler/tests/runtime_samples.rs`.
- The node driver + assertion kit live under `harness/`.

## TL;DR — add a case in 30 seconds

```
samples/text/interpolation/
├── App.lunas        # the component to mount (required)
├── _config.json     # optional: props / diagnostics / skip / description
├── expected.html    # normalized initial DOM (generate with UPDATE_EXPECTED=1)
├── steps.mjs        # optional: interactions + assertions
└── expected.after.html   # optional: DOM after steps (auto-generated)
```

1. Write `App.lunas`.
2. Run `UPDATE_EXPECTED=1 cargo test -p lunas_compiler --test runtime_samples`
   to generate `expected.html` (and `expected.after.html` if you added
   `steps.mjs`).
3. Review the generated HTML, commit. Done — it now has its own `#[test]`.

## Case format

### `App.lunas` (required)

The entry component. It is compiled and mounted; its rendered DOM is the initial
assertion target. It may `@use` sibling `*.lunas` files in the **same case dir**
for multi-component cases:

```
@use Child from "./Child.lunas"
html:
    <div><Child :count="n"/></div>
script:
    let n = 1
```

Put `Child.lunas` next to `App.lunas`. Every `*.lunas` in the case dir is
compiled; `@use "./Foo.lunas"` imports are rewired to the compiled sibling
automatically. There is no limit on the number of siblings.

### `_config.json` (optional)

All fields optional. Absent file = all defaults.

| field         | type     | default  | meaning                                                       |
| ------------- | -------- | -------- | ------------------------------------------------------------- |
| `props`       | object   | `{}`     | props passed to the `App` factory at mount                    |
| `diagnostics` | string   | `"none"` | `"none"` = compile must emit **no** warnings; `"expected"` = warnings allowed |
| `skip`        | string   | —        | if present, the case is skipped with this reason (loud eprintln, not a failure) |
| `description` | string   | —        | human note; ignored by the runner                            |

Compile **errors** always fail the case regardless of `diagnostics`.

```json
{ "props": { "start": 2 }, "description": "click increments a counter" }
```

### `expected.html` (regeneratable)

The normalized initial DOM after mount (see **Normalization** below). A single
trailing newline is ignored on compare. Regenerate with `UPDATE_EXPECTED=1`.

If `expected.html` is absent and you are **not** in update mode, the case fails
with a hint to run `UPDATE_EXPECTED=1`.

### `steps.mjs` (optional)

Drives interactions and asserts after the initial render. Default-export an
async function; the harness calls it with the assertion kit:

```js
export default async ({ root, roots, mount, tick, $, $$, click, dispatch,
                        setValue, expect, equal }) => {
  expect("button").text("count: 2");
  await click("button");
  expect("button").text("count: 3");
};
```

Steps run **after** the initial-DOM check passes. `steps.mjs` needs **no
imports** — everything is injected via the argument object.

### `expected.after.html` (optional, auto-generated)

If a case has `steps.mjs`, `UPDATE_EXPECTED=1` also writes the normalized DOM
*after* all steps run to `expected.after.html`. On a normal run, if that file
exists, the post-steps DOM is compared to it — a belt-and-suspenders snapshot on
top of the explicit `expect(...)` assertions in `steps.mjs`. Delete the file if
you don't want the extra snapshot; the `expect(...)` calls still run.

## Assertion kit API

The object passed to `steps.mjs`:

| helper                    | description                                                                 |
| ------------------------- | --------------------------------------------------------------------------- |
| `root`                    | the first mounted root node (the component wrapper `<div>`, or first fragment node) |
| `roots`                   | array of all mounted root nodes (single-root cases have one)                |
| `$ (sel)`                 | first element matching a selector (throws if none)                          |
| `$$ (sel)`                | array of all matching elements                                              |
| `click(selOrNode)`        | dispatch a `click` and `await tick()`                                       |
| `dispatch(selOrNode, ev, detail?)` | dispatch an arbitrary event and `await tick()`                    |
| `setValue(selOrNode, v)`  | set `el.value = v`, dispatch `input`, `await tick()` (drives two-way binds) |
| `tick()`                  | await one runtime flush (`setTimeout(0)`)                                    |
| `expect(selOrNode)`       | element assertions (below)                                                  |
| `equal(actual, expected)` | strict `===` value assertion (use for plain strings/numbers built in a step)|
| `mount(factory, props?)`  | mount an extra factory (advanced; rarely needed)                            |

### Selectors

A narrow CSS subset, matched depth-first from the mounted roots:
`tag`, `.class`, `#id`, and the combo `tag.class`. No descendant/child
combinators. Note the **component is wrapped in an outer `<div>`**, so `$("div")`
matches that wrapper first — target inner elements by a class or a distinct tag.

### `expect(selOrNode)` assertions (chainable)

| assertion                | passes when                                                          |
| ------------------------ | ------------------------------------------------------------------- |
| `.text(str)`             | the element's normalized innerHTML equals `str`                     |
| `.html(str)`             | same as `.text` (alias; reads better for markup)                    |
| `.attr(name, str)`       | `getAttribute(name) === str` (use for `class`, `style`, `:href`, …) |
| `.value(str)`            | the IDL `el.value === str` (use for `:value` / `::value` binds)     |
| `.prop(name, val)`       | the IDL property `el[name] === val` (`checked`, `disabled`, …)      |
| `.hasClass(cls)`         | the element's class list contains `cls`                             |
| `.count(n)`              | exactly `n` elements matched the selector                           |

> Attribute vs property: `:value="x"` sets the **property** (`el.value`), not the
> `value` attribute — assert it with `.value(...)`. `:class` / `:style` are
> applied as **attributes** — assert with `.attr("class", ...)` /
> `.attr("style", ...)`.

For raw value comparisons that are not about a DOM node (e.g. a joined label
string), use `equal(actual, expected)` — `expect(aString)` is always treated as
a selector.

## Normalization rules

Expected HTML is a **stable, diff-friendly** serialization of the shim DOM
(`harness/normalize.mjs`):

1. Elements serialize as `<tag attr="v" …>children</tag>`; void elements
   (`input`, `br`, `img`, …) have no closing tag and no children.
2. **Attributes are sorted alphabetically by name** — output never depends on
   insertion order.
3. `class` values are whitespace-collapsed (runs of spaces → one, trimmed) so
   `:class` merges are stable.
4. Adjacent text nodes are concatenated — the compiler splits interpolation
   anchors into multiple text nodes, and that split is invisible here.
5. Attribute values are HTML-escaped for `"` and `&` only (what the shim
   round-trips).
6. There is no inter-element whitespace to collapse: the compiler emits a
   whitespace-free skeleton. Text **inside** an element is preserved verbatim
   (it is significant).

`expected.html` stores exactly this serialization plus a trailing newline.

## Regenerating expected output — `UPDATE_EXPECTED=1`

```
UPDATE_EXPECTED=1 cargo test -p lunas_compiler --test runtime_samples
```

Regenerates `expected.html` (and `expected.after.html` for cases with
`steps.mjs`) for **every** case, then passes. Review the `git diff` before
committing. In update mode a stored-HTML mismatch is silently rewritten, but a
real **error** (a thrown exception, a broken module, a failing `expect(...)` in
`steps.mjs`) still fails loudly — update mode never masks bugs.

Run a single case's regen by filtering:

```
UPDATE_EXPECTED=1 cargo test -p lunas_compiler --test runtime_samples -- case_text__interpolation
```

(The test name is the case path with `/` → `__`.)

## How it runs (design)

- **Per-case granularity without a proc-macro.** `build.rs` scans `samples/` at
  build time and generates one `#[test] fn case_<name>()` per case dir. A failing
  case names itself; cases schedule in parallel under `cargo test`; a broken case
  fails only its own test.
- **A single batched node process.** The first generated test to run triggers the
  whole batch once (guarded by a `OnceLock`): all cases are compiled in Rust,
  their emitted modules + a JSON manifest are written into one temp dir, and
  `node harness/run-samples.mjs` is spawned **once** for the entire suite. Each
  per-case test then just looks up its own result. Node spawns stay **O(1)**
  regardless of case count — this is what keeps 1000+ cases fast.
- **Real runtime, real dom-shim.** Cases run against the actual runtime in
  `packages/lunas/src` under the dependency-free `packages/lunas/test/dom-shim.mjs`
  — the same environment the other exec suites use.

## Node absent → skip loudly

If node is not at the pinned path
(`~/.nvm/versions/node/v22.18.0/bin/node`), the whole suite skips with an
`eprintln`, exactly like the other exec suites — CI without node still passes.
CI's `runtime` job provides node 22, so the suite actually runs there.
