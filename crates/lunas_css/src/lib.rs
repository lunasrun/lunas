//! Component-scoped CSS for the Lunas compiler.
//!
//! This crate takes a component's raw `style:` block text and rewrites its
//! selectors so the styles only match that component's DOM, exactly like a
//! Vue single-file-component `<style scoped>`. It is a hand-written transform:
//! there is no external CSS parser dependency (keeping the wasm32 build lean),
//! and it never panics — malformed input is passed through as-is for the
//! unparseable region and reported with a [`Diagnostic`].
//!
//! # What scoping means
//!
//! Every compound selector gains the component's scope attribute:
//!
//! ```
//! use lunas_css::scope_css;
//!
//! let (out, diags) = scope_css(".btn:hover { color: red }", "data-lunas-x");
//! assert_eq!(out, ".btn[data-lunas-x]:hover { color: red }");
//! assert!(diags.is_empty());
//! ```
//!
//! Descendant selectors scope every compound:
//!
//! ```
//! # use lunas_css::scope_css;
//! let (out, _) = scope_css("ul li { }", "data-lunas-x");
//! assert_eq!(out, "ul[data-lunas-x] li[data-lunas-x] { }");
//! ```
//!
//! # Escape hatches (Vue semantics)
//!
//! * `:deep(sel)` — the compound it is attached to is scoped, and everything
//!   from `sel` onward is left unscoped, letting styles reach into child
//!   components:
//!
//! ```
//! # use lunas_css::scope_css;
//! let (out, _) = scope_css(".a :deep(.b) { }", "data-lunas-x");
//! assert_eq!(out, ".a[data-lunas-x] .b { }");
//! ```
//!
//! * `:global(sel)` — the selector is emitted verbatim, unscoped:
//!
//! ```
//! # use lunas_css::scope_css;
//! let (out, _) = scope_css(":global(.modal) { }", "data-lunas-x");
//! assert_eq!(out, ".modal { }");
//! ```
//!
//! # At-rules
//!
//! * `@media` / `@supports` / `@layer` / `@container` — the walker recurses into
//!   the block and scopes the rules inside.
//! * `@keyframes name` — the animation name is renamed with a scope-derived
//!   suffix, and `animation` / `animation-name` declarations that reference it
//!   are rewritten to match.
//! * `@font-face` / `@page` / `@import` / `@charset` and unknown at-rules — passed
//!   through untouched (their bodies hold declarations, not selectors).
//!
//! ```
//! # use lunas_css::scope_css;
//! let src = "@keyframes spin { to { transform: rotate(1turn) } }\n\
//!            .x { animation: spin 1s }";
//! let (out, _) = scope_css(src, "data-lunas-ab12");
//! assert!(out.contains("@keyframes spin-ab12"));
//! assert!(out.contains("animation: spin-ab12 1s"));
//! ```

mod rewrite;
mod scope_hash;
mod selector;
mod tokenizer;

pub use lunas_span::Diagnostic;

/// Rewrites a component's CSS so every selector is scoped by `scope_attr`.
///
/// `scope_attr` is the bare attribute name that codegen also stamps onto the
/// component's DOM elements (e.g. `data-lunas-abc123`, as produced by
/// [`scope_id`]). Each compound selector in every rule gains `[scope_attr]`.
///
/// The returned [`Diagnostic`]s (all warnings) describe recoverable problems
/// such as unterminated blocks; their [`TextRange`](lunas_span::TextRange) is a
/// byte range into the input `css`. Even when diagnostics are produced, the
/// returned string is always a valid best-effort transform of the whole input.
///
/// This function never panics for any input.
///
/// ```
/// use lunas_css::scope_css;
///
/// let (out, diags) = scope_css("a, b > c { color: red }", "data-lunas-x");
/// assert_eq!(out, "a[data-lunas-x], b[data-lunas-x] > c[data-lunas-x] { color: red }");
/// assert!(diags.is_empty());
/// ```
pub fn scope_css(css: &str, scope_attr: &str) -> (String, Vec<Diagnostic>) {
    rewrite::rewrite_stylesheet(css, scope_attr)
}

/// Produces a short, stable scope attribute name for a component from its
/// source text: `data-lunas-<hash>`, where `<hash>` is a hex FNV-1a digest.
///
/// The same source always yields the same id (stable across builds and
/// platforms), and different sources almost always differ. Codegen uses this
/// as the single source of truth for a component's scope: the attribute is
/// stamped on the component's root and descendant elements, and the same string
/// is passed to [`scope_css`] so the emitted `<style>` matches.
///
/// ```
/// use lunas_css::scope_id;
///
/// let id = scope_id("<template>…</template>");
/// assert!(id.starts_with("data-lunas-"));
/// // Deterministic.
/// assert_eq!(id, scope_id("<template>…</template>"));
/// ```
pub fn scope_id(component_source: &str) -> String {
    format!("data-lunas-{}", scope_hash::fnv1a_hex(component_source))
}
