use thiserror::Error;

use crate::{
    Affinity, Anchor, Change, ChangeMap, ChangeSet, DocumentSnapshot, Movement, MovementError,
    SelectionError, SelectionRange, SelectionSet, TextRange, TextRangeError, Transaction,
    TransactionError, TransactionKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteDirection {
    Backward,
    Forward,
}

/// Builds a deletion transaction without mutating the document.
///
/// Non-empty selections are deleted as-is. Carets delete one Unicode grapheme in
/// the requested direction. Overlapping delete ranges are merged before the
/// transaction is built so multi-cursor editing cannot generate overlapping changes.
pub fn deletion_transaction(
    snapshot: &DocumentSnapshot,
    selections: &SelectionSet,
    direction: DeleteDirection,
) -> Result<Option<Transaction>, EditError> {
    let mut delete_ranges = Vec::with_capacity(selections.len());
    let mut target_positions = Vec::with_capacity(selections.len());

    for range in selections.ranges() {
        let (start, end) = range.ordered_offsets();

        if start != end {
            delete_ranges.push(TextRange::new(start, end)?);
            target_positions.push(start);
            continue;
        }

        let movement = match direction {
            DeleteDirection::Backward => Movement::GraphemeBackward,
            DeleteDirection::Forward => Movement::GraphemeForward,
        };
        let moved = crate::move_anchor(snapshot, range.head, movement)?;
        let delete = match direction {
            DeleteDirection::Backward => TextRange::new(moved.offset, range.head.offset)?,
            DeleteDirection::Forward => TextRange::new(range.head.offset, moved.offset)?,
        };

        if !delete.is_empty() {
            delete_ranges.push(delete);
        }
        target_positions.push(delete.start());
    }

    if delete_ranges.is_empty() {
        return Ok(None);
    }

    let delete_ranges = merge_delete_ranges(delete_ranges)?;
    let changes = ChangeSet::new(
        delete_ranges
            .into_iter()
            .map(|range| Change::new(range, ""))
            .collect(),
    )?;
    let change_map = ChangeMap::from_change_set(&changes)?;

    let mapped_ranges = target_positions
        .into_iter()
        .map(|offset| {
            change_map
                .map_old_to_new(offset, Affinity::Before)
                .map(|mapped| {
                    SelectionRange::caret(Anchor::new(mapped.offset, Affinity::Before))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let selection_after = SelectionSet::new(mapped_ranges, selections.primary_index())?;

    Ok(Some(
        Transaction::new(snapshot.revision(), changes, TransactionKind::Delete)
            .with_selection(selection_after),
    ))
}

fn merge_delete_ranges(mut ranges: Vec<TextRange>) -> Result<Vec<TextRange>, TextRangeError> {
    ranges.sort_by_key(|range| range.start());
    let mut merged: Vec<TextRange> = Vec::with_capacity(ranges.len());

    for range in ranges {
        let Some(last) = merged.last_mut() else {
            merged.push(range);
            continue;
        };

        if range.start() <= last.end() {
            *last = TextRange::new(last.start(), last.end().max(range.end()))?;
        } else {
            merged.push(range);
        }
    }

    Ok(merged)
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EditError {
    #[error(transparent)]
    Movement(#[from] MovementError),

    #[error(transparent)]
    Selection(#[from] SelectionError),

    #[error(transparent)]
    TextRange(#[from] TextRangeError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, TextSize};

    fn caret(offset: usize) -> SelectionSet {
        SelectionSet::caret(Anchor::new(
            TextSize::try_from_usize(offset).unwrap(),
            Affinity::After,
        ))
    }

    #[test]
    fn backspace_deletes_whole_emoji_grapheme() {
        let mut document = Document::new("A👨‍👩‍👧‍👦B").unwrap();
        let offset = 1 + "👨‍👩‍👧‍👦".len();

        let tx = deletion_transaction(
            &document.snapshot(),
            &caret(offset),
            DeleteDirection::Backward,
        )
        .unwrap()
        .unwrap();

        document.apply(tx).unwrap();
        assert_eq!(document.text(), "AB");
    }

    #[test]
    fn backspace_deletes_crlf_as_one_grapheme() {
        let mut document = Document::new("a\r\nb").unwrap();

        let tx = deletion_transaction(
            &document.snapshot(),
            &caret(3),
            DeleteDirection::Backward,
        )
        .unwrap()
        .unwrap();

        document.apply(tx).unwrap();
        assert_eq!(document.text(), "ab");
    }

    #[test]
    fn deletion_of_selection_collapses_to_start() {
        let mut document = Document::new("abcdef").unwrap();
        let selection = SelectionSet::new(
            vec![SelectionRange {
                anchor: Anchor::new(TextSize::new(2), Affinity::After),
                head: Anchor::new(TextSize::new(5), Affinity::After),
            }],
            0,
        )
        .unwrap();

        let tx = deletion_transaction(
            &document.snapshot(),
            &selection,
            DeleteDirection::Backward,
        )
        .unwrap()
        .unwrap();
        let applied = document.apply(tx).unwrap();

        assert_eq!(document.text(), "abf");
        assert_eq!(
            applied.selection_after.unwrap().primary().head.offset,
            TextSize::new(2)
        );
    }
}
