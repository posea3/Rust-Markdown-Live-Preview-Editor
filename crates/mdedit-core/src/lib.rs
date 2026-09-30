#![forbid(unsafe_code)]

//! Framework-independent editing foundations.
//!
//! This crate intentionally contains no GUI, renderer, Markdown parser, filesystem,
//! or platform-window dependency. All source coordinates are UTF-8 byte offsets.

mod change;
mod document;
mod edit;
mod history;
mod movement;
mod selection;
mod text;

pub use change::{
    AppliedTransaction, Change, ChangeMap, ChangeSet, HistoryGroup, MappedPosition, Transaction,
    TransactionError, TransactionKind,
};
pub use document::{Document, DocumentError, DocumentSnapshot, Revision};
pub use edit::{deletion_transaction, DeleteDirection, EditError};
pub use history::{History, HistoryError};
pub use movement::{move_anchor, move_selection_heads, Movement, MovementError};
pub use selection::{
    Affinity, Anchor, SelectionError, SelectionRange, SelectionSet,
};
pub use text::{TextRange, TextRangeError, TextSize};
