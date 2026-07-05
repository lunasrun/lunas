// lunas — minimal runtime for Lunas-compiled components.
// Plain ESM, no build step. Compatibility floor: ES2015 + Proxy. No BigInt.
// Contract: crates/lunas_compiler/docs/output-design.md
//           crates/lunas_compiler/docs/for-diff-design.md

export {
  createContext,
  bind,
  markVar,
  flush,
  afterFlush,
  unbind,
  beginScope,
  endScope,
  dropScope,
  runScope,
} from "./core.mjs";

export { box, deepBox, shared, prop } from "./boxes.mjs";

export { computed } from "./computed.mjs";

export { watch, watchEffect } from "./watch.mjs";

export { nextTick, batch } from "./batch.mjs";

export {
  component,
  refs,
  on,
  fromHTML,
  anchorBefore,
  anchorBeforeSplit,
  anchorAppend,
} from "./dom.mjs";

export { ifBlock, ifChain, forBlock, mountChild } from "./blocks.mjs";

export {
  createForState,
  seedForState,
  reconcile,
  extractKeys,
  longestIncreasingSubsequence,
} from "./for_diff.mjs";

export { createStore, useStore, derivedStore } from "./store.mjs";

export {
  createRouter,
  memoryHistory,
  historyAdapter,
  routerOutlet,
  routerLink,
} from "./router.mjs";
