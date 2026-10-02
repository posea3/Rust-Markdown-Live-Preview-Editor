#![forbid(unsafe_code)]

//! Source-mapped Markdown syntax for the native editor.
//!
//! This crate deliberately hides parser-specific types behind project-owned
//! syntax and dialect abstractions. It does not implement Live Preview or
//! conceal Markdown source markers.

mod dialect;
mod parser;
mod pulldown;
mod syntax;

pub use dialect::MarkdownDialect;
pub use parser::MarkdownParser;
pub use pulldown::PulldownCmarkParser;
pub use syntax::{
    ParseStatus, RawFallbackReason, SyntaxKind, SyntaxNode, SyntaxSnapshot,
};
