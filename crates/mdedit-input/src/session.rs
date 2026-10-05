use mdedit_core::{
    Affinity, Anchor, Change, ChangeMap, ChangeSet, DeleteDirection, Document, DocumentError,
    EditError, History, HistoryError, Movement, MovementError, SelectionError, SelectionRange,
    SelectionSet, TextRange, TextRangeError, TextSize, Transaction, TransactionError,
    TransactionKind, deletion_transaction, move_selection_heads,
};
use thiserror::Error;

use crate::{CompositionError, CompositionState, EditorInput};

#[derive(Clone, Debug)]
pub struct EditorSession {
    document: Document,
    selections: SelectionSet,
    history: History,
    composition: Option<CompositionState>,
    ime_enabled: bool,
    focused: bool,
}

impl EditorSession {
    pub fn new(text: &str) -> Result<Self, SessionError> {
        Ok(Self {
            document: Document::new(text)?,
            selections: SelectionSet::default(),
            history: History::default(),
            composition: None,
            ime_enabled: false,
            focused: true,
        })
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    #[must_use]
    pub const fn selections(&self) -> &SelectionSet {
        &self.selections
    }

    #[must_use]
    pub const fn composition(&self) -> Option<&CompositionState> {
        self.composition.as_ref()
    }

    #[must_use]
    pub const fn ime_enabled(&self) -> bool {
        self.ime_enabled
    }

    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focused
    }

    pub fn display_text(&self) -> Result<String, SessionError> {
        let source = self.document.text();
        self.composition
            .as_ref()
            .map_or(Ok(source.clone()), |composition| {
                Ok(composition.display_text(&source)?)
            })
    }

    pub fn handle(&mut self, input: EditorInput) -> Result<bool, SessionError> {
        match input {
            EditorInput::InsertText(text) => self.insert_text(&text),
            EditorInput::WidgetReplace { range, text } => self.replace_widget_range(range, &text),
            EditorInput::ImeEnabled => {
                self.ime_enabled = true;
                Ok(true)
            }
            EditorInput::ImePreedit { text, selection } => self.ime_preedit(text, selection),
            EditorInput::ImeCommit(text) => self.ime_commit(&text),
            EditorInput::ImeDisabled => {
                self.ime_enabled = false;
                self.composition = None;
                Ok(true)
            }
            EditorInput::Move { movement, extend } => {
                self.cancel_composition();
                self.selections = move_selection_heads(
                    &self.document.snapshot(),
                    &self.selections,
                    movement,
                    extend,
                )?;
                Ok(true)
            }
            EditorInput::Delete(direction) => self.delete(direction),
            EditorInput::SetSelection(selection) => {
                self.cancel_composition();
                self.selections = selection;
                Ok(true)
            }
            EditorInput::Undo => self.undo(),
            EditorInput::Redo => self.redo(),
            EditorInput::Focused(focused) => {
                if !focused {
                    self.finish_composition_on_focus_loss()?;
                }
                self.focused = focused;
                Ok(true)
            }
        }
    }

    fn finish_composition_on_focus_loss(&mut self) -> Result<(), SessionError> {
        let Some(preedit) = self
            .composition
            .as_ref()
            .map(|composition| composition.preedit().to_owned())
        else {
            return Ok(());
        };

        if preedit.is_empty() {
            self.cancel_composition();
        } else {
            self.ime_commit(&preedit)?;
        }
        Ok(())
    }

    pub fn insert_text(&mut self, text: &str) -> Result<bool, SessionError> {
        self.cancel_composition();
        let transaction = replacement_transaction(
            &self.document,
            &self.selections,
            text,
            TransactionKind::Typing,
        )?;
        let before = self.selections.clone();
        if let Some(selection) =
            self.history
                .apply_and_record(&mut self.document, transaction, before)?
        {
            self.selections = selection;
        }
        Ok(true)
    }

    pub fn replace_widget_range(
        &mut self,
        range: TextRange,
        text: &str,
    ) -> Result<bool, SessionError> {
        self.cancel_composition();

        let changes = ChangeSet::single(Change::new(range, text));
        let change_map = ChangeMap::from_change_set(&changes)?;
        let mapped_ranges = self
            .selections
            .ranges()
            .iter()
            .map(|selection| {
                let anchor = change_map.map_old_to_new(
                    selection.anchor.offset,
                    selection.anchor.affinity,
                )?;
                let head =
                    change_map.map_old_to_new(selection.head.offset, selection.head.affinity)?;
                Ok(SelectionRange {
                    anchor: Anchor::new(anchor.offset, anchor.affinity),
                    head: Anchor::new(head.offset, head.affinity),
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        let selection_after =
            SelectionSet::new(mapped_ranges, self.selections.primary_index())?;

        let transaction = Transaction::new(
            self.document.revision(),
            changes,
            TransactionKind::Widget,
        )
        .with_selection(selection_after);
        let before = self.selections.clone();
        if let Some(selection) =
            self.history
                .apply_and_record(&mut self.document, transaction, before)?
        {
            self.selections = selection;
        }

        Ok(true)
    }

    pub fn ime_preedit(
        &mut self,
        text: String,
        selection: Option<std::ops::Range<usize>>,
    ) -> Result<bool, SessionError> {
        // winit/AppKit may emit an empty Preedit after a commit or while tearing
        // down an IME session. An empty preedit without an existing composition
        // is only a clearing signal; it must not create a phantom composition.
        if text.is_empty() && self.composition.is_none() {
            return Ok(false);
        }

        if self.composition.is_none() {
            let primary = self.selections.primary();
            let (start, end) = primary.ordered_offsets();
            self.selections = SelectionSet::new(vec![primary], 0)?;
            self.composition = Some(CompositionState::new(TextRange::new(start, end)?));
        }

        self.composition
            .as_mut()
            .expect("composition initialized above")
            .update(text, selection)?;
        Ok(true)
    }

    pub fn ime_commit(&mut self, text: &str) -> Result<bool, SessionError> {
        if let Some(composition) = self.composition.take() {
            let range = composition.replace_range();
            let insert_len =
                TextSize::try_from_usize(text.len()).map_err(|_| SessionError::PositionOverflow)?;
            let caret = TextSize::new(
                range
                    .start()
                    .get()
                    .checked_add(insert_len.get())
                    .ok_or(SessionError::PositionOverflow)?,
            );
            let selection_after = SelectionSet::caret(Anchor::new(caret, Affinity::After));
            let transaction = Transaction::new(
                self.document.revision(),
                ChangeSet::single(Change::new(range, text)),
                TransactionKind::ImeCommit,
            )
            .with_selection(selection_after);
            let before = self.selections.clone();
            if let Some(selection) =
                self.history
                    .apply_and_record(&mut self.document, transaction, before)?
            {
                self.selections = selection;
            }
            return Ok(true);
        }

        self.insert_text_with_kind(text, TransactionKind::ImeCommit)
    }

    pub fn delete(&mut self, direction: DeleteDirection) -> Result<bool, SessionError> {
        self.cancel_composition();
        let Some(transaction) =
            deletion_transaction(&self.document.snapshot(), &self.selections, direction)?
        else {
            return Ok(false);
        };

        let before = self.selections.clone();
        if let Some(selection) =
            self.history
                .apply_and_record(&mut self.document, transaction, before)?
        {
            self.selections = selection;
        }
        Ok(true)
    }

    pub fn set_selection(&mut self, selection: SelectionSet) {
        self.cancel_composition();
        self.selections = selection;
    }

    pub fn set_caret(&mut self, anchor: Anchor) {
        self.set_selection(SelectionSet::caret(anchor));
    }

    pub fn move_selection(
        &mut self,
        movement: Movement,
        extend: bool,
    ) -> Result<bool, SessionError> {
        self.handle(EditorInput::Move { movement, extend })
    }

    pub fn undo(&mut self) -> Result<bool, SessionError> {
        self.cancel_composition();
        if !self.history.can_undo() {
            return Ok(false);
        }
        if let Some(selection) = self.history.undo(&mut self.document)? {
            self.selections = selection;
        }
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, SessionError> {
        self.cancel_composition();
        if !self.history.can_redo() {
            return Ok(false);
        }
        if let Some(selection) = self.history.redo(&mut self.document)? {
            self.selections = selection;
        }
        Ok(true)
    }

    pub fn cancel_composition(&mut self) {
        self.composition = None;
    }

    fn insert_text_with_kind(
        &mut self,
        text: &str,
        kind: TransactionKind,
    ) -> Result<bool, SessionError> {
        let transaction = replacement_transaction(&self.document, &self.selections, text, kind)?;
        let before = self.selections.clone();
        if let Some(selection) =
            self.history
                .apply_and_record(&mut self.document, transaction, before)?
        {
            self.selections = selection;
        }
        Ok(true)
    }
}

fn replacement_transaction(
    document: &Document,
    selections: &SelectionSet,
    text: &str,
    kind: TransactionKind,
) -> Result<Transaction, SessionError> {
    let changes = ChangeSet::new(
        selections
            .ranges()
            .iter()
            .map(|selection| {
                let (start, end) = selection.ordered_offsets();
                Ok(Change::new(TextRange::new(start, end)?, text))
            })
            .collect::<Result<Vec<_>, TextRangeError>>()?,
    )?;

    let change_map = ChangeMap::from_change_set(&changes)?;
    let insert_len =
        TextSize::try_from_usize(text.len()).map_err(|_| SessionError::PositionOverflow)?;

    let mapped_ranges = selections
        .ranges()
        .iter()
        .map(|selection| {
            let (start, _) = selection.ordered_offsets();
            let new_start = change_map.map_old_to_new(start, Affinity::Before)?.offset;
            let caret = TextSize::new(
                new_start
                    .get()
                    .checked_add(insert_len.get())
                    .ok_or(SessionError::PositionOverflow)?,
            );
            Ok(SelectionRange::caret(Anchor::new(caret, Affinity::After)))
        })
        .collect::<Result<Vec<_>, SessionError>>()?;

    let selection_after = SelectionSet::new(mapped_ranges, selections.primary_index())?;

    Ok(Transaction::new(document.revision(), changes, kind).with_selection(selection_after))
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error(transparent)]
    Document(#[from] DocumentError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error(transparent)]
    History(#[from] HistoryError),

    #[error(transparent)]
    Movement(#[from] MovementError),

    #[error(transparent)]
    Edit(#[from] EditError),

    #[error(transparent)]
    Selection(#[from] SelectionError),

    #[error(transparent)]
    TextRange(#[from] TextRangeError),

    #[error(transparent)]
    Composition(#[from] CompositionError),

    #[error("editor position overflow")]
    PositionOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdedit_core::TextSize;

    #[test]
    fn preedit_does_not_mutate_canonical_document() {
        let mut session = EditorSession::new("abc").unwrap();
        session.set_caret(Anchor::new(TextSize::new(1), Affinity::After));

        session
            .ime_preedit("한".to_owned(), Some("한".len().."한".len()))
            .unwrap();

        assert_eq!(session.document().text(), "abc");
        assert_eq!(session.display_text().unwrap(), "a한bc");
    }

    #[test]
    fn stray_empty_preedit_does_not_create_phantom_composition() {
        let mut session = EditorSession::new("abc").unwrap();

        let changed = session.ime_preedit(String::new(), None).unwrap();

        assert!(!changed);
        assert!(session.composition().is_none());
        assert_eq!(session.display_text().unwrap(), "abc");
    }

    #[test]
    fn empty_preedit_after_commit_does_not_reopen_composition() {
        let mut session = EditorSession::new("").unwrap();
        session.ime_preedit("ㅎ".to_owned(), Some(3..3)).unwrap();
        session.ime_preedit(String::new(), None).unwrap();
        session.ime_commit("한").unwrap();

        assert!(session.composition().is_none());
        assert_eq!(session.document().text(), "한");

        let changed = session.ime_preedit(String::new(), None).unwrap();

        assert!(!changed);
        assert!(session.composition().is_none());
        assert_eq!(session.document().text(), "한");
    }

    #[test]
    fn empty_preedit_before_commit_preserves_replace_range() {
        let mut session = EditorSession::new("abc").unwrap();
        let selection = SelectionSet::new(
            vec![SelectionRange {
                anchor: Anchor::new(TextSize::new(1), Affinity::Before),
                head: Anchor::new(TextSize::new(2), Affinity::After),
            }],
            0,
        )
        .unwrap();
        session.set_selection(selection);

        session.ime_preedit("ㅎ".to_owned(), Some(3..3)).unwrap();
        session.ime_preedit(String::new(), None).unwrap();
        session.ime_commit("한").unwrap();

        assert_eq!(session.document().text(), "a한c");
        assert_eq!(session.selections().primary().head.offset, TextSize::new(4));
    }

    #[test]
    fn ime_commit_is_one_undoable_change() {
        let mut session = EditorSession::new("").unwrap();
        session.ime_preedit("ㅎ".to_owned(), Some(3..3)).unwrap();
        session.ime_preedit("하".to_owned(), Some(3..3)).unwrap();
        session.ime_preedit("한".to_owned(), Some(3..3)).unwrap();
        session.ime_preedit(String::new(), None).unwrap();
        session.ime_commit("한").unwrap();

        assert_eq!(session.document().text(), "한");
        session.undo().unwrap();
        assert_eq!(session.document().text(), "");
    }

    #[test]
    fn regular_text_replaces_selection_and_collapses_to_end() {
        let mut session = EditorSession::new("abcd").unwrap();
        let selection = SelectionSet::new(
            vec![SelectionRange {
                anchor: Anchor::new(TextSize::new(1), Affinity::Before),
                head: Anchor::new(TextSize::new(3), Affinity::After),
            }],
            0,
        )
        .unwrap();
        session.set_selection(selection);
        session.insert_text("X").unwrap();

        assert_eq!(session.document().text(), "aXd");
        assert_eq!(session.selections().primary().head.offset, TextSize::new(2));
    }

    #[test]
    fn widget_replace_is_undoable_and_preserves_unrelated_caret() {
        let mut session = EditorSession::new("- [ ] task").unwrap();
        session.set_caret(Anchor::new(TextSize::new(10), Affinity::After));
        let marker = TextRange::new(TextSize::new(2), TextSize::new(5)).unwrap();

        session
            .handle(EditorInput::WidgetReplace {
                range: marker,
                text: "[x]".to_owned(),
            })
            .unwrap();

        assert_eq!(session.document().text(), "- [x] task");
        assert_eq!(
            session.selections().primary().head,
            Anchor::new(TextSize::new(10), Affinity::After)
        );

        session.undo().unwrap();
        assert_eq!(session.document().text(), "- [ ] task");
        session.redo().unwrap();
        assert_eq!(session.document().text(), "- [x] task");
    }

    #[test]
    fn focus_loss_finalizes_visible_preedit() {
        let mut session = EditorSession::new("abc").unwrap();
        session.set_caret(Anchor::new(TextSize::new(1), Affinity::After));
        session.ime_preedit("한".to_owned(), Some(3..3)).unwrap();

        session.handle(EditorInput::Focused(false)).unwrap();

        assert_eq!(session.document().text(), "a한bc");
        assert!(session.composition().is_none());
        assert!(!session.focused());
        assert_eq!(session.display_text().unwrap(), "a한bc");
    }

    #[test]
    fn ime_disabled_discards_preedit_without_mutating_source() {
        let mut session = EditorSession::new("abc").unwrap();
        session.set_caret(Anchor::new(TextSize::new(1), Affinity::After));
        session.handle(EditorInput::ImeEnabled).unwrap();
        session
            .handle(EditorInput::ImePreedit {
                text: "한".to_owned(),
                selection: Some(3..3),
            })
            .unwrap();

        session.handle(EditorInput::ImeDisabled).unwrap();

        assert_eq!(session.document().text(), "abc");
        assert!(session.composition().is_none());
        assert!(!session.ime_enabled());
        assert_eq!(session.display_text().unwrap(), "abc");
    }
}
