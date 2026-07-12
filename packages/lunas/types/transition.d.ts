// transition.d.ts — types for src/transition.mjs
// CSS-class enter/leave transitions (c-transition).

/** Options for {@link withTransition} / {@link runPhase}. */
export interface TransitionOptions {
  /** Class base name (default "v"); classes are `name-enter-from` etc. */
  name?: string;
  /** Fallback timeout in ms after which a phase completes even if no
   *  `transitionend` fires (0 → next macrotask). */
  duration?: number;
}

/** The transition phase: entering or leaving. */
export type TransitionPhase = "enter" | "leave";

/**
 * runPhase(el, base, phase, opts, done) — run one enter/leave class
 * choreography on a single element:
 *   frame 0  + base-phase-from  + base-phase-active
 *   frame 1  − base-phase-from  + base-phase-to
 *   end      − base-phase-active − base-phase-to  (transitionend / timeout)
 * Calls `done()` when the phase completes. In a non-browser env (no
 * requestAnimationFrame) the sequence runs synchronously and `done()` fires
 * immediately. Returns a `cancel()`.
 */
export function runPhase(
  el: Element,
  base: string,
  phase: TransitionPhase,
  opts: TransitionOptions | undefined,
  done: (() => void) | null
): () => void;

/** The controller returned by {@link withTransition}. */
export interface TransitionController {
  /** enter(nodes, insert) — insert the nodes, then choreograph enter classes. */
  enter(nodes: Node | Node[], insert: () => void): void;
  /** leave(nodes, remove) — choreograph leave classes, then call `remove()`
   *  once every node's leave phase finishes. */
  leave(nodes: Node | Node[], remove: () => void): void;
}

/**
 * withTransition(opts) — build a transition controller composing with a block's
 * insert/remove closures. Degrades to immediate insert/remove (with the class
 * sequence still applied synchronously) outside a browser.
 */
export function withTransition(opts?: TransitionOptions): TransitionController;
