# Conditional rendering

`:if`, `:elseif`, and `:else` conditionally render an element (and its subtree).
They are **attributes on the element itself**, not wrapper tags — the element
they sit on is the conditional's body.

## Basic `:if`

```lunas
html:
    <div>
        <button @click="toggle()">toggle</button>
        <span :if="show">Now you see me</span>
    </div>

script:
    let show = true
    function toggle() { show = !show }
```

When `show` is truthy the `<span>` is built and inserted; when falsy its nodes
are removed. A permanent text **anchor** marks the slot position, so the location
is always known even while the branch is absent.

## Cascades: `:elseif` / `:else`

`:elseif` and `:else` chain onto a preceding sibling `:if` to form one cascade.
Exactly one branch is shown at a time:

```lunas
html:
    <div>
        <button @click="bump()">step</button>
        <p :if="n == 0">zero</p>
        <p :elseif="n > 3">big ${n}</p>
        <p :else>small ${n}</p>
    </div>

script:
    let n = 0
    function bump() { n = n + 1 }
```

- `:if` and `:elseif` carry a condition expression.
- `:else` carries **no value** and matches when every preceding condition is
  false.
- Branches must be adjacent siblings (whitespace between them is fine). An
  `:elseif` / `:else` with no matching `:if` is an error.
- A cascade **without** an `:else` shows nothing when all conditions are false.

## Behavior: build-on-show

A branch's DOM is built only when it becomes visible:

- On show, the branch is constructed from its own static skeleton (one
  `innerHTML` parse for that branch) and inserted before the anchor.
- On hide, its nodes are removed and any bindings inside it are torn down, so a
  hidden branch receives no updates and leaks nothing.
- Switching branches in a cascade tears down the old branch and builds the new
  one.

This means an expensive subtree behind an `:if="false"` costs nothing until it is
first shown.

### How it compiles

A lone `:if` compiles to an `ifBlock`; a full cascade compiles to a single
`ifChain` with a `which()` selector that maps the conditions to a branch index
(`-1` when nothing should show):

```js
// <p :if="n > 0">…</p><p :elseif="n < 0">…</p><p :else>…</p>
ifChain(c, a0, [0], () => (n.v > 0) ? 0 : (n.v < 0) ? 1 : 2, [ /* branch builders */ ]);
```

The whole cascade shares one anchor and one dependency set, so a change
re-evaluates the selector once and swaps branches only if the winning index
changed.

## Nesting

Conditionals nest by element nesting — an `:if` element can contain other `:if`
elements, `:for` lists, or components, and each inner block builds and tears down
with its parent branch:

```lunas
html:
    <ul>
        <li :for="group of groups" :key="group.id">
            <b>${group.name}</b>
            <em :if="group.open">(open)</em>
        </li>
    </ul>
```

Here the `:if` lives inside a `:for` item; showing/hiding it re-evaluates against
the current item's data, and removing the item tears the inner `:if` down with
it. See [list rendering](./list-rendering.md) for `:for` + `:if` interaction.

## Notes

- `:if` toggles **presence in the DOM**. To toggle visibility while keeping the
  node (and its state) mounted, bind a class or style instead — see
  [class and style](./class-and-style.md).
- Because a hidden branch's bindings are removed, its component children unmount
  and their [`onDestroy`](./lifecycle.md) fires; re-showing rebuilds fresh.

## Related

- [List rendering](./list-rendering.md) — `:for`, and `:for` + `:if`.
- [Template syntax](./template-syntax.md).
- Keeping hidden components alive across toggles: keep-alive in [built-ins](../built-ins/).
