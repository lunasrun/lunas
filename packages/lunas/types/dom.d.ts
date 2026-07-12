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
 * A multi-root component's node group: an Array of DOM nodes carrying the
 * component context on `__lunasCtx` (like a single-root ComponentFactory's
 * return value, but as a fragment of nodes instead of a single root).
 */
export type FragmentNodes = Node[] & { __lunasCtx?: unknown };

/** The per-instance factory returned by fragment(...). */
export type FragmentFactory<P = Record<string, unknown>> = (
  props?: P
) => FragmentNodes;

/**
 * fragment(attrs, HTML, setup) — the compiled factory for a MULTI-ROOT
 * component. Unlike `component`, there is no wrapper element: the static
 * skeleton HTML has several top-level nodes, so the factory parses it into a
 * throwaway host and returns the host's child nodes as a fragment. `attrs` is
 * accepted for signature parity but ignored (there is no single root to
 * attribute).
 */
export function fragment<P = Record<string, unknown>>(
  attrs: RootAttrs,
  HTML: string,
  setup: SetupFn<P>
): FragmentFactory<P>;

/**
 * fromHTML(html, near) — parse a static block skeleton (an :if branch, a
 * :for item, …) into a detached scratch element via one bulk innerHTML.
 * `near` is any node used to reach the owner document.
 */
export function fromHTML(html: string, near?: Node | null): Element;

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

/**
 * normClass(value) — flatten a `:class` binding into a space-separated
 * string. Falsy entries are dropped; object keys are included when their
 * value is truthy; arrays are flattened recursively.
 */
export type ClassValue =
  | string
  | number
  | null
  | undefined
  | boolean
  | Record<string, unknown>
  | ClassValue[];
export function normClass(value: ClassValue): string;

/**
 * setClass(el, staticClass, value) — merge the element's static class string
 * with the normalized dynamic `value` and write the whole `class` attribute.
 */
export function setClass(
  el: Element,
  staticClass: string | undefined,
  value: ClassValue
): void;

/**
 * normStyle(value) — flatten a `:style` binding into a `prop: value;` string.
 * A string passes through; an object maps camelCase keys to kebab-case CSS
 * properties; arrays merge left-to-right (later entries win).
 */
export type StyleValue =
  | string
  | null
  | undefined
  | boolean
  | Record<string, unknown>
  | StyleValue[];
export function normStyle(value: StyleValue): string;

/**
 * setStyle(el, staticStyle, value) — merge the static style string with the
 * normalized dynamic `value` and write the whole `style` attribute.
 */
export function setStyle(
  el: Element,
  staticStyle: string | undefined,
  value: StyleValue
): void;
