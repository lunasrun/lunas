# Registration — using one component inside another

A Lunas component becomes a child of another component by being **declared with
`@use`** and then **referenced by tag** in the template. There is no global
registry and no runtime lookup: `@use` is a compile-time import statement, so a
tag only resolves to a component if that component was `@use`d in the same file.

## Declaring a child with `@use`

`@use` lines sit at the very top of a `.lunas` file, before the `html:` and
`script:` sections:

```lunas
@use Counter from "./Counter.lunas"
@use Card    from "./Card.lunas"
@use Badge   from "./Badge.lunas"

html:
    <main class="app">
        <Card tone="lead">
            <Counter :start="seed"/>
        </Card>
        <Badge :text="user"/>
    </main>

script:
    let seed = 10
    let user = "ada"
```

The grammar is:

```
@use <Name> from "<path>"
```

- **`<Name>`** is the local tag you will write in the template. It is the name
  you choose in *this* file — it does not have to match the child's filename,
  though matching keeps things readable.
- **`<path>`** is a module path, taken **verbatim**. The compiler does not
  rewrite or normalize it — extension handling is the module resolver's job (see
  [Paths](#paths) below).

### How it compiles

Each `@use` becomes a plain ES import in the emitted module:

```js
import Card from "./Card.lunas";
import Counter from "./Counter.lunas";
import Badge from "./Badge.lunas";
```

Two refinements the generator applies:

- **Only components actually used in the template are imported.** A `@use`
  declaration that is never referenced by a tag is dropped from the output — it
  is dead code. (The one exception is dynamic components: when a `<component
  :is="…">` tag is present, *all* `@use` factories are emitted, because the
  compiler cannot know statically which one `:is` will select. See
  [dynamic-components.md](./dynamic-components.md).)
- **Tag-name collisions with runtime identifiers are aliased.** If you name a
  component after a runtime helper — e.g. a component literally named `bind` —
  it is imported under a `$`-suffixed alias so it never shadows the runtime.

## Naming is case-sensitive

Tag names are matched **exactly** against the `@use` names. `Counter`, `counter`
and `COUNTER` are three different tags, and only the one that matches a `@use`
declaration resolves to a component. By convention Lunas components are named in
`PascalCase`, which also keeps them visually distinct from native HTML elements
(which are lowercase):

```lunas
@use UserCard from "./UserCard.lunas"

html:
    <UserCard :name="name"/>   <!-- resolves: matches @use UserCard -->
    <usercard :name="name"/>   <!-- does NOT resolve: treated as a plain tag -->
```

A tag that matches no `@use` name is left as an ordinary HTML element in the
static skeleton — it is **not** an error, so a typo in a component tag silently
renders as an unknown element rather than mounting your component. Watch the
casing.

## Paths

The path string is passed straight through to the emitted `import`, so it
follows your bundler's / runtime's module resolution:

```lunas
@use Counter from "./Counter.lunas"        <!-- relative, same folder -->
@use Header  from "../layout/Header.lunas" <!-- relative, parent folder -->
@use Button  from "@ui/Button.lunas"       <!-- alias resolved by the bundler -->
```

- **Relative paths** (`./`, `../`) resolve from the current file, as usual.
- **Bare / aliased paths** are handed to the resolver unchanged; configure
  aliases (e.g. `@ui`) in your bundler.
- The **`.lunas` extension** is written explicitly here because the compiler does
  not add or strip it. Whether your toolchain wants the extension present is a
  resolver concern, not a compiler one — write the path exactly as your setup
  expects to import it.

## Where components live

Lunas imposes no directory layout. A component is just a `.lunas` file that
exports a factory; put them wherever your project's conventions dictate (a
`components/` folder, colocated beside the feature that uses them, etc.). What
matters is that the `@use` path resolves to the file.

A single-root component compiles to a `component(tag, attrs, HTML, setup)`
factory as its default export; a multi-root component compiles to a
`fragment(...)` factory (see [fragments.md](./fragments.md)). Either way the
default export is the child factory that `@use` imports and that a parent mounts.

## Every reference needs a `@use`

There is no "auto-registration" or ambient/global component set. If a template
uses `<Widget/>`, the same file must contain `@use Widget from "…"`. This keeps
each module's dependencies explicit and lets the compiler tree-shake unused
imports.

## Related

- [props.md](./props.md) — declaring and passing `@input` props to a child.
- [events.md](./events.md) — child → parent communication with `emit`.
- [slots.md](./slots.md) — projecting parent content into a child.
- [dynamic-components.md](./dynamic-components.md) — `<component :is="expr">` and
  why it forces all `@use` imports to be emitted.
- [fragments.md](./fragments.md) — multi-root (wrapper-less) components.
- [../guide/](../guide/) — the getting-started guide.
