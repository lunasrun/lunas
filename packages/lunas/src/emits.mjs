// emits.mjs — child → parent events (c-emits).
// See output-design.md §5 (runtime API) and §6 (syntax → output mapping).
//
// A child raises a named event with an optional payload; the parent listens by
// passing a handler prop named `on<Name>` (camel-cased). This mirrors Vue's
// `@name` on a component tag, which the future codegen lowers to an `onName`
// prop on the mountChild props object (documented in §5's emits block):
//
//   <Child @save="handle($event)" />   ⇒   mountChild(c, a, Child, {
//                                             onSave: (payload) => handle(payload)
//                                           })
//
// In the child's setup, `emit(c, "save", payload)` looks up `onSave` in the
// props the child was constructed with and calls it. Critically, emit does NOT
// mark the parent dirty: the two contexts are independent (§6). If the handler
// mutates parent reactive state, the box setters mark the parent — the handler
// decides. A no-op when there is no listener (no parent, or prop absent).

// eventPropName("save") → "onSave"; ("update:model") → "onUpdate:model" is NOT
// produced — event names are plain identifiers; the codegen maps `@name` to the
// camel-cased `on` + Capitalized name. Kebab names ("save-all") map to
// "onSaveAll".
export function eventPropName(name) {
  // Split on "-" and capitalize each segment after the first char of the whole.
  const camel = name.replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
  return "on" + camel.charAt(0).toUpperCase() + camel.slice(1);
}

// registerEmits(c, props, declared?) — called at the top of a child `setup` to
// stash the props (so emit can find handlers) and optionally the declared event
// names for lean validation. Additive: stores on `c._emitProps`. Returns c.
export function registerEmits(c, props, declared) {
  c._emitProps = props || {};
  if (declared) c._emits = declared; // array of allowed event names (optional)
  return c;
}

// emit(c, name, payload) — invoke the parent's `on<Name>` handler, if any.
// Returns true if a handler ran, false otherwise (no listener). Never marks the
// parent dirty by itself.
export function emit(c, name, payload) {
  const props = c && c._emitProps;
  if (c && c._emits && c._emits.indexOf(name) < 0) {
    // Optional lean validation: warn (do not throw) on an undeclared event.
    if (typeof console !== "undefined" && console.warn) {
      console.warn(
        'lunas: emitted undeclared event "' + name + '" (not in emits list)'
      );
    }
  }
  if (!props) return false;
  const handler = props[eventPropName(name)];
  if (typeof handler === "function") {
    handler(payload);
    return true;
  }
  return false;
}
