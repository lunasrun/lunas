// diagnostics.mjs — map compiler diagnostics to Vite/Rollup error positions.
//
// The compiler reports spans as UTF-8 *byte* offsets into the source. Rollup's
// error `loc` wants 1-based line and 1-based column (in characters). We build a
// small line index over the source and convert.

// Convert a UTF-8 byte offset to a { line, column, character } position.
// `line` is 1-based, `column` is 1-based, both suitable for Rollup's `loc`.
function byteOffsetToPosition(source, byteOffset) {
  // Walk the string by code point, tracking the running UTF-8 byte length,
  // until we reach the target byte offset. This is O(n) per lookup but n is a
  // single component file, and there are only a handful of diagnostics.
  let bytes = 0;
  let line = 1;
  let column = 1;
  let character = 0;

  if (byteOffset <= 0) return { line, column, character };

  for (const ch of source) {
    if (bytes >= byteOffset) break;
    if (ch === "\n") {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
    // UTF-8 byte length of this code point.
    bytes += utf8Len(ch.codePointAt(0));
    character += 1;
  }
  return { line, column, character };
}

function utf8Len(codePoint) {
  if (codePoint <= 0x7f) return 1;
  if (codePoint <= 0x7ff) return 2;
  if (codePoint <= 0xffff) return 3;
  return 4;
}

// Format a single diagnostic into a Rollup-style error payload:
// `{ message, id, loc: { file, line, column }, frame }`.
export function toRollupError(diag, id, source) {
  const pos = byteOffsetToPosition(source, diag.start);
  const frame = codeFrame(source, pos.line, pos.column);
  return {
    message: diag.message,
    id,
    loc: { file: id, line: pos.line, column: pos.column },
    frame,
  };
}

// A one-line code frame: the offending source line plus a caret under the
// column. Kept dependency-free and intentionally minimal.
export function codeFrame(source, line, column) {
  const lines = source.split(/\r\n|\r|\n/);
  const text = lines[line - 1] != null ? lines[line - 1] : "";
  const gutter = String(line);
  const pad = " ".repeat(gutter.length);
  const caret = " ".repeat(Math.max(0, column - 1)) + "^";
  return `${gutter} | ${text}\n${pad} | ${caret}`;
}

export { byteOffsetToPosition };
