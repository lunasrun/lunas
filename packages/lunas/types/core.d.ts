// core.d.ts — types for src/core.mjs
// A component context `c` created by createContext(root) and threaded
// through bind/markVar/flush/unbind and the scope helpers.

/** A single registered update function plus its bookkeeping. Opaque to callers. */
export interface BindRecord {
  fn: () => void;
  q: boolean;
  alive: boolean;
  deps: number[];
}

/** Teardown bookkeeping for a control-flow block's inner binds (see blocks.mjs). */
export interface Scope {
  subs: BindRecord[];
  children: Scope[];
  parent: Scope | null;
}

/**
 * A component context: { root, deps, queue, pending, scope, post }.
 * `deps[i]` is the adjacency list of bind records that read reactive
 * variable `i`. `root` is whatever value the caller passed to
 * createContext (typically the component's root DOM node). `post` is the
 * list of pending afterFlush callbacks (nextTick's primitive); `batchDepth`
 * is used internally by batch() in batch.mjs.
 */
export interface Context<R = unknown> {
  root: R;
  deps: (BindRecord[] | undefined)[];
  queue: BindRecord[];
  pending: boolean;
  scope: Scope | null;
  post: (() => void)[] | null;
  batchDepth?: number;
  /** Link to the parent component's context (set by mountChild); null on a
   *  root. provide/inject (provide.mjs) walks this chain. Additive. */
  parent: Context | null;
  /** Optional per-context post-update hook the flush loop invokes after an
   *  update pass that actually ran; wired by onUpdate (lifecycle.mjs). */
  onUpdate: (() => void) | null;
}

/** Create a fresh reactive context rooted at `root`. */
export function createContext<R = unknown>(root: R): Context<R>;

/**
 * Register an update function that reads the reactive variable indices in
 * `deps`. Runs `fn` once immediately. Returns the bind record (needed for
 * unbind).
 */
export function bind(
  c: Context,
  deps: number[],
  fn: () => void
): BindRecord;

/**
 * Reactive variable `i` changed: enqueue its dependents (deduplicated) and
 * schedule a microtask flush of `c`.
 */
export function markVar(c: Context, i: number): void;

/**
 * Run every queued update once. Only affected parts run. After the update
 * pass, drains any post-flush callbacks registered via `afterFlush`.
 */
export function flush(c: Context): void;

/**
 * Run `cb` once, after the next flush completes (i.e. after the DOM update
 * pass). If a flush is already pending the callback rides that one;
 * otherwise a flush is scheduled so the callback still fires this tick.
 * This is the primitive behind nextTick() (see batch.mjs).
 */
export function afterFlush(c: Context, cb: () => void): void;

/**
 * Permanently unregister a bind record. Safe to call while a flush
 * containing `s` is pending (the dead record is skipped at flush).
 */
export function unbind(c: Context, s: BindRecord): void;

/**
 * Open a new collection scope, nested under the context's currently-open
 * scope (if any). Every bind created until the matching endScope is
 * collected into the returned scope.
 */
export function beginScope(c: Context): Scope;

/** Close the currently-open scope, restoring the parent as current. */
export function endScope(c: Context): void;

/**
 * Unregister every bind collected in `scope` (recursively, including
 * nested child scopes) and detach `scope` from its parent.
 */
export function dropScope(c: Context, scope: Scope): void;
