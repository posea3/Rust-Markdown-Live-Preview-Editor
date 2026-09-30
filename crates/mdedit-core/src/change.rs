use std::sync::Arc;

use smallvec::SmallVec;
use thiserror::Error;

use crate::{Affinity, Revision, SelectionSet, TextRange, TextSize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub range: TextRange,
    pub insert: Arc<str>,
}

impl Change {
    #[must_use]
    pub fn new(range: TextRange, insert: impl Into<Arc<str>>) -> Self {
        Self {
            range,
            insert: insert.into(),
        }
    }

    pub(crate) fn inserted_len(&self) -> Result<TextSize, TransactionError> {
        TextSize::try_from_usize(self.insert.len()).map_err(|_| TransactionError::DocumentTooLarge)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeSet {
    changes: SmallVec<[Change; 1]>,
}

impl ChangeSet {
    pub fn new(mut changes: Vec<Change>) -> Result<Self, TransactionError> {
        changes.sort_by_key(|change| change.range.start());

        for pair in changes.windows(2) {
            if pair[0].range.end() > pair[1].range.start() {
                return Err(TransactionError::OverlappingChanges {
                    left: pair[0].range,
                    right: pair[1].range,
                });
            }
        }

        Ok(Self {
            changes: changes.into_iter().collect(),
        })
    }

    #[must_use]
    pub fn single(change: Change) -> Self {
        Self {
            changes: SmallVec::from_vec(vec![change]),
        }
    }

    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionKind {
    Typing,
    Delete,
    Paste,
    Cut,
    ImeCommit,
    Format,
    Command,
    Widget,
    Programmatic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub base_revision: Revision,
    pub changes: ChangeSet,
    pub selection: Option<SelectionSet>,
    pub kind: TransactionKind,
}

impl Transaction {
    #[must_use]
    pub fn new(base_revision: Revision, changes: ChangeSet, kind: TransactionKind) -> Self {
        Self {
            base_revision,
            changes,
            selection: None,
            kind,
        }
    }

    #[must_use]
    pub fn with_selection(mut self, selection: SelectionSet) -> Self {
        self.selection = Some(selection);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappedPosition {
    pub offset: TextSize,
    pub affinity: Affinity,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeMap {
    segments: Vec<MapSegment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MapSegment {
    old: TextRange,
    new: TextRange,
}

impl ChangeMap {
    pub(crate) fn from_changes(changes: &[Change]) -> Result<Self, TransactionError> {
        let mut segments = Vec::with_capacity(changes.len());
        let mut delta: i64 = 0;

        for change in changes {
            let inserted = i64::from(change.inserted_len()?.get());
            let removed = i64::from(change.range.len().get());
            let old_start = i64::from(change.range.start().get());
            let new_start = old_start + delta;
            let new_end = new_start + inserted;

            let new = TextRange::new(text_size_from_i64(new_start)?, text_size_from_i64(new_end)?)
                .map_err(|_| TransactionError::DocumentTooLarge)?;

            segments.push(MapSegment {
                old: change.range,
                new,
            });
            delta += inserted - removed;
        }

        Ok(Self { segments })
    }

    pub fn map_old_to_new(
        &self,
        offset: TextSize,
        affinity: Affinity,
    ) -> Result<MappedPosition, TransactionError> {
        let mut delta: i64 = 0;

        for segment in &self.segments {
            if offset < segment.old.start() {
                break;
            }

            if offset <= segment.old.end() {
                let mapped = match affinity {
                    Affinity::Before => segment.new.start(),
                    Affinity::After => segment.new.end(),
                };
                return Ok(MappedPosition {
                    offset: mapped,
                    affinity,
                });
            }

            delta += i64::from(segment.new.len().get()) - i64::from(segment.old.len().get());
        }

        Ok(MappedPosition {
            offset: text_size_from_i64(i64::from(offset.get()) + delta)?,
            affinity,
        })
    }

    pub fn map_new_to_old(
        &self,
        offset: TextSize,
        affinity: Affinity,
    ) -> Result<MappedPosition, TransactionError> {
        let mut delta: i64 = 0;

        for segment in &self.segments {
            if offset < segment.new.start() {
                break;
            }

            if offset <= segment.new.end() {
                let mapped = match affinity {
                    Affinity::Before => segment.old.start(),
                    Affinity::After => segment.old.end(),
                };
                return Ok(MappedPosition {
                    offset: mapped,
                    affinity,
                });
            }

            delta += i64::from(segment.old.len().get()) - i64::from(segment.new.len().get());
        }

        Ok(MappedPosition {
            offset: text_size_from_i64(i64::from(offset.get()) + delta)?,
            affinity,
        })
    }
}

fn text_size_from_i64(value: i64) -> Result<TextSize, TransactionError> {
    let value = u32::try_from(value).map_err(|_| TransactionError::DocumentTooLarge)?;
    Ok(TextSize::new(value))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedTransaction {
    pub old_revision: Revision,
    pub new_revision: Revision,
    pub change_map: ChangeMap,
    pub inverse: Transaction,
    pub selection_after: Option<SelectionSet>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TransactionError {
    #[error("transaction was based on revision {expected:?}, but document is at {actual:?}")]
    StaleRevision {
        expected: Revision,
        actual: Revision,
    },

    #[error("change range {range:?} is outside document length {document_len:?}")]
    OutOfBounds {
        range: TextRange,
        document_len: TextSize,
    },

    #[error("change range {range:?} is not on a UTF-8 character boundary")]
    InvalidUtf8Boundary { range: TextRange },

    #[error("changes overlap: {left:?} and {right:?}")]
    OverlappingChanges { left: TextRange, right: TextRange },

    #[error("document exceeds the supported source size")]
    DocumentTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
    }

    #[test]
    fn rejects_overlapping_changes() {
        let changes = vec![Change::new(range(1, 4), "x"), Change::new(range(3, 5), "y")];
        assert!(matches!(
            ChangeSet::new(changes),
            Err(TransactionError::OverlappingChanges { .. })
        ));
    }

    #[test]
    fn maps_positions_across_replacement() {
        let changes = vec![Change::new(range(2, 4), "XYZ")];
        let map = ChangeMap::from_changes(&changes).unwrap();

        assert_eq!(
            map.map_old_to_new(TextSize::new(6), Affinity::After)
                .unwrap()
                .offset,
            TextSize::new(7)
        );
        assert_eq!(
            map.map_old_to_new(TextSize::new(2), Affinity::Before)
                .unwrap()
                .offset,
            TextSize::new(2)
        );
        assert_eq!(
            map.map_old_to_new(TextSize::new(4), Affinity::After)
                .unwrap()
                .offset,
            TextSize::new(5)
        );
    }
}
