use core::fmt;
use core::ops::Range;

use thiserror::Error;

/// UTF-8 byte offset in a document.
///
/// The editor deliberately does not expose bare `usize` positions in its public API.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TextSize(u32);

impl TextSize {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }

    pub fn try_from_usize(value: usize) -> Result<Self, TextRangeError> {
        u32::try_from(value)
            .map(Self)
            .map_err(|_| TextRangeError::DocumentTooLarge)
    }
}

impl fmt::Debug for TextSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    pub fn new(start: TextSize, end: TextSize) -> Result<Self, TextRangeError> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(TextRangeError::Reversed { start, end })
        }
    }

    #[must_use]
    pub const fn empty(at: TextSize) -> Self {
        Self { start: at, end: at }
    }

    #[must_use]
    pub const fn start(self) -> TextSize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> TextSize {
        self.end
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.0 == self.end.0
    }

    #[must_use]
    pub const fn len(self) -> TextSize {
        TextSize(self.end.0 - self.start.0)
    }

    #[must_use]
    pub const fn contains(self, offset: TextSize) -> bool {
        self.start.0 <= offset.0 && offset.0 < self.end.0
    }

    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.start.0 < other.end.0 && other.start.0 < self.end.0
    }

    #[must_use]
    pub fn as_usize_range(self) -> Range<usize> {
        self.start.to_usize()..self.end.to_usize()
    }
}

impl fmt::Debug for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum TextRangeError {
    #[error("text range is reversed: {start:?}..{end:?}")]
    Reversed { start: TextSize, end: TextSize },

    #[error("document exceeds the supported 4 GiB UTF-8 source range")]
    DocumentTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_rejects_reversed_bounds() {
        let result = TextRange::new(TextSize::new(4), TextSize::new(3));
        assert!(matches!(result, Err(TextRangeError::Reversed { .. })));
    }
}
