# Transitions

Lunas animates the **enter** and **leave** of control-flow content (an `:if`
branch, a `:for` item, a mounted child) by toggling a well-known sequence of CSS
classes around the insert/remove. Your CSS drives the actual animation; the
runtime just adds and removes classes and waits for `transitionend`.

This is the Vue-style transition model: name a transition `n`, write CSS for the
`n-enter-*` / `n-leave-*` classes, and Lunas choreographs them.

## The class sequence

For a transition named `n`, an element gets, on **enter**:

| Phase | Classes |
|---|---|
| frame 0 (before paint) | `+ n-enter-from`  `+ n-enter-active` |
| frame 1 (next frame) | `− n-enter-from`  `+ n-enter-to` |
| transitionend / timeout | `− n-enter-active`  `− n-enter-to` |

and symmetrically on **leave** (`n-leave-from`, `n-leave-active`,
`n-leave-to`) — with the node **removed only after the leave transition
finishes**.

So a full fade looks like:

```html
<script>
  let show = true;
</script>

<button @click="show = !show">Toggle</button>

<div :if="show" class="box" c-transition="fade">
  Hello
</div>

<style>
  .fade-enter-active,
  .fade-leave-active {
    transition: opacity 200ms ease;
  }
  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }
  /* fade-enter-to and fade-leave-from are the resting state (opacity: 1),
     so they need no rule here. */
</style>
```

- `fade-enter-from` starts the element invisible; frame 1 swaps to
  `fade-enter-to` (the resting state), and the `fade-enter-active` transition
  animates the change.
- On leave the element is held in the DOM while `fade-leave-active` animates it
  back to `fade-leave-to` (invisible), then removed.

## Naming

The transition **base name** defaults to `v` if you don't provide one — so
`v-enter-from`, `v-leave-active`, etc. Give it a name (`fade` above) to run
several distinct transitions in one app.

## When enter/leave completes

The runtime finishes a phase on the element's `transitionend` event. Because
some elements never fire `transitionend` (`display: none`, a zero-duration
transition, or an interrupted one), a **timeout fallback** is always armed so
teardown can never hang. You can set that fallback explicitly:

- The `duration` option is the fallback timeout in milliseconds (`0` → next
  macrotask). It is a *safety net*, not the animation length — the animation
  length still comes from your CSS `transition` property.

`transitionend` events that bubble up from **child** elements are ignored — only
the transitioning element's own `transitionend` counts. This keeps a transition
on a container from finishing early because an inner element's transition fired.

## Degradation outside the browser

There is no CSS engine outside a browser (and the test DOM has none), so when
`requestAnimationFrame` is unavailable the transition **degrades to an immediate
insert/remove**. Importantly, the class *sequence still runs synchronously* — the
classes are added and removed in order and completion fires right away — so the
logic stays observable and testable. In a real browser the frames and
`transitionend` (with the `duration` timeout fallback) drive real timing.

This means:

- SSR / non-DOM environments never hang waiting for an animation.
- Transitions are a pure enhancement: if the environment can't animate, content
  still appears and disappears correctly.

## How it works

The compiler wraps a block's own insert/remove closures with the runtime's
[`withTransition`](../api/runtime.md):

```js
const t = withTransition({ name: "fade", duration: 200 });
// enter: insert the nodes, then choreograph enter classes on each root
t.enter(nodes, insert);
// leave: choreograph leave classes, then remove() once every node's leave finishes
t.leave(nodes, remove);
```

`withTransition` returns `{ enter, leave }` that compose with the block's own
mount/unmount. `runPhase(el, base, phase, opts, done)` is the lower-level
primitive that runs one enter/leave choreography on a single element — useful if
you're wiring transitions by hand.

## Gotchas

- **Leave is asynchronous.** The node lingers in the DOM through the whole leave
  animation, then is removed. Don't assume it's gone the instant `:if` flips
  false.
- **Multi-root content:** every root node gets the class sequence; a leave
  completes only when *all* roots have finished, then all are removed together.
- **`duration` is a fallback, not the source of truth.** Your CSS `transition`
  duration is what animates. Set `duration` to something ≥ your CSS duration so
  the fallback never cuts an animation short.
- **The `-active` class is where you put the `transition` property.** The
  `-from`/`-to` classes set start/end states; `-active` defines *how* to move
  between them.
- Transitions attach to control-flow blocks (`:if`, `:for`, mounted children),
  not to arbitrary always-present elements — there's nothing to enter or leave
  if the element is never inserted or removed.

## See also

- [Control flow](../guide/control-flow.md) — `:if` / `:for` blocks that
  transitions wrap.
- [Teleport](./teleport.md) — animate a teleported modal.
- [Keep-alive](./keep-alive.md) — cached instances that activate/deactivate
  instead of enter/leave.
- [Runtime API](../api/runtime.md) — `withTransition` / `runPhase`.
