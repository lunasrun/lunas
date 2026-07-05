// provide.mjs — dependency injection down the component tree (c-provide).
// See output-design.md §5 (runtime API) and §6.
//
// A component `provide(c, key, value)`s a value; any descendant
// `inject(c, key, default?)`s it by walking the parent-context chain
// (`c.parent`, linked by mountChild — blocks.mjs). The nearest ancestor that
// provided `key` wins (shadowing). Keys may be strings or Symbols.
//
// The parent link is additive and populated only for children mounted via
// mountChild; a root component's `c.parent` is null, so inject on a root falls
// straight through to the default.

// provide(c, key, value) — register `key → value` on this component's context.
// A later provide of the same key on the same context overwrites it.
export function provide(c, key, value) {
  const m = c._provides || (c._provides = new Map());
  m.set(key, value);
  return value;
}

// inject(c, key, def) — resolve `key` from the nearest ancestor that provided
// it (this context included). Returns `def` (default undefined) when no ancestor
// provides the key. `def` may be a factory: pass `{ factory: fn }`? No — kept
// simple: `def` is a plain value to match Vue's common case; callers wanting a
// lazy default can pass a thunk and call it. The chain walk is O(depth).
export function inject(c, key, def) {
  let n = c;
  while (n) {
    const m = n._provides;
    if (m && m.has(key)) return m.get(key);
    n = n.parent;
  }
  return def;
}

// hasInjection(c, key) — true if any ancestor (or self) provides `key`. Useful
// for optional-injection call sites that must distinguish "provided undefined"
// from "not provided".
export function hasInjection(c, key) {
  let n = c;
  while (n) {
    const m = n._provides;
    if (m && m.has(key)) return true;
    n = n.parent;
  }
  return false;
}
