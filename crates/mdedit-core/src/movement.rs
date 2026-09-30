use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    Affinity, Anchor, DocumentSnapshot, SelectionError, SelectionRange, SelectionSet, TextRangeError,
    TextSize,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    GraphemeBackward,
    GraphemeForward,
    WordBackward,
    WordForward,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
}

impl Movement {
    const fn affinity(self) -> Affinity {
        match self {
            Self::GraphemeBackward
            | Self::WordBackward
            | Self::LineStart
            | Self::DocumentStart => Affinity::Before,
            Self::GraphemeForward
            | Self::WordForward
            | Self::LineEnd
            | Self::DocumentEnd => Affinity::After,
        }
    }
}

pub fn move_anchor(
    snapshot: &DocumentSnapshot,
    anchor: Anchor,
    movement: Movement,
) -> Result<Anchor, MovementError> {
    let text = snapshot.text();
    let len = TextSize::try_from_usize(text.len())?;
    if anchor.offset > len {
        return Err(MovementError::OutOfBounds {
            offset: anchor.offset,
            document_len: len,
        });
    }

    let offset = anchor.offset.to_usize();
    let next = match movement {
        Movement::GraphemeBackward => previous_grapheme_boundary(&text, offset),
        Movement::GraphemeForward => next_grapheme_boundary(&text, offset),
        Movement::WordBackward => previous_word_start(&text, offset),
        Movement::WordForward => next_word_end(&text, offset),
        Movement::LineStart => line_start(&text, offset),
        Movement::LineEnd => line_end(&text, offset),
        Movement::DocumentStart => 0,
        Movement::DocumentEnd => text.len(),
    };

    Ok(Anchor::new(
        TextSize::try_from_usize(next)?,
        movement.affinity(),
    ))
}

/// Moves every selection head using source/logical coordinates.
///
/// When `extend` is true the anchor is preserved. Otherwise each range becomes a
/// caret at the moved head. Visual left/right/up/down movement is intentionally not
/// implemented here because it depends on shaped layout and BiDi information.
pub fn move_selection_heads(
    snapshot: &DocumentSnapshot,
    selections: &SelectionSet,
    movement: Movement,
    extend: bool,
) -> Result<SelectionSet, MovementError> {
    let mut ranges = Vec::with_capacity(selections.len());

    for range in selections.ranges() {
        let moved = move_anchor(snapshot, range.head, movement)?;
        ranges.push(if extend {
            SelectionRange {
                anchor: range.anchor,
                head: moved,
            }
        } else {
            SelectionRange::caret(moved)
        });
    }

    Ok(SelectionSet::new(ranges, selections.primary_index())?)
}

fn previous_grapheme_boundary(text: &str, offset: usize) -> usize {
    let mut previous = 0;
    for (start, _) in text.grapheme_indices(true) {
        if start >= offset {
            break;
        }
        previous = start;
    }
    previous
}

fn next_grapheme_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(start, _)| start)
        .find(|start| *start > offset)
        .unwrap_or(text.len())
}

fn previous_word_start(text: &str, offset: usize) -> usize {
    let mut previous = 0;

    for (start, word) in text.unicode_word_indices() {
        let end = start + word.len();
        if start < offset && offset <= end {
            return start;
        }
        if start >= offset {
            break;
        }
        previous = start;
    }

    previous
}

fn next_word_end(text: &str, offset: usize) -> usize {
    for (start, word) in text.unicode_word_indices() {
        let end = start + word.len();
        if start <= offset && offset < end {
            return end;
        }
        if start >= offset {
            return end;
        }
    }

    text.len()
}

fn line_start(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |newline| newline + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = bytes[offset..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |relative| offset + relative);

    if end > 0 && bytes[end - 1] == b'\r' {
        end -= 1;
    }

    end
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MovementError {
    #[error(transparent)]
    TextRange(#[from] TextRangeError),

    #[error(transparent)]
    Selection(#[from] SelectionError),

    #[error("anchor {offset:?} is outside document length {document_len:?}")]
    OutOfBounds {
        offset: TextSize,
        document_len: TextSize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    fn at(offset: usize) -> Anchor {
        Anchor::new(TextSize::try_from_usize(offset).unwrap(), Affinity::After)
    }

    #[test]
    fn grapheme_movement_does_not_split_family_emoji() {
        let document = Document::new("A👨‍👩‍👧‍👦B").unwrap();
        let snapshot = document.snapshot();

        let after_a = 1;
        let after_emoji = 1 + "👨‍👩‍👧‍👦".len();

        assert_eq!(
            move_anchor(&snapshot, at(after_a), Movement::GraphemeForward)
                .unwrap()
                .offset
                .to_usize(),
            after_emoji
        );
        assert_eq!(
            move_anchor(&snapshot, at(after_emoji), Movement::GraphemeBackward)
                .unwrap()
                .offset
                .to_usize(),
            after_a
        );
    }

    #[test]
    fn grapheme_movement_does_not_split_combining_sequence() {
        let text = "e\u{301}x";
        let document = Document::new(text).unwrap();
        let snapshot = document.snapshot();

        assert_eq!(
            move_anchor(&snapshot, at(0), Movement::GraphemeForward)
                .unwrap()
                .offset
                .to_usize(),
            "e\u{301}".len()
        );
    }

    #[test]
    fn word_movement_handles_unicode_words() {
        let text = "hello 한글 world";
        let document = Document::new(text).unwrap();
        let snapshot = document.snapshot();
        let korean_start = "hello ".len();
        let korean_end = korean_start + "한글".len();

        assert_eq!(
            move_anchor(&snapshot, at(korean_start), Movement::WordForward)
                .unwrap()
                .offset
                .to_usize(),
            korean_end
        );
        assert_eq!(
            move_anchor(&snapshot, at(korean_end), Movement::WordBackward)
                .unwrap()
                .offset
                .to_usize(),
            korean_start
        );
    }

    #[test]
    fn line_boundaries_exclude_crlf() {
        let document = Document::new("first\r\nsecond").unwrap();
        let snapshot = document.snapshot();

        assert_eq!(
            move_anchor(&snapshot, at(0), Movement::LineEnd)
                .unwrap()
                .offset
                .to_usize(),
            5
        );
        assert_eq!(
            move_anchor(&snapshot, at(7), Movement::LineStart)
                .unwrap()
                .offset
                .to_usize(),
            7
        );
    }
}
