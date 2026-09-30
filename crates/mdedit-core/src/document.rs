use crop::Rope;
use thiserror::Error;

use crate::{
    AppliedTransaction, Change, ChangeMap, ChangeSet, TextRange, TextRangeError, TextSize,
    Transaction, TransactionError, TransactionKind,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    rope: Rope,
    revision: Revision,
}

impl Document {
    pub fn new(text: &str) -> Result<Self, DocumentError> {
        TextSize::try_from_usize(text.len())?;
        Ok(Self {
            rope: Rope::from(text),
            revision: Revision::initial(),
        })
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    pub fn len(&self) -> Result<TextSize, DocumentError> {
        Ok(TextSize::try_from_usize(self.rope.byte_len())?)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.byte_len() == 0
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn slice(&self, range: TextRange) -> Result<String, DocumentError> {
        self.validate_range(range)?;
        Ok(self.rope.byte_slice(range.as_usize_range()).to_string())
    }

    #[must_use]
    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            rope: self.rope.clone(),
            revision: self.revision,
        }
    }

    pub fn apply(
        &mut self,
        transaction: Transaction,
    ) -> Result<AppliedTransaction, TransactionError> {
        if transaction.base_revision != self.revision {
            return Err(TransactionError::StaleRevision {
                expected: transaction.base_revision,
                actual: self.revision,
            });
        }

        let document_len = TextSize::try_from_usize(self.rope.byte_len())
            .map_err(|_| TransactionError::DocumentTooLarge)?;

        for change in transaction.changes.changes() {
            if change.range.end() > document_len {
                return Err(TransactionError::OutOfBounds {
                    range: change.range,
                    document_len,
                });
            }
            if !self.is_char_boundary(change.range.start().to_usize())
                || !self.is_char_boundary(change.range.end().to_usize())
            {
                return Err(TransactionError::InvalidUtf8Boundary {
                    range: change.range,
                });
            }
        }

        let change_map = ChangeMap::from_changes(transaction.changes.changes())?;
        let old_revision = self.revision;

        let mut inverse_changes = Vec::with_capacity(transaction.changes.changes().len());
        for change in transaction.changes.changes() {
            let removed = self
                .rope
                .byte_slice(change.range.as_usize_range())
                .to_string();

            let new_start = change_map
                .map_old_to_new(change.range.start(), crate::Affinity::Before)?
                .offset;
            let inserted_len = change.inserted_len()?;
            let new_end = TextSize::new(
                new_start
                    .get()
                    .checked_add(inserted_len.get())
                    .ok_or(TransactionError::DocumentTooLarge)?,
            );
            let inverse_range = TextRange::new(new_start, new_end)
                .map_err(|_| TransactionError::DocumentTooLarge)?;
            inverse_changes.push(Change::new(inverse_range, removed));
        }

        for change in transaction.changes.changes().iter().rev() {
            self.rope.delete(change.range.as_usize_range());
            if !change.insert.is_empty() {
                self.rope
                    .insert(change.range.start().to_usize(), &change.insert);
            }
        }

        self.revision = self.revision.next();

        let inverse = Transaction {
            base_revision: self.revision,
            changes: ChangeSet::new(inverse_changes)?,
            selection: None,
            kind: TransactionKind::Programmatic,
        };

        Ok(AppliedTransaction {
            old_revision,
            new_revision: self.revision,
            change_map,
            inverse,
            selection_after: transaction.selection,
        })
    }

    fn validate_range(&self, range: TextRange) -> Result<(), DocumentError> {
        let len = self.len()?;
        if range.end() > len {
            return Err(DocumentError::OutOfBounds {
                range,
                document_len: len,
            });
        }
        if !self.is_char_boundary(range.start().to_usize())
            || !self.is_char_boundary(range.end().to_usize())
        {
            return Err(DocumentError::InvalidUtf8Boundary { range });
        }
        Ok(())
    }

    fn is_char_boundary(&self, byte: usize) -> bool {
        self.rope.is_char_boundary(byte)
    }
}

#[derive(Clone, Debug)]
pub struct DocumentSnapshot {
    rope: Rope,
    revision: Revision,
}

impl DocumentSnapshot {
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn len(&self) -> Result<TextSize, TextRangeError> {
        TextSize::try_from_usize(self.rope.byte_len())
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DocumentError {
    #[error(transparent)]
    TextRange(#[from] TextRangeError),

    #[error("range {range:?} is outside document length {document_len:?}")]
    OutOfBounds {
        range: TextRange,
        document_len: TextSize,
    },

    #[error("range {range:?} is not on a UTF-8 character boundary")]
    InvalidUtf8Boundary { range: TextRange },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Change, ChangeSet};

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
    }

    #[test]
    fn applies_replacement_and_builds_inverse() {
        let mut doc = Document::new("abcdef").unwrap();
        let tx = Transaction::new(
            doc.revision(),
            ChangeSet::single(Change::new(range(2, 4), "XYZ")),
            TransactionKind::Typing,
        );
        let applied = doc.apply(tx).unwrap();
        assert_eq!(doc.text(), "abXYZef");

        let inverse = applied.inverse;
        doc.apply(inverse).unwrap();
        assert_eq!(doc.text(), "abcdef");
    }

    #[test]
    fn applies_multiple_changes_in_source_coordinates() {
        let mut doc = Document::new("abcdefghij").unwrap();
        let changes = ChangeSet::new(vec![
            Change::new(range(1, 3), "X"),
            Change::new(range(7, 9), "YZ"),
        ])
        .unwrap();

        doc.apply(Transaction::new(
            doc.revision(),
            changes,
            TransactionKind::Programmatic,
        ))
        .unwrap();

        assert_eq!(doc.text(), "aXdefgYZj");
    }

    #[test]
    fn rejects_mid_codepoint_edit() {
        let mut doc = Document::new("한글").unwrap();
        let tx = Transaction::new(
            doc.revision(),
            ChangeSet::single(Change::new(range(1, 1), "x")),
            TransactionKind::Typing,
        );

        assert!(matches!(
            doc.apply(tx),
            Err(TransactionError::InvalidUtf8Boundary { .. })
        ));
    }
}
