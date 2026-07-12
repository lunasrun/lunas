# E2E fixtures

Representative `.lunas` components exercising the whole compiler feature matrix.
Each fixture is compiled by:

- `codegen_snapshot.rs` — emits the JS and diffs it against `tests/snapshots/<name>.js`.
- `codegen_app.rs` — for the multi-file app fixtures, runs the emitted modules
  against the real runtime under the node dom-shim.
- `browser_smoke.rs` — pre-compiles a subset, inlines runtime + compiled JS, and
  drives them in real headless Chrome.

Single-file fixtures live at the top level. Multi-file app fixtures live under
`app/` (a parent + its children, wired through `@use`).

## Feature coverage map

| fixture              | features exercised                                           |
| -------------------- | ------------------------------------------------------------ |
| `text_attr_event`    | text bind, mixed text run, bound attr, static-interp attr, event listener |
| `two_way`            | `::value` (input) + `::checked` (change) two-way bindings    |
| `if_cascade`         | `:if` / `:elseif` / `:else` chain, nested reactive text      |
| `for_keyed_nested`   | keyed `:for`, nested `:for`, nested `:if` inside an item     |
| `class_style`        | `:class` object + static merge, `:style` object              |
| `ref_html`           | `:ref` element handle, `:html` raw insertion                 |
| `dynamic_teleport`   | `<component :is>`, `<teleport>` portal                       |
| `multi_root`         | fragment / multi-root component with a top-level `:if`       |
| `app/App`            | parent: child components, `@input` props, slots, `<component :is>`, `@use` |
| `app/Counter`        | `@input` prop, local state, event, two-way                  |
| `app/Card`           | default + named + scoped slots, slot fallback                |
| `app/Badge`          | tiny leaf child (`@input`), used through `:is` and in `:for` |
