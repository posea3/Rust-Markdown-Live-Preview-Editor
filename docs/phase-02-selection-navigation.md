# Phase 2 - Selection, Navigation, and Deletion

Branch: `feature/selection-navigation-phase-02`

## Scope

Phase 2 extends the framework-independent editor core without introducing GUI, shaping, Markdown, or platform dependencies.

Implemented source-level primitives:

- normalized multi-selection sets
- primary-selection tracking through sorting/merging
- Unicode grapheme movement
- Unicode word movement
- line start/end movement
- document start/end movement
- selection-head movement/extension
- grapheme-safe backward/forward deletion
- grouped history metadata and explicit grouped undo/redo
- CRLF and large-document foundation tests

## Unicode segmentation

`unicode-segmentation` is used for UAX #29 grapheme and word boundaries. Cursor and deletion logic must never assume one Unicode scalar or one UTF-8 code point equals one user-visible character.

Important cases covered by tests include:

- emoji ZWJ family sequences
- combining marks
- Korean words
- CRLF treated as one grapheme for backward deletion

## BiDi policy

This phase intentionally implements **logical/source movement only**.

`mdedit-core` can answer:

- previous/next grapheme
- previous/next word boundary
- source line start/end
- document start/end

It must **not** guess visual left/right/up/down in bidirectional or wrapped text.

Visual movement requires shaped layout information and belongs to the future `mdedit-view` / text-layout layer:

```text
source anchor
    |
    | logical movement: mdedit-core
    v
source anchor

source anchor
    |
    | source -> projection -> layout
    v
visual caret
    |
    | visual movement: mdedit-view
    v
visual caret
    |
    | reverse layout/projection map
    v
source anchor
```

This separation prevents RTL/BiDi behavior from being incorrectly approximated in the source model.

## Selection normalization

`SelectionSet::new` sorts ranges by ordered source bounds, merges overlapping/touching ranges, and preserves the primary range's direction when a merge contains it.

This invariant ensures downstream multi-cursor edits can construct non-overlapping `ChangeSet` values.

## Deletion model

`deletion_transaction` never mutates the document directly.

For a non-empty selection it removes the selected source range. For a caret it removes one Unicode grapheme backward or forward. Multi-cursor delete ranges are merged before transaction construction.

The resulting caret positions are mapped through `ChangeMap` and attached as `selection_after` on the transaction.

## History grouping

Transactions now carry:

```text
HistoryGroup::Isolated
HistoryGroup::Explicit(id)
```

Explicitly equal adjacent groups are stored as one history entry, allowing a host/editor layer to group a typing session or IME commit sequence without putting clocks or platform policy into `mdedit-core`.

Automatic time-based typing grouping remains a later editor-layer policy.

## Exit criteria

Phase 2 is complete when CI confirms:

- emoji ZWJ sequences are not split
- combining sequences are not split
- CJK source movement is UTF-8 safe
- CRLF deletion is stable
- multi-edit inverse properties hold
- ChangeMap remains monotonic
- logical/visual BiDi boundary is documented
