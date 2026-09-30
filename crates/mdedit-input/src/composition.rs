use std::ops::Range;

use mdedit_core::{Affinity, TextRange, TextSize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionState {
    replace_range: TextRange,
    preedit: String,
    selection: Option<Range<usize>>,
}

impl CompositionState {
    #[must_use]
    pub fn new(replace_range: TextRange) -> Self {
        Self {
            replace_range,
            preedit: String::new(),
            selection: None,
        }
    }

    #[must_use]
    pub const fn replace_range(&self) -> TextRange {
        self.replace_range
    }

    #[must_use]
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        self.selection.clone()
    }

    pub fn update(
        &mut self,
        preedit: String,
        selection: Option<Range<usize>>,
    ) -> Result<(), CompositionError> {
        if let Some(range) = &selection {
            if range.start > range.end || range.end > preedit.len() {
                return Err(CompositionError::SelectionOutOfBounds {
                    start: range.start,
                    end: range.end,
                    preedit_len: preedit.len(),
                });
            }
            if !preedit.is_char_boundary(range.start) || !preedit.is_char_boundary(range.end) {
                return Err(CompositionError::SelectionNotUtf8Boundary {
                    start: range.start,
                    end: range.end,
                });
            }
        }

        self.preedit = preedit;
        self.selection = selection;
        Ok(())
    }

    pub fn display_text(&self, source: &str) -> Result<String, CompositionError> {
        let start = self.replace_range.start().to_usize();
        let end = self.replace_range.end().to_usize();
        if end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            return Err(CompositionError::InvalidSourceRange {
                range: self.replace_range,
                source_len: source.len(),
            });
        }

        let mut display = String::with_capacity(
            source.len() - (end - start) + self.preedit.len(),
        );
        display.push_str(&source[..start]);
        display.push_str(&self.preedit);
        display.push_str(&source[end..]);
        Ok(display)
    }

    #[must_use]
    pub fn display_preedit_range(&self) -> Range<usize> {
        let start = self.replace_range.start().to_usize();
        start..start + self.preedit.len()
    }

    pub fn display_cursor_offset(&self) -> Result<TextSize, CompositionError> {
        let relative = self
            .selection
            .as_ref()
            .map_or(self.preedit.len(), |range| range.end);
        let absolute = self
            .replace_range
            .start()
            .to_usize()
            .checked_add(relative)
            .ok_or(CompositionError::PositionOverflow)?;
        TextSize::try_from_usize(absolute).map_err(|_| CompositionError::PositionOverflow)
    }

    pub fn source_to_display(
        &self,
        source_offset: TextSize,
        affinity: Affinity,
    ) -> Result<TextSize, CompositionError> {
        let start = self.replace_range.start();
        let end = self.replace_range.end();

        let mapped = if source_offset < start {
            source_offset.to_usize()
        } else if source_offset > end {
            source_offset
                .to_usize()
                .checked_sub(self.replace_range.len().to_usize())
                .and_then(|value| value.checked_add(self.preedit.len()))
                .ok_or(CompositionError::PositionOverflow)?
        } else {
            match affinity {
                Affinity::Before => start.to_usize(),
                Affinity::After => start
                    .to_usize()
                    .checked_add(self.preedit.len())
                    .ok_or(CompositionError::PositionOverflow)?,
            }
        };

        TextSize::try_from_usize(mapped).map_err(|_| CompositionError::PositionOverflow)
    }

    pub fn display_to_source(
        &self,
        display_offset: TextSize,
        affinity: Affinity,
    ) -> Result<TextSize, CompositionError> {
        let display = display_offset.to_usize();
        let start = self.replace_range.start().to_usize();
        let preedit_end = start
            .checked_add(self.preedit.len())
            .ok_or(CompositionError::PositionOverflow)?;

        let mapped = if display < start {
            display
        } else if display > preedit_end {
            display
                .checked_sub(self.preedit.len())
                .and_then(|value| value.checked_add(self.replace_range.len().to_usize()))
                .ok_or(CompositionError::PositionOverflow)?
        } else {
            match affinity {
                Affinity::Before => self.replace_range.start().to_usize(),
                Affinity::After => self.replace_range.end().to_usize(),
            }
        };

        TextSize::try_from_usize(mapped).map_err(|_| CompositionError::PositionOverflow)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CompositionError {
    #[error("preedit selection {start}..{end} exceeds preedit length {preedit_len}")]
    SelectionOutOfBounds {
        start: usize,
        end: usize,
        preedit_len: usize,
    },

    #[error("preedit selection {start}..{end} is not on UTF-8 boundaries")]
    SelectionNotUtf8Boundary { start: usize, end: usize },

    #[error("composition source range {range:?} is invalid for source length {source_len}")]
    InvalidSourceRange { range: TextRange, source_len: usize },

    #[error("composition position overflow")]
    PositionOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
    }

    #[test]
    fn composition_replaces_source_only_in_display_projection() {
        let mut composition = CompositionState::new(range(1, 2));
        composition
            .update("한".to_owned(), Some(("한".len(), "한".len())))
            .unwrap();

        assert_eq!(composition.display_text("abc").unwrap(), "a한c");
        assert_eq!(composition.display_preedit_range(), 1..4);
    }

    #[test]
    fn mapping_accounts_for_replaced_source_length() {
        let mut composition = CompositionState::new(range(1, 3));
        composition.update("한".to_owned(), None).unwrap();

        assert_eq!(
            composition
                .source_to_display(TextSize::new(4), Affinity::After)
                .unwrap(),
            TextSize::new(5)
        );
        assert_eq!(
            composition
                .display_to_source(TextSize::new(5), Affinity::After)
                .unwrap(),
            TextSize::new(4)
        );
    }
}
