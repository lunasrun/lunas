// keepalive.d.ts — types for src/keepalive.mjs
// Component instance caching (c-keepalive).

import type { Context } from "./core.js";
import type { ChildFactory, MountedChild } from "./blocks.js";

/** Options for {@link keepAlive}. */
export interface KeepAliveOptions {
  /** LRU capacity; unbounded when omitted. Overflow evicts the least-recently
   *  shown instance (firing its onDestroy). */
  max?: number;
}

/** A mountChild handle extended with the cache key it is stored under. */
export interface KeptChild extends MountedChild {
  ctx?: Context;
  key?: unknown;
}

/** The controller returned by {@link keepAlive}. */
export interface KeepAliveController {
  /** show(c, anchor, key, factory, props?) — make the instance for `key` the
   *  one mounted before `anchor`, activating a cached instance or mounting a
   *  fresh one and deactivating the previously-shown instance. */
  show<P = Record<string, unknown>>(
    c: Context,
    anchor: Node,
    key: unknown,
    factory: ChildFactory<P>,
    props?: P
  ): KeptChild;
  /** has(key) — whether an instance is currently cached. */
  has(key: unknown): boolean;
  /** Number of cached instances. */
  readonly size: number;
  /** destroy() — evict and destroy every cached instance. */
  destroy(): void;
}

/**
 * keepAlive(opts) — cache mountChild-produced instances by key instead of
 * destroying them on switch. Deactivation detaches nodes (keeping the context
 * and reactive state alive); activation re-attaches with no rebuild. Real
 * eviction (LRU overflow / destroy) fires onDestroy; activate/deactivate fire
 * onActivated/onDeactivated (lifecycle.mjs).
 */
export function keepAlive(opts?: KeepAliveOptions): KeepAliveController;
