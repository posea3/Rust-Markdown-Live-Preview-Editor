#![forbid(unsafe_code)]

//! Framework-independent editing foundations.
//!
//! This crate intentionally contains no GUI, renderer, Markdown parser, filesystem,
//! or platform-window dependency. All source coordinates are UTF-8 byte offsets.

mod change;
mod document;
mod history;
mod selection;
mod text;

pub use change::{
    AppliedTransaction, Change, ChangeMap, ChangeSet, MappedPosition, Transaction, TransactionError,
    TransactionKind,
};
pub use document::{Document, DocumentError, DocumentSnapshot, Revision};
pub use history::{History, HistoryError};
pub use selection::{Affinity, Anchor, SelectionRange, SelectionSet};
pub use text::{TextRange, TextRangeError, TextSize};
