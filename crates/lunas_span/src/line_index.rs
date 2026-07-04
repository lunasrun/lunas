use crate::text_size::TextSize;
use serde::{Deserialize, Serialize};

/// A 0-based line/column position.
///
/// `col` is a UTF-8 byte offset from the start of the line, matching the rest
/// of the span model. For UTF-16 columns (the LSP default), use
/// [`LineIndex::utf16_line_col`] / [`LineIndex::offset_utf16`]; this type stays
/// byte-oriented for internal consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineCol {
    pub const fn new(line: u32, col: u32) -> Self {
        LineCol { line, col }
    }
}

/// Maps byte offsets to `(line, col)` and back in O(log n).
///
/// Built once per source file. Stores the byte offset of the start of every
/// line so both directions are a binary search plus subtraction.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of the first character of each line. `line_starts[0]` is
    /// always 0; there is one entry per line.
    line_starts: Vec<TextSize>,
    /// Total length of the source in bytes, used to clamp out-of-range queries.
    len: TextSize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::new(0)];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                // The next line starts right after the newline.
                let next = TextSize::new(offset as u32 + 1);
                line_starts.push(next);
            }
        }
        LineIndex {
            line_starts,
            len: TextSize::new(text.len() as u32),
        }
    }

    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Converts a byte offset to a `LineCol`. Offsets past the end of the text
    /// are clamped to the final position.
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let offset = offset.min(self.len);
        // Find the last line whose start is <= offset.
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        let line_start = self.line_starts[line];
        LineCol {
            line: line as u32,
            col: (offset - line_start).raw(),
        }
    }

    /// Converts a `LineCol` back to a byte offset. Out-of-range lines clamp to
    /// the end; out-of-range columns clamp to the start of the next line (or
    /// end of file for the last line).
    pub fn offset(&self, pos: LineCol) -> TextSize {
        if pos.line as usize >= self.line_starts.len() {
            return self.len;
        }
        let line_start = self.line_starts[pos.line as usize];
        let line_end = self
            .line_starts
            .get(pos.line as usize + 1)
            .copied()
            .unwrap_or(self.len);
        let candidate = line_start + TextSize::new(pos.col);
        candidate.min(line_end)
    }

    /// Converts a byte `offset` into a `(line, utf16_col)` position, where
    /// `utf16_col` counts UTF-16 code units from the start of the line — the
    /// column convention LSP uses by default. `text` must be the same source
    /// the index was built from.
    pub fn utf16_line_col(&self, offset: TextSize, text: &str) -> LineCol {
        let lc = self.line_col(offset);
        let line_start = self.line_starts[lc.line as usize].as_usize();
        let end = offset.min(self.len).as_usize();
        let col16 = text
            .get(line_start..end)
            .unwrap_or("")
            .chars()
            .map(|c| c.len_utf16() as u32)
            .sum();
        LineCol::new(lc.line, col16)
    }

    /// Inverse of [`utf16_line_col`](Self::utf16_line_col): converts a
    /// `(line, utf16_col)` LSP position back to a byte offset. A column past the
    /// end of the line clamps to the line's end.
    pub fn offset_utf16(&self, line: u32, utf16_col: u32, text: &str) -> TextSize {
        if line as usize >= self.line_starts.len() {
            return self.len;
        }
        let line_start = self.line_starts[line as usize].as_usize();
        let line_end = self
            .line_starts
            .get(line as usize + 1)
            .map(|p| p.as_usize())
            .unwrap_or(self.len.as_usize());
        let mut units = 0u32;
        for (i, c) in text.get(line_start..line_end).unwrap_or("").char_indices() {
            if units >= utf16_col {
                return TextSize::new((line_start + i) as u32);
            }
            units += c.len_utf16() as u32;
        }
        TextSize::new(line_end as u32)
    }
}

impl TextSize {
    fn min(self, other: TextSize) -> TextSize {
        if self.raw() <= other.raw() {
            self
        } else {
            other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(n: u32) -> TextSize {
        TextSize::new(n)
    }

    #[test]
    fn utf16_columns_for_multibyte() {
        // "あ😀x": あ = 3 bytes / 1 UTF-16 unit; 😀 = 4 bytes / 2 UTF-16 units.
        let text = "あ😀x";
        let idx = LineIndex::new(text);
        // byte offset 0 -> col16 0; after あ (byte 3) -> 1; after 😀 (byte 7) -> 3.
        assert_eq!(idx.utf16_line_col(ts(0), text), LineCol::new(0, 0));
        assert_eq!(idx.utf16_line_col(ts(3), text), LineCol::new(0, 1));
        assert_eq!(idx.utf16_line_col(ts(7), text), LineCol::new(0, 3));
        // Inverse round-trips.
        assert_eq!(idx.offset_utf16(0, 0, text), ts(0));
        assert_eq!(idx.offset_utf16(0, 1, text), ts(3));
        assert_eq!(idx.offset_utf16(0, 3, text), ts(7));
        // Past-end clamps to line end.
        assert_eq!(idx.offset_utf16(0, 99, text), ts(text.len() as u32));
    }

    #[test]
    fn utf16_columns_multiline() {
        let text = "a\nか😀\nb";
        let idx = LineIndex::new(text);
        // On line 1, after か (1 unit) the 😀 starts at col16 1.
        let off = idx.offset_utf16(1, 1, text);
        assert_eq!(idx.utf16_line_col(off, text), LineCol::new(1, 1));
        assert_eq!(text[off.as_usize()..].chars().next(), Some('😀'));
    }

    #[test]
    fn crlf_line_breaks() {
        // "ab\r\ncd" — the \n at offset 3 ends line 0; line 1 starts at offset 4.
        let idx = LineIndex::new("ab\r\ncd");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_col(ts(2)), LineCol::new(0, 2)); // the \r (col on line 0)
        assert_eq!(idx.line_col(ts(4)), LineCol::new(1, 0)); // 'c'
        assert_eq!(idx.line_col(ts(5)), LineCol::new(1, 1)); // 'd'
                                                             // Round-trips for every offset.
        for off in 0..=6u32 {
            let lc = idx.line_col(ts(off));
            assert_eq!(idx.offset(lc), ts(off.min(6)));
        }
    }

    #[test]
    fn single_line_no_newline() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_col(ts(0)), LineCol::new(0, 0));
        assert_eq!(idx.line_col(ts(4)), LineCol::new(0, 4));
    }

    #[test]
    fn multi_line_offsets() {
        // "ab\ncd\nef"
        //  0123 4567 8
        let idx = LineIndex::new("ab\ncd\nef");
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.line_col(ts(0)), LineCol::new(0, 0));
        assert_eq!(idx.line_col(ts(2)), LineCol::new(0, 2)); // the '\n' itself
        assert_eq!(idx.line_col(ts(3)), LineCol::new(1, 0));
        assert_eq!(idx.line_col(ts(5)), LineCol::new(1, 2));
        assert_eq!(idx.line_col(ts(6)), LineCol::new(2, 0));
        assert_eq!(idx.line_col(ts(7)), LineCol::new(2, 1));
    }

    #[test]
    fn roundtrip_every_offset() {
        let text = "first\nsecond line\n\nlast";
        let idx = LineIndex::new(text);
        for off in 0..=text.len() as u32 {
            let lc = idx.line_col(ts(off));
            let back = idx.offset(lc);
            assert_eq!(back, ts(off), "roundtrip failed at offset {off}");
        }
    }

    #[test]
    fn empty_line_in_middle() {
        // line 1 is empty (two consecutive newlines)
        let idx = LineIndex::new("a\n\nb");
        assert_eq!(idx.line_col(ts(2)), LineCol::new(1, 0));
        assert_eq!(idx.offset(LineCol::new(1, 0)), ts(2));
    }

    #[test]
    fn trailing_newline_creates_empty_last_line() {
        let idx = LineIndex::new("a\n");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_col(ts(2)), LineCol::new(1, 0));
    }

    #[test]
    fn offset_past_end_clamps() {
        let idx = LineIndex::new("abc");
        assert_eq!(idx.line_col(ts(999)), LineCol::new(0, 3));
    }

    #[test]
    fn offset_for_out_of_range_line_clamps_to_end() {
        let idx = LineIndex::new("abc\ndef");
        assert_eq!(idx.offset(LineCol::new(99, 0)), ts(7));
    }

    #[test]
    fn offset_for_out_of_range_col_clamps_to_line_end() {
        let idx = LineIndex::new("abc\ndef");
        // column 99 on line 0 should clamp to start of line 1 (offset 4)
        assert_eq!(idx.offset(LineCol::new(0, 99)), ts(4));
        // column 99 on last line clamps to EOF
        assert_eq!(idx.offset(LineCol::new(1, 99)), ts(7));
    }

    #[test]
    fn empty_text() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_col(ts(0)), LineCol::new(0, 0));
        assert_eq!(idx.offset(LineCol::new(0, 0)), ts(0));
    }
}
