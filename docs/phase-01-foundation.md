# Phase 0 + Phase 1 Implementation Notes

Branch: `feature/foundation-phase-01`

## What this phase establishes

The first implementation intentionally stops before Markdown parsing and rendering. It builds the source-editing invariants that every later feature depends on.

### Canonical coordinate

All document coordinates are UTF-8 byte offsets represented as `TextSize`. Public APIs use `TextRange` instead of naked Rust ranges.

### Transaction-only mutation

`Document::apply(Transaction)` is the canonical mutation path. A transaction carries its base revision, validated non-overlapping source-coordinate changes, optional resulting selection, and a semantic kind.

### Change mapping

`ChangeMap` maps anchors between pre-transaction and post-transaction source coordinate systems. This is intentionally different from the future `ProjectionMap` used by Live Preview.

### Inverse generation

Applying a transaction captures replaced source text and constructs an inverse transaction against the new revision. History builds undo/redo on top of this mechanism.

### UTF-8 safety

Change ranges are rejected when they are outside the document or split a UTF-8 code point.

## Deliberate omissions

This phase does not yet implement:

- grapheme navigation
- IME composition
- GUI/rendering
- Markdown parsing
- Live Preview projection
- history coalescing
- incremental parsing

Those are separate gates because mixing them into the initial text model would make failures much harder to isolate.

## Review checklist

Before merging this branch:

- CI passes on Windows/macOS/Linux.
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- review transaction inverse behavior for multiple edits
- review boundary mapping semantics for zero-width insertions/deletions
