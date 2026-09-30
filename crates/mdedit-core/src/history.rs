use thiserror::Error;

use crate::{Document, SelectionSet, Transaction, TransactionError};

#[derive(Clone, Debug)]
struct HistoryEntry {
    undo: Transaction,
    redo: Transaction,
    selection_before: SelectionSet,
    selection_after: Option<SelectionSet>,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl History {
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn apply_and_record(
        &mut self,
        document: &mut Document,
        transaction: Transaction,
        selection_before: SelectionSet,
    ) -> Result<Option<SelectionSet>, HistoryError> {
        let redo = transaction.clone();
        let applied = document.apply(transaction)?;

        self.undo.push(HistoryEntry {
            undo: applied.inverse,
            redo,
            selection_before,
            selection_after: applied.selection_after.clone(),
        });
        self.redo.clear();
        Ok(applied.selection_after)
    }

    pub fn undo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<SelectionSet>, HistoryError> {
        let mut entry = self.undo.pop().ok_or(HistoryError::NothingToUndo)?;
        entry.undo.base_revision = document.revision();

        let applied = document.apply(entry.undo.clone())?;
        entry.redo = applied.inverse;
        let selection = Some(entry.selection_before.clone());
        self.redo.push(entry);
        Ok(selection)
    }

    pub fn redo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<SelectionSet>, HistoryError> {
        let mut entry = self.redo.pop().ok_or(HistoryError::NothingToRedo)?;
        entry.redo.base_revision = document.revision();

        let applied = document.apply(entry.redo.clone())?;
        entry.undo = applied.inverse;
        let selection = entry.selection_after.clone();
        self.undo.push(entry);
        Ok(selection)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HistoryError {
    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error("nothing to undo")]
    NothingToUndo,

    #[error("nothing to redo")]
    NothingToRedo,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Affinity, Anchor, Change, ChangeSet, TextRange, TextSize, TransactionKind,
    };

    #[test]
    fn undo_and_redo_round_trip() {
        let mut doc = Document::new("abc").unwrap();
        let before = SelectionSet::caret(Anchor::new(TextSize::new(3), Affinity::After));
        let range = TextRange::empty(TextSize::new(3));
        let tx = Transaction::new(
            doc.revision(),
            ChangeSet::single(Change::new(range, "d")),
            TransactionKind::Typing,
        );

        let mut history = History::default();
        history
            .apply_and_record(&mut doc, tx, before.clone())
            .unwrap();
        assert_eq!(doc.text(), "abcd");

        history.undo(&mut doc).unwrap();
        assert_eq!(doc.text(), "abc");

        history.redo(&mut doc).unwrap();
        assert_eq!(doc.text(), "abcd");
    }
}
