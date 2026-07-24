# lunas

Dependency-free ES2015 runtime for [Lunas](https://github.com/lunasrun/lunas)-compiled
components. Plain ESM, no build step, no dependencies. Compatibility floor:
ES2015 (no `Proxy`, no `BigInt`).

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
`lunas/boxes`, `lunas/computed`, `lunas/watch`, `lunas/batch`, `lunas/dom`,
`lunas/blocks`, `lunas/for_diff`, `lunas/store`, `lunas/router`.

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
| `afterFlush(c, cb)` | `lunas/core` | Run `cb` once, after the next flush completes (nextTick's primitive). |
| `unbind(c, s)` | `lunas/core` | Permanently unregister a bind record. |
| `beginScope(c)` | `lunas/core` | Open a collection scope for a control-flow block's inner binds. |
| `endScope(c)` | `lunas/core` | Close the currently-open scope. |
| `dropScope(c, scope)` | `lunas/core` | Unregister every bind collected in `scope` (recursively) and tear it down. |
| `box(c, i, v)` | `lunas/boxes` | Reassign-only reactive cell (plain getter/setter, no Proxy). |
| `deepBox(c, i, v)` | `lunas/boxes` | Deeply-mutated reactive cell (raw value; the compiler injects `touch()`/`touchElem()` after a deep mutation to invalidate it — no Proxy). |
| `shared(v)` | `lunas/boxes` | A value shared/mutated across multiple components. |
| `computed(c, i, deps, fn)` | `lunas/computed` | Lazily-evaluated, memoized derived value. |
| `watch(c, deps, cb, opts?)` | `lunas/watch` | Run `cb` when any of `deps` changes (optionally `immediate`). |
| `watchEffect(c, deps, fn)` | `lunas/watch` | Run `fn` immediately and again on any `deps` change. |
| `nextTick(c)` | `lunas/batch` | Promise resolved after the next flush (DOM updated). |
| `batch(c, fn)` | `lunas/batch` | Run `fn`, then flush synchronously, collapsing writes into one update pass. |
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
| `createStore(initial)` | `lunas/store` | Module-level reactive state (named fields) usable by many components. |
| `useStore(c, i, store, key)` | `lunas/store` | Adopt store field `key` at component context `c`'s reactive index `i`. |
| `derivedStore(store, deps, fn)` | `lunas/store` | Lazily-evaluated, memoized value derived from one or more store fields. |
| `createRouter(routes, options)` | `lunas/router` | Client-side router: route table + matching (static > param > catch-all), store-backed reactive current route, navigation guards. |
| `memoryHistory(initial)` | `lunas/router` | In-memory History-API stand-in for tests/SSR (injectable via `options.history`). |
| `historyAdapter(win)` | `lunas/router` | Default History-API adapter (pushState/replaceState/popstate). |
| `routerOutlet(c, anchor, router, opts)` | `lunas/router` | Mount the matched route's component at an anchor, swapping on navigation; params passed as props. |
| `routerLink(el, router, path, opts)` | `lunas/router` | Wire an element's click to a client-side navigation (preventDefault + push). |
| `asyncComponent(loader, opts?)` | `lunas/async` | Wrap a lazy module loader (`() => import(...)`) into a mountable child factory; resolves default export or bare factory, caches after first load, optional `loading`/`error`/`delay`/`timeout`. |
| `mountAsyncChild(c, anchor, factory, props?)` | `lunas/async` | Mount an async component at an anchor (mountChild contract + suspense registration); `unmount()` cancels any in-flight load. |
| `suspenseBlock(c, anchor, contentFactory, fallbackFactory?)` | `lunas/async` | Async boundary at an anchor: shows `fallback` while any async child under it is pending, reveals `content` once all resolve (batched, no flash); nested boundaries are independent. |
| `onMount(c, fn)` | `lunas/lifecycle` | Run `fn` after the component's root attaches to a live tree (fires once). |
| `onDestroy(c, fn)` | `lunas/lifecycle` | Run `fn` when the component is torn down (fires once, every unmount path). |
| `onUpdate(c, fn)` | `lunas/lifecycle` | Run `fn` after each flush of `c` that ran updates. |
| `onActivated(c, fn)` / `onDeactivated(c, fn)` | `lunas/lifecycle` | Keep-alive (de)activation hooks — fire on cache re-attach / detach. |
| `attach(root, host)` | `lunas/lifecycle` | Append a detached component root to a live host and fire the subtree's `onMount` callbacks. |
| `isLive(node)` | `lunas/lifecycle` | Whether `node` is in a live tree (`isConnected` + shim fallback). |
| `emit(c, name, payload?)` | `lunas/emits` | Raise a child→parent event; invokes the parent's `on<Name>` handler prop. Never marks the parent dirty by itself. |
| `registerEmits(c, props, declared?)` | `lunas/emits` | Stash a child's props (so `emit` finds handlers); optional declared-events validation. |
| `eventPropName(name)` | `lunas/emits` | Map an event name to its prop (`save` → `onSave`); the codegen's `@name`→`onName` mapping. |
| `provide(c, key, value)` | `lunas/provide` | Provide a value (string or Symbol key) to descendants via the parent-context chain. |
| `inject(c, key, default?)` | `lunas/provide` | Resolve a provided value from the nearest ancestor, else the default. |
| `hasInjection(c, key)` | `lunas/provide` | Whether any ancestor provides `key` (distinguishes provided-`undefined` from absent). |
| `withTransition(opts?)` | `lunas/transition` | Build an enter/leave CSS-class transition controller composing with a block's insert/remove; degrades to immediate outside a browser. |
| `runPhase(el, base, phase, opts, done)` | `lunas/transition` | Run one enter/leave class choreography on an element (frame classes + `transitionend`/timeout). |
| `keepAlive(opts?)` | `lunas/keepalive` | Cache mountChild instances by key: deactivate detaches (keeps state), activate re-attaches (no rebuild); LRU `max`; eviction fires `onDestroy`. |

## Testing

```sh
npm test                    # runs every test/*.test.mjs via test/run-all.mjs
node test/treeshake.check.mjs   # tree-shaking / no-side-effects proof
```

Requires Node.js >= 18.

## License

MIT
