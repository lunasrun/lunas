// index.d.ts — public API surface, mirroring src/index.mjs's re-exports.
// lunas — minimal runtime for Lunas-compiled components.
// Plain ESM, no build step. Compatibility floor: ES2015 + Proxy. No BigInt.

export type {
  BindRecord,
  Scope,
  Context,
} from "./core.js";

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
} from "./core.js";

export type { Box, Shared } from "./boxes.js";

export { box, deepBox, shared } from "./boxes.js";

export type { Computed } from "./computed.js";

export { computed } from "./computed.js";

export type { StopHandle, WatchOpts } from "./watch.js";

export { watch, watchEffect } from "./watch.js";

export { nextTick, batch } from "./batch.js";

export type {
  RootAttrs,
  SetupFn,
  ComponentFactory,
} from "./dom.js";

export {
  component,
  refs,
  on,
  anchorBefore,
  anchorBeforeSplit,
  anchorAppend,
} from "./dom.js";

export type {
  BlockNodes,
  BlockHandle,
  ForBlockOpts,
  ChildFactory,
  MountedChild,
} from "./blocks.js";

export { ifBlock, forBlock, mountChild } from "./blocks.js";

export type {
  Key,
  KeyOf,
  PatchItem,
  WarnFn,
  ReconcileHost,
  MakeItem,
  ForState,
  ForSeed,
  ReconcileOpts,
} from "./for_diff.js";

export {
  createForState,
  seedForState,
  reconcile,
  longestIncreasingSubsequence,
} from "./for_diff.js";

export type { Store, StoreField, Unsubscribe } from "./store.js";

export { createStore, useStore, derivedStore } from "./store.js";

export type {
  AsyncModule,
  AsyncLoader,
  AsyncComponentOptions,
  SuspenseHandle,
} from "./async.js";

export { asyncComponent, mountAsyncChild, suspenseBlock } from "./async.js";

export {
  onMount,
  onDestroy,
  onUpdate,
  onActivated,
  onDeactivated,
  attach,
  isLive,
} from "./lifecycle.js";

export { emit, registerEmits, eventPropName } from "./emits.js";

export type { InjectionKey } from "./provide.js";
export { provide, inject, hasInjection } from "./provide.js";

export type {
  TransitionOptions,
  TransitionPhase,
  TransitionController,
} from "./transition.js";
export { withTransition, runPhase } from "./transition.js";

export type {
  KeepAliveOptions,
  KeepAliveController,
  KeptChild,
} from "./keepalive.js";
export { keepAlive } from "./keepalive.js";
