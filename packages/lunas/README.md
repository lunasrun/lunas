# lunas

Dependency-free ES2015 runtime for [Lunas](https://github.com/lunasrun/lunas)-compiled
components. Plain ESM, no build step, no dependencies. Compatibility floor:
ES2015 + `Proxy` (no `BigInt`).

This package is the runtime target that the Lunas compiler emits calls into —
`bind`/`markVar`/`flush` for the reactive core, `box`/`deepBox`/`shared` for
reactive state, DOM/event wiring helpers, and the control-flow blocks
(`if`/`for`/child components) with a keyed LIS reconciler for `:for`.

## Install

```sh
npm install lunas
```

## Usage

The package is pure ESM (`"type": "module"`) and marks `"sideEffects": false`,
so bundlers can tree-shake unused exports. Import only what you need from the
package root, or deep-import a single module to keep your bundle minimal:

```js
import { box, bind, markVar } from "lunas";
// or, equivalently, a deep import of just the module you need:
import { box } from "lunas/boxes";
```

Available deep-import subpaths mirror the internal modules: `lunas/core`,
`lunas/boxes`, `lunas/dom`, `lunas/blocks`, `lunas/for_diff`.

TypeScript types are included (`types/index.d.ts`, plus one `.d.ts` per
subpath) — no `@types` package needed.

```js
import { createContext, bind, markVar, box } from "lunas";

const c = createContext(document.body);
const count = box(c, 0, 0);

bind(c, [0], () => {
  document.title = "count: " + count.v;
});

count.v++; // marks index 0 dirty; the bind above re-runs on next microtask
markVar(c, 0);
```

In practice this package is not meant to be hand-written against — the Lunas
compiler generates the calls into it from `.lun` component files. See
`crates/lunas_compiler/docs/output-design.md` and `for-diff-design.md` in the
main repo for the calling contract.

## API

| Export | From | Description |
| --- | --- | --- |
| `createContext(root)` | `lunas/core` | Create a fresh reactive context rooted at `root`. |
| `bind(c, deps, fn)` | `lunas/core` | Register an update fn for reactive indices `deps`; runs once immediately. |
| `markVar(c, i)` | `lunas/core` | Mark reactive variable `i` dirty; schedules a microtask flush. |
| `flush(c)` | `lunas/core` | Run every queued update once. |
| `unbind(c, s)` | `lunas/core` | Permanently unregister a bind record. |
| `beginScope(c)` | `lunas/core` | Open a collection scope for a control-flow block's inner binds. |
| `endScope(c)` | `lunas/core` | Close the currently-open scope. |
| `dropScope(c, scope)` | `lunas/core` | Unregister every bind collected in `scope` (recursively) and tear it down. |
| `box(c, i, v)` | `lunas/boxes` | Reassign-only reactive cell (plain getter/setter, no Proxy). |
| `deepBox(c, i, v)` | `lunas/boxes` | Deeply-mutated reactive cell (Proxy-wrapped nested reads/writes). |
| `shared(v)` | `lunas/boxes` | A value shared/mutated across multiple components. |
| `component(tag, attrs, HTML, setup)` | `lunas/dom` | Compiled-component factory: builds the root, parses static HTML, runs `setup`. |
| `refs(root, paths)` | `lunas/dom` | Positional navigation to dynamic elements by child-index paths. |
| `on(el, ev, fn)` | `lunas/dom` | `addEventListener` shorthand. |
| `anchorBefore(node)` | `lunas/dom` | Create a permanent empty-text-node anchor before `node`. |
| `anchorBeforeSplit(textNode, offset)` | `lunas/dom` | Split a text node and place an anchor between head/tail. |
| `anchorAppend(parent)` | `lunas/dom` | Create an anchor as the last child of `parent`. |
| `ifBlock(c, anchor, deps, cond, make)` | `lunas/blocks` | Conditional block: mounts/unmounts a branch at `anchor`. |
| `forBlock(c, anchor, deps, items, opts)` | `lunas/blocks` | Keyed list block; reconciles via `for_diff`'s LIS algorithm. |
| `mountChild(c, anchor, childFactory, props)` | `lunas/blocks` | Instantiate and mount a child component at `anchor`. |
| `createForState()` | `lunas/for_diff` | Create empty reconciler state. |
| `seedForState(state, keys, nodes, data)` | `lunas/for_diff` | Seed reconciler state from a bulk initial render. |
| `reconcile(state, host, items, makeItem, opts)` | `lunas/for_diff` | Diff old vs. new keyed list and mutate `host` with minimal moves. |
| `longestIncreasingSubsequence(arr)` | `lunas/for_diff` | LIS helper used internally by `reconcile`. |

## Testing

```sh
npm test                    # runs every test/*.test.mjs via test/run-all.mjs
node test/treeshake.check.mjs   # tree-shaking / no-side-effects proof
```

Requires Node.js >= 18.

## License

MIT
