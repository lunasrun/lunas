use serde::{Deserialize, Serialize};

/// A byte offset into source text.
///
/// Modeled after the `text-size` crate used by rust-analyzer: offsets are byte
/// offsets (not char offsets), stored as `u32` because source files are never
/// expected to exceed 4 GiB.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TextSize(u32);

impl TextSize {
    pub const fn new(raw: u32) -> Self {
        TextSize(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for TextSize {
    fn from(value: u32) -> Self {
        TextSize(value)
    }
}

impl TryFrom<usize> for TextSize {
    type Error = std::num::TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(TextSize(u32::try_from(value)?))
    }
}

impl std::ops::Add for TextSize {
    type Output = TextSize;

    fn add(self, rhs: TextSize) -> TextSize {
        TextSize(self.0 + rhs.0)
    }
}

impl std::ops::Sub for TextSize {
    type Output = TextSize;

    fn sub(self, rhs: TextSize) -> TextSize {
        TextSize(self.0 - rhs.0)
    }
}

/// A half-open byte range `[start, end)` into source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    /// Creates a range. `start` must not exceed `end`.
    pub fn new(start: TextSize, end: TextSize) -> Self {
        debug_assert!(start <= end, "TextRange start must not exceed end");
        TextRange { start, end }
    }

    /// Creates an empty range at `offset`.
    pub fn empty(offset: TextSize) -> Self {
        TextRange {
            start: offset,
            end: offset,
        }
    }

    /// Convenience constructor from raw `u32` offsets.
    pub fn at(start: u32, end: u32) -> Self {
        TextRange::new(TextSize::new(start), TextSize::new(end))
    }

    pub fn start(self) -> TextSize {
        self.start
    }

    pub fn end(self) -> TextSize {
        self.end
    }

    pub fn len(self) -> TextSize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn contains(self, offset: TextSize) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn contains_inclusive(self, offset: TextSize) -> bool {
        self.start <= offset && offset <= self.end
    }

    /// Returns the smallest range covering both `self` and `other`.
    pub fn cover(self, other: TextRange) -> TextRange {
        TextRange {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Slices `text` by this range. Returns `None` if the range is out of
    /// bounds or does not fall on UTF-8 char boundaries.
    pub fn slice(self, text: &str) -> Option<&str> {
        text.get(self.start.as_usize()..self.end.as_usize())
    }

    /// Returns this range translated forward by `by` bytes. Used to rebase a
    /// range parsed against a substring back onto the enclosing source.
    pub fn shifted(self, by: TextSize) -> TextRange {
        TextRange {
            start: self.start + by,
            end: self.end + by,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_size_arithmetic() {
        let a = TextSize::new(3);
        let b = TextSize::new(5);
        assert_eq!((a + b).raw(), 8);
        assert_eq!((b - a).raw(), 2);
        assert_eq!(a.as_usize(), 3);
    }

    #[test]
    fn try_from_usize_within_range() {
        assert_eq!(TextSize::try_from(42usize).unwrap(), TextSize::new(42));
    }

    #[test]
    fn try_from_usize_overflow() {
        let too_big = (u32::MAX as usize) + 1;
        assert!(TextSize::try_from(too_big).is_err());
    }

    #[test]
    fn range_len_and_empty() {
        let r = TextRange::at(4, 9);
        assert_eq!(r.len().raw(), 5);
        assert!(!r.is_empty());
        assert!(TextRange::empty(TextSize::new(7)).is_empty());
    }

    #[test]
    fn range_contains_is_half_open() {
        let r = TextRange::at(2, 5);
        assert!(!r.contains(TextSize::new(1)));
        assert!(r.contains(TextSize::new(2)));
        assert!(r.contains(TextSize::new(4)));
        assert!(!r.contains(TextSize::new(5)));
        assert!(r.contains_inclusive(TextSize::new(5)));
    }

    #[test]
    fn range_cover() {
        let a = TextRange::at(2, 5);
        let b = TextRange::at(7, 9);
        assert_eq!(a.cover(b), TextRange::at(2, 9));
        assert_eq!(b.cover(a), TextRange::at(2, 9));
    }

    #[test]
    fn range_shifted() {
        assert_eq!(
            TextRange::at(2, 5).shifted(TextSize::new(10)),
            TextRange::at(12, 15)
        );
        assert_eq!(
            TextRange::at(0, 3).shifted(TextSize::new(0)),
            TextRange::at(0, 3)
        );
    }

    #[test]
    fn range_slice_ascii() {
        let text = "hello world";
        assert_eq!(TextRange::at(0, 5).slice(text), Some("hello"));
        assert_eq!(TextRange::at(6, 11).slice(text), Some("world"));
    }

    #[test]
    fn range_slice_out_of_bounds() {
        let text = "abc";
        assert_eq!(TextRange::at(0, 99).slice(text), None);
    }

    #[test]
    fn range_slice_non_char_boundary() {
        // "あ" is 3 bytes; slicing mid-codepoint must fail gracefully.
        let text = "あ";
        assert_eq!(TextRange::at(0, 1).slice(text), None);
        assert_eq!(TextRange::at(0, 3).slice(text), Some("あ"));
    }

    #[test]
    fn serde_roundtrip() {
        let r = TextRange::at(3, 8);
        let json = serde_json::to_string(&r).unwrap();
        let back: TextRange = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
