// dom.d.ts — types for src/dom.mjs
// DOM construction & wiring helpers.

import type { Context } from "./core.js";

/** Static attributes applied to the component's root element via setAttribute. */
export type RootAttrs = Record<string, string>;

/** Setup function: wires binds/children against the freshly-parsed root. */
export type SetupFn<P = Record<string, unknown>> = (
  c: Context<Element>,
  props: P
) => void;

/** The per-instance factory returned by component(...). */
export type ComponentFactory<P = Record<string, unknown>> = (
  props?: P
) => Element;

/**
 * component(tag, attrs, HTML, setup) — the compiled-component factory.
 * Builds the root detached, bulk-parses the comment-free static skeleton via
 * innerHTML, runs setup (wiring happens off-DOM), returns a factory that
 * builds one instance per call. The caller attaches the returned root to the
 * live DOM once.
 */
export function component<P = Record<string, unknown>>(
  tag: string,
  attrs: RootAttrs,
  HTML: string,
  setup: SetupFn<P>
): ComponentFactory<P>;

/**
 * refs(root, paths) — positional navigation to dynamic elements.
 * paths = [[0], [1, 2], ...]: each path is child indices from root.
 */
export function refs(root: Node, paths: number[][]): Node[];

/** on(el, ev, fn) — addEventListener shorthand. */
export function on(
  el: EventTarget,
  ev: string,
  fn: EventListenerOrEventListenerObject
): void;

/**
 * anchorBefore(node) — create a permanent empty text-node anchor immediately
 * before an existing node.
 */
export function anchorBefore(node: Node): Text;

/**
 * anchorBeforeSplit(textNode, utf16Offset) — split a static text node at the
 * given UTF-16 offset and place the anchor between head and tail (i.e. the
 * anchor sits before the tail). Used when a dynamic seam falls inside a text
 * run.
 */
export function anchorBeforeSplit(textNode: Text, utf16Offset: number): Text;

/**
 * anchorAppend(parent) — anchor as the last child of parent (e.g. a :for
 * slot at the end of a container).
 */
export function anchorAppend(parent: Node): Text;
