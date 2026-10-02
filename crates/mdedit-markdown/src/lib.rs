#![forbid(unsafe_code)]

//! Source-mapped Markdown syntax for the native editor.
//!
//! This crate deliberately hides parser-specific types behind project-owned
//! syntax and dialect abstractions. It does not implement Live Preview or
//! conceal Markdown source markers.

mod block;
mod delimiter;
mod dialect;
mod extension;
mod metadata;
mod obsidian;
mod parser;
mod pulldown;
mod syntax;

pub use block::{
    BlockCache, BlockCacheError, BlockEntry, BlockFingerprint, BlockId, BlockSnapshot,
};
pub use delimiter::{
    DelimiterError, DelimiterIssue, DelimiterIssueReason, DelimiterKind, DelimiterResolver,
    DelimiterSnapshot, DelimiterSpan,
};
pub use dialect::MarkdownDialect;
pub use extension::{
    ExtensionAttribute, ExtensionCandidate, ExtensionError, ExtensionId, ExtensionIssue,
    ExtensionIssueReason, ExtensionKind, ExtensionMatch, ExtensionOverlapPolicy,
    ExtensionScanContext, ExtensionScanMode, ExtensionSet, ExtensionSnapshot, SyntaxExtension,
};
pub use obsidian::{
    BLOCK_ID as OBSIDIAN_BLOCK_ID, CALLOUT as OBSIDIAN_CALLOUT, COMMENT as OBSIDIAN_COMMENT,
    EMBED as OBSIDIAN_EMBED, HIGHLIGHT as OBSIDIAN_HIGHLIGHT, OBSIDIAN_EXTENSION_ID,
    ObsidianSyntaxExtension, WIKILINK as OBSIDIAN_WIKILINK, obsidian_extension_set,
};
pub use metadata::{
    SyntaxAttribute, SyntaxBlockQuoteKind, SyntaxCodeBlockKind, SyntaxLinkMetadata,
    SyntaxLinkType, SyntaxMetadata, SyntaxMetadataBlockKind, SyntaxTableAlignment,
};
pub use parser::MarkdownParser;
pub use pulldown::PulldownCmarkParser;
pub use syntax::{ParseStatus, RawFallbackReason, SyntaxKind, SyntaxNode, SyntaxSnapshot};
