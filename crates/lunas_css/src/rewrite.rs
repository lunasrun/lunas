//! Structural CSS walker that drives selector scoping and at-rule handling.
//!
//! The walker splits a stylesheet body into a sequence of top-level items:
//!
//! * an **at-rule** — starts with `@`, ends at either the matching `;` (e.g.
//!   `@import …;`) or a balanced `{ … }` block;
//! * a **qualified rule** — a selector list followed by a `{ … }` declaration
//!   block;
//! * trailing junk (e.g. an unterminated block) which is emitted verbatim with
//!   a diagnostic.
//!
//! Only selectors of qualified rules and the bodies of conditional group
//! at-rules (`@media`, `@supports`, `@layer`) are rewritten. `@keyframes` are
//! renamed; `@font-face`, `@page`, `@import`, `@charset` and unknown at-rules
//! pass through (their bodies are declaration blocks with no selectors, so they
//! must not be scoped).

use lunas_span::{Diagnostic, TextRange};

use crate::selector::{scope_complex_selector, split_selector_list};
use crate::tokenizer::{consume_bracketed, consume_comment, consume_string, Bracket, Scanner};

/// Shared state threaded through the recursive walk.
pub(crate) struct Ctx<'a> {
    pub(crate) scope_attr: &'a str,
    /// Base byte offset of the region currently being walked, so diagnostics
    /// point into the *original* css even when we recurse into a sub-slice.
    pub(crate) base: usize,
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// Keyframe names discovered in this stylesheet, mapped to their scoped
    /// rename, so `animation`/`animation-name` references can be rewritten.
    pub(crate) keyframes: Vec<(String, String)>,
}

impl Ctx<'_> {
    fn diag(&mut self, start: usize, end: usize, msg: impl Into<String>) {
        let range = TextRange::at((self.base + start) as u32, (self.base + end) as u32);
        self.diagnostics.push(Diagnostic::warning(range, msg));
    }
}

/// Rewrites a stylesheet `body`, appending output to `out`. Two passes are run:
/// pass one collects `@keyframes` names (so forward references in `animation`
/// resolve), pass two performs the rewrite. Both passes use the same walker.
pub(crate) fn rewrite_stylesheet(css: &str, scope_attr: &str) -> (String, Vec<Diagnostic>) {
    let mut ctx = Ctx {
        scope_attr,
        base: 0,
        diagnostics: Vec::new(),
        keyframes: Vec::new(),
    };

    // Pass one: collect keyframe names (no output, no diagnostics kept).
    collect_keyframes(css, scope_attr, &mut ctx.keyframes);

    let mut out = String::with_capacity(css.len() + 16);
    walk_block(css, &mut ctx, &mut out);
    (out, ctx.diagnostics)
}

/// Pass one: find every `@keyframes name` (and `@-*-keyframes name`) and record
/// `name -> name-<scopehash>`.
fn collect_keyframes(css: &str, scope_attr: &str, out: &mut Vec<(String, String)>) {
    let suffix = keyframe_suffix(scope_attr);
    let mut sc = Scanner::new(css);
    while !sc.is_eof() {
        if skip_inert(&mut sc) {
            continue;
        }
        if sc.peek() == Some(b'@') {
            let at_start = sc.pos();
            let name = read_at_keyword(&mut sc);
            if is_keyframes_keyword(name) {
                // Read the animation name that follows.
                skip_ws_and_comments(&mut sc);
                let n_start = sc.pos();
                read_ident(&mut sc);
                let kf_name = &css[n_start..sc.pos()];
                if !kf_name.is_empty() {
                    let scoped = format!("{kf_name}-{suffix}");
                    if !out.iter().any(|(k, _)| k == kf_name) {
                        out.push((kf_name.to_string(), scoped));
                    }
                }
                // Skip the block so nested `{` from keyframe selectors don't
                // confuse the top-level scan.
                skip_to_block_and_over(&mut sc);
            } else {
                let _ = at_start;
                // Not keyframes: skip to end of its statement/block.
                skip_at_rule_tail(&mut sc);
            }
        } else {
            // Qualified rule: skip its block.
            skip_prelude_and_block(&mut sc);
        }
    }
}

/// The recursive workhorse: walks a `{`-delimited body (or the whole sheet when
/// `base`-relative), emitting rewritten CSS into `out`.
fn walk_block(body: &str, ctx: &mut Ctx, out: &mut String) {
    let mut sc = Scanner::new(body);
    loop {
        // Emit leading inert (whitespace/comments) verbatim.
        let inert_start = sc.pos();
        while skip_inert(&mut sc) {}
        out.push_str(&body[inert_start..sc.pos()]);

        if sc.is_eof() {
            break;
        }

        if sc.peek() == Some(b'@') {
            walk_at_rule(body, &mut sc, ctx, out);
        } else {
            walk_qualified_rule(body, &mut sc, ctx, out);
        }
    }
}

/// Handles a qualified rule: `<selector-list> { <declarations> }`.
fn walk_qualified_rule(body: &str, sc: &mut Scanner, ctx: &mut Ctx, out: &mut String) {
    let prelude_start = sc.pos();
    // Scan the prelude up to the opening brace (or `;`/EOF for malformed input).
    let brace = scan_to_brace_or_semi(sc);
    match brace {
        BraceScan::Brace(open) => {
            let prelude = &body[prelude_start..open];
            let scoped = scope_selector_list_text(prelude, ctx.scope_attr);
            out.push_str(&scoped);
            out.push('{');
            // Move past `{`.
            sc.seek(open + 1);
            walk_declaration_block(body, sc, ctx, out);
        }
        BraceScan::Semi(semi) => {
            // A stray `;` with no block — emit prelude+`;` verbatim.
            out.push_str(&body[prelude_start..=semi]);
            sc.seek(semi + 1);
        }
        BraceScan::Eof => {
            let text = &body[prelude_start..sc.len()];
            if !text.trim().is_empty() {
                ctx.diag(prelude_start, sc.len(), "unterminated rule (missing `{`)");
            }
            out.push_str(text);
            sc.seek(sc.len());
        }
    }
}

/// Walks the declarations inside a qualified rule's block, rewriting
/// `animation`/`animation-name` values that reference a scoped keyframe. The
/// cursor starts just after `{`.
fn walk_declaration_block(body: &str, sc: &mut Scanner, ctx: &mut Ctx, out: &mut String) {
    let block_start = sc.pos();
    match balanced_block_end(sc) {
        Some(close) => {
            let inner = &body[block_start..close];
            rewrite_declarations(inner, ctx, out);
            out.push('}');
            sc.seek(close + 1);
        }
        None => {
            let inner = &body[block_start..sc.len()];
            ctx.diag(block_start, sc.len(), "unterminated block (missing `}`)");
            rewrite_declarations(inner, ctx, out);
            sc.seek(sc.len());
        }
    }
}

/// Rewrites `animation` / `animation-name` values inside a declaration block so
/// keyframe references point at their scoped rename. Everything else passes
/// through byte-for-byte.
fn rewrite_declarations(inner: &str, ctx: &mut Ctx, out: &mut String) {
    if ctx.keyframes.is_empty() {
        out.push_str(inner);
        return;
    }
    let mut sc = Scanner::new(inner);
    // Track declaration boundaries: a declaration is `prop : value ;`.
    loop {
        // Read a property name up to `:` (or `}`/`;`/EOF).
        let seg_start = sc.pos();
        let mut colon: Option<usize> = None;
        while !sc.is_eof() {
            if consume_comment(&mut sc) {
                continue;
            }
            match sc.peek() {
                Some(b'"' | b'\'') => {
                    consume_string(&mut sc);
                }
                Some(b'{') => {
                    // Nested block (shouldn't normally appear); bail to verbatim.
                    consume_bracketed_curly(&mut sc);
                }
                Some(b';') => break,
                Some(b':') => {
                    colon = Some(sc.pos());
                    break;
                }
                _ => {
                    sc.bump();
                }
            }
        }
        let Some(colon_pos) = colon else {
            // No `:` — emit the rest verbatim.
            out.push_str(&inner[seg_start..sc.pos()]);
            if sc.is_eof() {
                break;
            }
            // We stopped on `;`; emit it and continue.
            if sc.peek() == Some(b';') {
                out.push(';');
                sc.bump();
            }
            continue;
        };
        let prop = inner[seg_start..colon_pos].trim();
        let is_anim =
            prop.eq_ignore_ascii_case("animation") || prop.eq_ignore_ascii_case("animation-name");
        // Emit `prop:`.
        out.push_str(&inner[seg_start..=colon_pos]);
        sc.bump(); // past `:`
        let val_start = sc.pos();
        // Read the value up to `;` or EOF, respecting strings/comments/parens.
        while !sc.is_eof() {
            if consume_comment(&mut sc) {
                continue;
            }
            match sc.peek() {
                Some(b'"' | b'\'') => {
                    consume_string(&mut sc);
                }
                Some(b'(') => {
                    consume_bracketed(&mut sc, Bracket::Paren);
                }
                Some(b';') => break,
                _ => {
                    sc.bump();
                }
            }
        }
        let value = &inner[val_start..sc.pos()];
        if is_anim {
            out.push_str(&rewrite_animation_value(value, &ctx.keyframes));
        } else {
            out.push_str(value);
        }
        if sc.peek() == Some(b';') {
            out.push(';');
            sc.bump();
        } else {
            break;
        }
    }
}

/// Replaces whole-token keyframe names inside an `animation`/`animation-name`
/// value. Tokens are runs of ident characters; only exact matches are renamed.
fn rewrite_animation_value(value: &str, keyframes: &[(String, String)]) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if is_ident_byte(b) || b == b'-' {
            let start = i;
            while i < bytes.len() && (is_ident_byte(bytes[i]) || bytes[i] == b'-') {
                i += 1;
            }
            let tok = &value[start..i];
            if let Some((_, scoped)) = keyframes.iter().find(|(k, _)| k == tok) {
                out.push_str(scoped);
            } else {
                out.push_str(tok);
            }
        } else {
            // Copy the (possibly multi-byte) char verbatim.
            let ch_len = utf8_len(b);
            let end = (i + ch_len).min(bytes.len());
            out.push_str(&value[i..end]);
            i = end;
        }
    }
    out
}

/// Handles any at-rule.
fn walk_at_rule(body: &str, sc: &mut Scanner, ctx: &mut Ctx, out: &mut String) {
    let at_start = sc.pos();
    let name = read_at_keyword(sc);

    if is_conditional_group(name) {
        // `@media (...)` / `@supports (...)` / `@layer name` { <rules> }
        // Emit the prelude verbatim, then recurse into the block.
        let prelude_start = sc.pos();
        match scan_to_brace_or_semi(sc) {
            BraceScan::Brace(open) => {
                out.push_str(&body[at_start..open]);
                out.push('{');
                let block_start = open + 1;
                sc.seek(block_start);
                match balanced_block_end(sc) {
                    Some(close) => {
                        let inner = &body[block_start..close];
                        // Recurse with a shifted base for correct diagnostics.
                        let saved_base = ctx.base;
                        ctx.base = saved_base + block_start;
                        walk_block(inner, ctx, out);
                        ctx.base = saved_base;
                        out.push('}');
                        sc.seek(close + 1);
                    }
                    None => {
                        let inner = &body[block_start..sc.len()];
                        ctx.diag(block_start, sc.len(), "unterminated at-rule block");
                        let saved_base = ctx.base;
                        ctx.base = saved_base + block_start;
                        walk_block(inner, ctx, out);
                        ctx.base = saved_base;
                        sc.seek(sc.len());
                    }
                }
            }
            BraceScan::Semi(semi) => {
                // `@layer a, b;` — statement form. Pass through.
                out.push_str(&body[at_start..=semi]);
                sc.seek(semi + 1);
            }
            BraceScan::Eof => {
                out.push_str(&body[at_start..sc.len()]);
                let _ = prelude_start;
                sc.seek(sc.len());
            }
        }
    } else if is_keyframes_keyword(name) {
        walk_keyframes(body, sc, at_start, ctx, out);
    } else {
        // @font-face / @page / @import / @charset / unknown: pass through.
        // Their body (if any) is a declaration block with no selectors.
        match scan_to_brace_or_semi(sc) {
            BraceScan::Brace(open) => {
                out.push_str(&body[at_start..=open]);
                let block_start = open + 1;
                sc.seek(block_start);
                match balanced_block_end(sc) {
                    Some(close) => {
                        // Rewrite declarations (for animation refs) but do NOT
                        // scope selectors.
                        let inner = &body[block_start..close];
                        rewrite_declarations(inner, ctx, out);
                        out.push('}');
                        sc.seek(close + 1);
                    }
                    None => {
                        let inner = &body[block_start..sc.len()];
                        ctx.diag(block_start, sc.len(), "unterminated at-rule block");
                        out.push_str(inner);
                        sc.seek(sc.len());
                    }
                }
            }
            BraceScan::Semi(semi) => {
                out.push_str(&body[at_start..=semi]);
                sc.seek(semi + 1);
            }
            BraceScan::Eof => {
                out.push_str(&body[at_start..sc.len()]);
                sc.seek(sc.len());
            }
        }
    }
}

/// Rewrites `@keyframes name { … }` → `@keyframes name-<suffix> { … }`. The
/// keyframe selectors inside (`from`, `to`, `0%`…) are NOT scoped.
fn walk_keyframes(body: &str, sc: &mut Scanner, at_start: usize, ctx: &mut Ctx, out: &mut String) {
    // Emit `@keyframes` (and any vendor prefix) verbatim up to the name.
    let after_kw = sc.pos();
    out.push_str(&body[at_start..after_kw]);
    skip_ws_and_comments_emit(body, sc, out);
    let name_start = sc.pos();
    read_ident(sc);
    let name = &body[name_start..sc.pos()];
    if let Some((_, scoped)) = ctx.keyframes.iter().find(|(k, _)| k == name) {
        out.push_str(scoped);
    } else {
        out.push_str(name);
    }
    // Emit the rest (the `{ … }` block) verbatim.
    let after_name = sc.pos();
    match scan_to_brace_or_semi(sc) {
        BraceScan::Brace(open) => {
            out.push_str(&body[after_name..=open]);
            let block_start = open + 1;
            sc.seek(block_start);
            match balanced_block_end(sc) {
                Some(close) => {
                    out.push_str(&body[block_start..close]);
                    out.push('}');
                    sc.seek(close + 1);
                }
                None => {
                    ctx.diag(block_start, sc.len(), "unterminated @keyframes block");
                    out.push_str(&body[block_start..sc.len()]);
                    sc.seek(sc.len());
                }
            }
        }
        BraceScan::Semi(semi) => {
            out.push_str(&body[after_name..=semi]);
            sc.seek(semi + 1);
        }
        BraceScan::Eof => {
            out.push_str(&body[after_name..sc.len()]);
            sc.seek(sc.len());
        }
    }
}

/// Scopes a raw selector-list text (the prelude before `{`), preserving the
/// original whitespace around commas as far as reasonable.
fn scope_selector_list_text(list: &str, scope_attr: &str) -> String {
    let parts = split_selector_list(list);
    let mut out = String::with_capacity(list.len() + scope_attr.len() + 4);
    for (i, (s, e)) in parts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let sel = &list[*s..*e];
        // Preserve leading/trailing whitespace of the selector so formatting is
        // stable; scope only the trimmed core. A whitespace-only selector (e.g.
        // around a stray comma) passes through verbatim.
        if sel.trim().is_empty() {
            out.push_str(sel);
            continue;
        }
        let leading_len = sel.len() - sel.trim_start().len();
        let trailing_len = sel.len() - sel.trim_end().len();
        let core = &sel[leading_len..sel.len() - trailing_len];
        out.push_str(&sel[..leading_len]);
        out.push_str(&scope_complex_selector(core, scope_attr));
        out.push_str(&sel[sel.len() - trailing_len..]);
    }
    out
}

// ---------------------------------------------------------------------------
// Low-level scanning helpers (structure only; no rewriting).
// ---------------------------------------------------------------------------

enum BraceScan {
    Brace(usize),
    Semi(usize),
    Eof,
}

/// Scans forward until the first top-level `{` or `;`, respecting strings,
/// comments, escapes, and paren/bracket nesting. Leaves the cursor *at* the
/// terminator.
fn scan_to_brace_or_semi(sc: &mut Scanner) -> BraceScan {
    while !sc.is_eof() {
        if consume_comment(sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b'(') => {
                consume_bracketed(sc, Bracket::Paren);
            }
            Some(b'[') => {
                consume_bracketed(sc, Bracket::Square);
            }
            Some(b'{') => {
                let p = sc.pos();
                return BraceScan::Brace(p);
            }
            Some(b';') => {
                let p = sc.pos();
                return BraceScan::Semi(p);
            }
            _ => {
                sc.bump();
            }
        }
    }
    BraceScan::Eof
}

/// Given a cursor just after an opening `{`, returns the byte position of the
/// matching `}`, respecting nesting/strings/comments. Leaves the cursor *at*
/// the closing brace on success, or at EOF if unbalanced. Returns `None` if
/// unbalanced.
fn balanced_block_end(sc: &mut Scanner) -> Option<usize> {
    let mut depth = 1usize;
    while !sc.is_eof() {
        if consume_comment(sc) {
            continue;
        }
        match sc.peek() {
            Some(b'"' | b'\'') => {
                consume_string(sc);
            }
            Some(b'\\') => {
                sc.bump();
                sc.bump();
            }
            Some(b'(') => {
                consume_bracketed(sc, Bracket::Paren);
            }
            Some(b'[') => {
                consume_bracketed(sc, Bracket::Square);
            }
            Some(b'{') => {
                depth += 1;
                sc.bump();
            }
            Some(b'}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(sc.pos());
                }
                sc.bump();
            }
            _ => {
                sc.bump();
            }
        }
    }
    None
}

/// Consumes a balanced `{ … }` group starting at the cursor (used only as a
/// bail-out inside declaration parsing).
fn consume_bracketed_curly(sc: &mut Scanner) {
    if sc.peek() != Some(b'{') {
        return;
    }
    sc.bump();
    let _ = balanced_block_end(sc);
    if sc.peek() == Some(b'}') {
        sc.bump();
    }
}

/// Skips one inert construct (whitespace run or comment). Returns `true` if it
/// advanced.
fn skip_inert(sc: &mut Scanner) -> bool {
    if consume_comment(sc) {
        return true;
    }
    if matches!(sc.peek(), Some(b' ' | b'\t' | b'\n' | b'\r') | Some(0x0c)) {
        while matches!(sc.peek(), Some(b' ' | b'\t' | b'\n' | b'\r') | Some(0x0c)) {
            sc.bump();
        }
        return true;
    }
    false
}

fn skip_ws_and_comments(sc: &mut Scanner) {
    while skip_inert(sc) {}
}

fn skip_ws_and_comments_emit(body: &str, sc: &mut Scanner, out: &mut String) {
    let start = sc.pos();
    while skip_inert(sc) {}
    out.push_str(&body[start..sc.pos()]);
}

/// Reads an at-keyword `@name`, returning the name without the `@`. The cursor
/// starts on `@` and ends just after the keyword.
fn read_at_keyword<'a>(sc: &mut Scanner<'a>) -> &'a str {
    debug_assert_eq!(sc.peek(), Some(b'@'));
    sc.bump(); // '@'
    let start = sc.pos();
    // Vendor prefixes and names: [-a-zA-Z0-9].
    while matches!(sc.peek(), Some(b) if is_ident_byte(b) || b == b'-') {
        sc.bump();
    }
    &sc.src()[start..sc.pos()]
}

/// Reads a CSS identifier at the cursor (used for keyframe/animation names).
fn read_ident(sc: &mut Scanner) {
    while let Some(b) = sc.peek() {
        if b == b'\\' {
            sc.bump();
            sc.bump();
        } else if is_ident_byte(b) || b == b'-' || b >= 0x80 {
            sc.bump();
        } else {
            break;
        }
    }
}

// --- pass-one skip helpers (structure only, no output) ---

fn skip_at_rule_tail(sc: &mut Scanner) {
    match scan_to_brace_or_semi(sc) {
        BraceScan::Brace(open) => {
            sc.seek(open + 1);
            if let Some(close) = balanced_block_end(sc) {
                sc.seek(close + 1);
            } else {
                sc.seek(sc.len());
            }
        }
        BraceScan::Semi(semi) => sc.seek(semi + 1),
        BraceScan::Eof => sc.seek(sc.len()),
    }
}

fn skip_prelude_and_block(sc: &mut Scanner) {
    match scan_to_brace_or_semi(sc) {
        BraceScan::Brace(open) => {
            sc.seek(open + 1);
            if let Some(close) = balanced_block_end(sc) {
                sc.seek(close + 1);
            } else {
                sc.seek(sc.len());
            }
        }
        BraceScan::Semi(semi) => sc.seek(semi + 1),
        BraceScan::Eof => sc.seek(sc.len()),
    }
}

fn skip_to_block_and_over(sc: &mut Scanner) {
    match scan_to_brace_or_semi(sc) {
        BraceScan::Brace(open) => {
            sc.seek(open + 1);
            if let Some(close) = balanced_block_end(sc) {
                sc.seek(close + 1);
            } else {
                sc.seek(sc.len());
            }
        }
        BraceScan::Semi(semi) => sc.seek(semi + 1),
        BraceScan::Eof => sc.seek(sc.len()),
    }
}

fn is_conditional_group(name: &str) -> bool {
    let n = strip_vendor(name);
    n.eq_ignore_ascii_case("media")
        || n.eq_ignore_ascii_case("supports")
        || n.eq_ignore_ascii_case("layer")
        || n.eq_ignore_ascii_case("container")
        || n.eq_ignore_ascii_case("scope")
}

fn is_keyframes_keyword(name: &str) -> bool {
    strip_vendor(name).eq_ignore_ascii_case("keyframes")
}

/// Strips a leading vendor prefix like `-webkit-` from an at-keyword name.
fn strip_vendor(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix('-') {
        if let Some(dash) = rest.find('-') {
            return &rest[dash + 1..];
        }
    }
    name
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    }
}

/// The suffix appended to keyframe names. Derived from the scope attribute so
/// two components with different scopes get distinct animation names. We reuse
/// the hash embedded in the attribute (`data-lunas-<hash>` → `<hash>`); if the
/// attribute has no recognizable hash we fall back to the whole attribute with
/// non-ident bytes stripped.
fn keyframe_suffix(scope_attr: &str) -> String {
    let tail = scope_attr.rsplit('-').next().unwrap_or(scope_attr);
    let cleaned: String = tail
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "lunas".to_string()
    } else {
        cleaned
    }
}
