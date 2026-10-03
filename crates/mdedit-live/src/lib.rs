#![forbid(unsafe_code)]

//! Framework-independent Live Preview projection.
//!
//! Canonical Markdown remains owned by mdedit-core. This crate derives a display
//! projection plus explicit source/projection mappings; it never rewrites source.

mod caret;
mod projection;

pub use caret::{CaretDirection, ProjectedCaretStop, ProjectedCaretStops};
pub use projection::{
    ConcealSpan, ProjectedBlock, ProjectedRange, ProjectedSize, ProjectedSpan, Projection,
    ProjectionBias, ProjectionBuildError, ProjectionFallbackReason, ProjectionMap,
    ProjectionStatus, RevealContext, RevealGroup, RevealGroupId, RevealPolicy,
    StructuralPaddingKind, StructuralPaddingSpan, StyleKind, StyleSpan,
};
