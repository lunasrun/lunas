# Keep-alive

Keep-alive **caches component instances** instead of destroying them when they're
swapped out. When you switch away from a component wrapped in keep-alive, its DOM
is detached but its context, reactive state, and subtree stay alive. When you
switch back, the cached instance is **re-attached with no rebuild** — preserving
scroll position, form input, and every reactive variable.

The natural targets are a dynamic component (`:is`) or a routed
[outlet](../scaling/routing.md): a set of interchangeable panes where rebuilding
from scratch on every switch would be wasteful and would throw away UI state.

## Example

```html
<script>
  let tab = "editor"; // "editor" | "preview" | "settings"
</script>

<nav>
  <button @click="tab = 'editor'">Editor</button>
  <button @click="tab = 'preview'">Preview</button>
  <button @click="tab = 'settings'">Settings</button>
</nav>

<keep-alive>
  <component :is="tab" />
</keep-alive>
```

Type into a field on the **Editor** tab, switch to **Preview**, switch back —
your text is still there, because the Editor instance was *deactivated*, not
destroyed.

## Cache policy — LRU with `max`

Keep-alive is a **keyed LRU cache**. Each distinct key (here, the tab name) maps
to one cached instance. With an optional `max` you cap how many instances are
kept:

```html
<keep-alive max="2">
  <component :is="tab" />
</keep-alive>
```

- Cached instances are ordered by recency of use (most-recently-shown last).
- When the cache overflows `max`, the **least-recently-used** instance is
  **evicted** — and that's the only path (besides tearing the whole keep-alive
  down) that actually destroys an instance and fires its `onDestroy`.
- Deactivation (switching away without overflow) **never** destroys.

Omit `max` for an unbounded cache (every instance ever shown is kept for the
keep-alive's lifetime).

## Lifecycle hooks

Because a kept instance is deactivated rather than destroyed, it gets two extra
lifecycle hooks in addition to the usual `onMount` / `onDestroy`:

```html
<script>
  import { onActivated, onDeactivated } from "lunas";

  onActivated(() => {
    // fires when this instance is (re)attached from the cache
    startPolling();
  });

  onDeactivated(() => {
    // fires when this instance is detached but kept alive
    stopPolling();
  });
</script>
```

The lifecycle mapping:

| Event | Hook fired |
|---|---|
| First mount | `onMount` (+ `onActivated`) |
| Switch back to a cached instance | `onActivated` |
| Switch away (kept in cache) | `onDeactivated` |
| LRU eviction / keep-alive destroy | `onDestroy` |

A fresh instance is considered *activated* on its first show too, so
`onActivated` fires on first mount as well as on every reactivation. Use
`onActivated` / `onDeactivated` to pause and resume work (timers, subscriptions,
media) that shouldn't run while the pane is hidden.

## Node identity is preserved

Reactivation re-inserts the **same DOM nodes** that were detached — the exact
node objects, not clones. That's what preserves imperative DOM state (focus,
scroll offset, `<video>` playback position, uncontrolled input values) that
reactive state alone wouldn't capture.

## How it works

The compiler drives a keep-alive controller from the runtime's
[`keepAlive`](../api/runtime.md):

```js
const ka = keepAlive({ max: 2 });
// show the instance for `key`, mounting fresh or activating a cache hit,
// and deactivating whichever instance was previously shown:
ka.show(c, anchor, key, factory, props);
```

`show(c, anchor, key, factory, props)`:

- deactivates the outgoing instance (detach + `onDeactivated`),
- on a **cache hit**, moves the entry to most-recently-used and re-attaches it
  (`onActivated`, no rebuild),
- on a **miss**, mounts a fresh instance via `mountChild`, then trims the cache
  to `max`.

`ka.has(key)` and `ka.size` are available for tests/introspection; `ka.destroy()`
evicts and destroys every cached instance.

## Gotchas

- **State is preserved across switches — that's the point, but be intentional.**
  If a pane should reset each time it's shown, either don't keep-alive it or
  reset its state in `onActivated`.
- **`max` eviction is silent from the UI's perspective** but fires `onDestroy` on
  the victim. If an evicted instance held a subscription that `onDeactivated`
  paused, make sure `onDestroy` fully cleans it up.
- **Keys must be stable and distinguishing.** Two panes sharing a key share one
  cached instance. With `:is`, the component name is the natural key.
- Keep-alive caches by key within one keep-alive block; it does not share
  instances across different keep-alive blocks.

## See also

- [Suspense](./suspense.md) — pair keep-alive with async panes.
- [Routing](../scaling/routing.md) — keep-alive a router outlet to preserve page
  state across navigation.
- [Transition](./transition.md) — note that kept instances *activate/deactivate*
  rather than enter/leave.
- [Runtime API](../api/runtime.md) — the `keepAlive` controller.
