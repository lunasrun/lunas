//! Selector-list rewriting.
//!
//! Given the raw selector text that precedes a `{` (a *selector list*, e.g.
//! `.a, ul li`), this module rewrites each selector so every compound selector
//! carries the scope attribute, Vue-SFC style:
//!
//! ```text
//! .btn:hover        →  .btn[data-lunas-x]:hover
//! ul li             →  ul[data-lunas-x] li[data-lunas-x]
//! a > b             →  a[data-lunas-x] > b[data-lunas-x]
//! ```
//!
//! `:deep(inner)` scopes the compound it is attached to, then leaves everything
//! after it untouched. `:global(sel)` drops the pseudo and leaves its argument
//! fully unscoped.

use crate::tokenizer::{consume_bracketed, consume_comment, consume_string, Bracket, Scanner};

/// Splits a selector list into its top-level selectors (comma-separated),
/// respecting strings, comments, escapes, and bracket/paren nesting. Returns
/// each selector as a byte range `[start, end)` into `list`.
pub(crate) fn split_selector_list(list: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut sc = Scanner::new(list);
    let mut start = 0usize;
    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(&mut sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b'(') => {
                consume_bracketed(&mut sc, Bracket::Paren);
            }
            Some(b'[') => {
                consume_bracketed(&mut sc, Bracket::Square);
            }
            Some(b',') => {
                out.push((start, sc.pos()));
                sc.bump();
                start = sc.pos();
            }
            _ => {
                sc.bump();
            }
        }
    }
    out.push((start, sc.len()));
    out
}

/// A single lexical unit of a complex selector: either a combinator (` `, `>`,
/// `+`, `~`) or a compound selector (a run of simple selectors with no
/// combinator between them).
enum Unit {
    /// A combinator token, kept verbatim including surrounding whitespace.
    Combinator(String),
    /// A compound selector's byte range `[start, end)` into the selector text.
    Compound(usize, usize),
}

/// Rewrites one complex selector (no top-level commas) by attaching the scope
/// attribute to each compound selector. `scope_attr` is the bare attribute name
/// (e.g. `data-lunas-abc`); it is wrapped in `[…]` here.
///
/// `:deep(x)` — the compound it is attached to is scoped, then the descendant
/// combinator and `x` are emitted unscoped. `:global(x)` — the whole selector
/// from that point is emitted unscoped (Vue treats `:global` as a top-level
/// escape hatch; we scope nothing once it appears).
pub(crate) fn scope_complex_selector(sel: &str, scope_attr: &str) -> String {
    // Whitespace-only or empty (can happen with trailing commas): pass through.
    if sel.trim().is_empty() {
        return sel.to_string();
    }

    // A top-level `:global(...)` anywhere means the selector is authored as
    // global: nothing in it is scoped, and each `:global(x)` wrapper is
    // unwrapped to `x`, matching Vue.
    if let Some(unwrapped) = strip_globals(sel) {
        return unwrapped;
    }

    let units = split_units(sel);
    let mut out = String::with_capacity(sel.len() + scope_attr.len() + 2);
    let mut deep_reached = false;

    for unit in &units {
        match unit {
            Unit::Combinator(c) => out.push_str(c),
            Unit::Compound(s, e) => {
                let text = &sel[*s..*e];
                if deep_reached {
                    // Past a :deep(): emit compounds untouched.
                    out.push_str(text);
                    continue;
                }
                match find_deep(text) {
                    Some((before, inner, after)) => {
                        // `before:deep(inner)after` → scope `before`, then emit
                        // `inner` (unscoped) followed by `after` unscoped, and
                        // stop scoping the rest of the selector.
                        let scoped_before = attach_scope(before, scope_attr);
                        out.push_str(&scoped_before);
                        if !inner.is_empty() {
                            // Separate the scoped compound from the deep target
                            // with a descendant combinator, mirroring Vue.
                            if !scoped_before.is_empty() {
                                out.push(' ');
                            }
                            out.push_str(inner);
                        }
                        out.push_str(after);
                        deep_reached = true;
                    }
                    None => {
                        out.push_str(&attach_scope(text, scope_attr));
                    }
                }
            }
        }
    }
    out
}

/// If `sel` contains a top-level `:global(...)`, returns the selector with each
/// such wrapper replaced by its inner content (and nothing scoped). Returns
/// `None` when the selector has no top-level `:global`.
fn strip_globals(sel: &str) -> Option<String> {
    let mut out = String::with_capacity(sel.len());
    let mut found = false;
    let mut sc = Scanner::new(sel);
    let mut copied_to = 0usize;
    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(&mut sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b'[') => {
                consume_bracketed(&mut sc, Bracket::Square);
            }
            Some(b':') if sc.starts_with(":global") => {
                let kw_start = sc.pos();
                for _ in 0..7 {
                    sc.bump();
                }
                // Reject `:global-foo` style idents (longer keyword).
                if matches!(sc.peek(), Some(b) if is_ident_like(b)) {
                    continue;
                }
                while matches!(sc.peek(), Some(w) if is_ws(w)) {
                    sc.bump();
                }
                if sc.peek() != Some(b'(') {
                    continue;
                }
                let paren_start = sc.pos();
                consume_bracketed(&mut sc, Bracket::Paren);
                let paren_end = sc.pos();
                // Copy everything before `:global`, then the unwrapped inner.
                out.push_str(&sel[copied_to..kw_start]);
                let inner = sel
                    .get(paren_start + 1..paren_end.saturating_sub(1))
                    .unwrap_or("")
                    .trim();
                out.push_str(inner);
                copied_to = paren_end;
                found = true;
            }
            Some(b'(') => {
                consume_bracketed(&mut sc, Bracket::Paren);
            }
            _ => {
                sc.bump();
            }
        }
    }
    if found {
        out.push_str(&sel[copied_to..]);
        Some(out)
    } else {
        None
    }
}

fn is_ident_like(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

/// Splits a complex selector into an alternating-ish sequence of compounds and
/// combinators. Whitespace runs that separate compounds become descendant
/// combinators; `>`, `+`, `~` (with any surrounding whitespace) become their
/// respective combinators.
#[allow(unused_assignments)]
fn split_units(sel: &str) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut sc = Scanner::new(sel);
    let mut compound_start = 0usize;
    let mut have_compound = false;

    // Helper to flush a pending compound.
    macro_rules! flush {
        ($end:expr) => {{
            if have_compound && $end > compound_start {
                units.push(Unit::Compound(compound_start, $end));
            }
            have_compound = false;
        }};
    }

    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            // A comment on its own does not constitute a compound selector; it
            // is inert text attached to whatever compound surrounds it.
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                have_compound = true;
                consume_string(&mut sc);
            }
            Some(b'\\') => {
                have_compound = true;
                sc.bump();
                sc.bump();
            }
            Some(b'(') => {
                have_compound = true;
                consume_bracketed(&mut sc, Bracket::Paren);
            }
            Some(b'[') => {
                have_compound = true;
                consume_bracketed(&mut sc, Bracket::Square);
            }
            Some(b) if is_ws(b) => {
                let ws_start = sc.pos();
                // Consume the whitespace run.
                while matches!(sc.peek(), Some(w) if is_ws(w)) {
                    sc.bump();
                }
                // Skip any comments embedded between the whitespace and a
                // following combinator symbol so `a /* c */ > b` still parses.
                while consume_comment(&mut sc) {
                    while matches!(sc.peek(), Some(w) if is_ws(w)) {
                        sc.bump();
                    }
                }
                // A combinator symbol may follow the whitespace.
                if matches!(sc.peek(), Some(b'>' | b'+' | b'~')) {
                    sc.bump(); // the symbol
                               // trailing whitespace after the symbol
                    while matches!(sc.peek(), Some(w) if is_ws(w)) {
                        sc.bump();
                    }
                }
                // The combinator text starts where the preceding compound ends.
                // If there was a real compound, that is `ws_start`; otherwise the
                // whole `[compound_start, ws_start)` region is inert (comments)
                // and belongs to the combinator so it is not dropped.
                let comb_start = if have_compound {
                    ws_start
                } else {
                    compound_start
                };
                flush!(ws_start);
                units.push(Unit::Combinator(sel[comb_start..sc.pos()].to_string()));
                compound_start = sc.pos();
            }
            Some(b'>' | b'+' | b'~') => {
                // Combinator with no leading whitespace.
                let sym_pos = sc.pos();
                let comb_start = if have_compound {
                    sym_pos
                } else {
                    compound_start
                };
                flush!(sym_pos);
                sc.bump();
                while matches!(sc.peek(), Some(w) if is_ws(w)) {
                    sc.bump();
                }
                units.push(Unit::Combinator(sel[comb_start..sc.pos()].to_string()));
                compound_start = sc.pos();
            }
            _ => {
                have_compound = true;
                sc.bump();
            }
        }
    }
    flush!(sc.len());
    units
}

/// Within a single compound selector, finds a top-level `:deep(...)` and
/// returns `(before, inner, after)` byte-slices where `before` is the compound
/// text preceding `:deep`, `inner` is the argument, and `after` is any trailing
/// text after the closing paren.
fn find_deep(compound: &str) -> Option<(&str, &str, &str)> {
    let mut sc = Scanner::new(compound);
    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(&mut sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b'[') => {
                consume_bracketed(&mut sc, Bracket::Square);
            }
            Some(b':') if sc.starts_with(":deep") => {
                let before_end = sc.pos();
                // Advance past ":deep".
                for _ in 0..5 {
                    sc.bump();
                }
                // Skip whitespace before the paren.
                while matches!(sc.peek(), Some(w) if is_ws(w)) {
                    sc.bump();
                }
                if sc.peek() != Some(b'(') {
                    // `:deep` without `(` — not our pseudo, keep scanning.
                    continue;
                }
                let paren_start = sc.pos();
                consume_bracketed(&mut sc, Bracket::Paren);
                let paren_end = sc.pos();
                let before = &compound[..before_end];
                // inner excludes the parens; clamp for malformed input.
                let inner = compound
                    .get(paren_start + 1..paren_end.saturating_sub(1))
                    .unwrap_or("")
                    .trim();
                let after = &compound[paren_end..];
                return Some((before, inner, after));
            }
            Some(b'(') => {
                consume_bracketed(&mut sc, Bracket::Paren);
            }
            _ => {
                sc.bump();
            }
        }
    }
    None
}

/// Attaches `[scope_attr]` to a compound selector, inserting it after the type/
/// universal/class/id/attribute part but *before* any pseudo-class or
/// pseudo-element so that e.g. `.btn:hover` → `.btn[scope]:hover` and `::before`
/// → `[scope]::before`.
fn attach_scope(compound: &str, scope_attr: &str) -> String {
    // A compound with no real selector token (only comments / whitespace) is
    // inert and passed through untouched.
    if !has_selector_content(compound) {
        return compound.to_string();
    }
    let insert_at = pseudo_insertion_point(compound);
    let mut out = String::with_capacity(compound.len() + scope_attr.len() + 2);
    out.push_str(&compound[..insert_at]);
    out.push('[');
    out.push_str(scope_attr);
    out.push(']');
    out.push_str(&compound[insert_at..]);
    out
}

/// Finds the byte offset in a compound selector at which to insert the scope
/// attribute: just before the first top-level pseudo (`:` or `::`) that is not
/// part of a functional pseudo we scan through. If the compound is only a
/// pseudo (e.g. `::before` or `:hover`), the insertion point is 0.
fn pseudo_insertion_point(compound: &str) -> usize {
    let mut sc = Scanner::new(compound);
    // Position just past the last real (non-comment, non-whitespace) token, so a
    // trailing comment like `a/* c */` gets the scope inserted before it.
    let mut last_content_end = 0usize;
    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(&mut sc);
                last_content_end = sc.pos();
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
                last_content_end = sc.pos();
            }
            Some(b'[') => {
                consume_bracketed(&mut sc, Bracket::Square);
                last_content_end = sc.pos();
            }
            Some(b'(') => {
                consume_bracketed(&mut sc, Bracket::Paren);
                last_content_end = sc.pos();
            }
            Some(b':') => {
                return sc.pos();
            }
            Some(b) if is_ws(b) => {
                sc.bump();
            }
            _ => {
                sc.bump();
                last_content_end = sc.pos();
            }
        }
    }
    last_content_end
}

/// True if a compound contains at least one real selector token (i.e. something
/// other than comments and whitespace).
fn has_selector_content(compound: &str) -> bool {
    let mut sc = Scanner::new(compound);
    while !sc.is_eof() {
        if consume_comment(&mut sc) {
            continue;
        }
        match sc.peek() {
            Some(b) if is_ws(b) => {
                sc.bump();
            }
            Some(_) => return true,
            None => break,
        }
    }
    false
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}
