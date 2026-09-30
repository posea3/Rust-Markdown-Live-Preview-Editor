use thiserror::Error;

use crate::{Document, HistoryGroup, SelectionSet, Transaction, TransactionError};

#[derive(Clone, Debug)]
struct HistoryStep {
    undo: Transaction,
    redo: Transaction,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    steps: Vec<HistoryStep>,
    group: HistoryGroup,
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
        let group = transaction.history_group;
        let redo = transaction.clone();
        let applied = document.apply(transaction)?;
        let selection_after = applied.selection_after.clone();
        let step = HistoryStep {
            undo: applied.inverse,
            redo,
        };

        let can_group = matches!(group, HistoryGroup::Explicit(_))
            && self
                .undo
                .last()
                .is_some_and(|entry| entry.group == group);

        if can_group {
            let entry = self.undo.last_mut().expect("checked above");
            entry.steps.push(step);
            entry.selection_after = selection_after.clone();
        } else {
            self.undo.push(HistoryEntry {
                steps: vec![step],
                group,
                selection_before,
                selection_after: selection_after.clone(),
            });
        }

        self.redo.clear();
        Ok(selection_after)
    }

    pub fn undo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<SelectionSet>, HistoryError> {
        let mut entry = self.undo.pop().ok_or(HistoryError::NothingToUndo)?;

        for step in entry.steps.iter_mut().rev() {
            step.undo.base_revision = document.revision();
            let applied = document.apply(step.undo.clone())?;
            step.redo = applied.inverse;
        }

        let selection = Some(entry.selection_before.clone());
        self.redo.push(entry);
        Ok(selection)
    }

    pub fn redo(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<SelectionSet>, HistoryError> {
        let mut entry = self.redo.pop().ok_or(HistoryError::NothingToRedo)?;

        for step in &mut entry.steps {
            step.redo.base_revision = document.revision();
            let applied = document.apply(step.redo.clone())?;
            step.undo = applied.inverse;
        }

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

    fn insert_transaction(document: &Document, offset: u32, text: &str) -> Transaction {
        Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(TextRange::empty(TextSize::new(offset)), text)),
            TransactionKind::Typing,
        )
    }

    #[test]
    fn undo_and_redo_round_trip() {
        let mut doc = Document::new("abc").unwrap();
        let before = SelectionSet::caret(Anchor::new(TextSize::new(3), Affinity::After));
        let tx = insert_transaction(&doc, 3, "d");

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

    #[test]
    fn explicit_history_group_undoes_multiple_steps_together() {
        let mut doc = Document::new("").unwrap();
        let selection = SelectionSet::default();
        let mut history = History::default();
        let group = HistoryGroup::Explicit(7);

        let first = insert_transaction(&doc, 0, "a").with_history_group(group);
        history
            .apply_and_record(&mut doc, first, selection.clone())
            .unwrap();

        let second = insert_transaction(&doc, 1, "b").with_history_group(group);
        history
            .apply_and_record(&mut doc, second, selection.clone())
            .unwrap();

        assert_eq!(doc.text(), "ab");
        history.undo(&mut doc).unwrap();
        assert_eq!(doc.text(), "");
        history.redo(&mut doc).unwrap();
        assert_eq!(doc.text(), "ab");
    }

    #[test]
    fn isolated_history_entries_undo_separately() {
        let mut doc = Document::new("").unwrap();
        let selection = SelectionSet::default();
        let mut history = History::default();

        let first = insert_transaction(&doc, 0, "a");
        history
            .apply_and_record(&mut doc, first, selection.clone())
            .unwrap();
        let second = insert_transaction(&doc, 1, "b");
        history
            .apply_and_record(&mut doc, second, selection)
            .unwrap();

        history.undo(&mut doc).unwrap();
        assert_eq!(doc.text(), "a");
    }
}
