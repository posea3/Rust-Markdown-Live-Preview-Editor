#![forbid(unsafe_code)]

//! Framework-independent input/session state for the native Markdown editor.
//!
//! This crate owns transient IME composition and converts semantic editing
//! actions into `mdedit-core` transactions. It intentionally contains no
//! window-system or GPU types.

mod composition;
mod event;
mod session;
mod trace;

pub use composition::{CompositionError, CompositionState};
pub use event::{EditorInput, PlatformRequest, Point, Rect, Size};
pub use session::{EditorSession, SessionError};
pub use trace::{EDITOR_TRACE_VERSION, EditorTrace, TraceError};
