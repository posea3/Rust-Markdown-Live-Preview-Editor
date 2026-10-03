# Phase 5 - Live Preview Core

Branch: `feature/live-preview-phase-05`

## Purpose

Phase 5 turns source-mapped Markdown syntax into a display projection without ever rewriting the canonical Markdown document.

The core invariant remains:

```text
canonical source
      |
      | ProjectionMap
      v
display projection
```

`ChangeMap` continues to map document revisions. `ProjectionMap` maps source coordinates to display coordinates. They are separate abstractions.

## First slice

The first Phase 5 slice adds the framework-independent `mdedit-live` crate with:

- `Projection`
- `ProjectionMap`
- project-owned `ProjectedSize` / `ProjectedRange`
- `ProjectedSpan` and `ProjectedBlock`
- semantic `StyleSpan` values
- `ConcealSpan`
- `RevealGroup` / `RevealGroupId`
- `RevealContext`
- `RevealPolicy`
- raw projection fallback
- separate structural padding spans for headings, Setext marker lines, block quotes, and list separators
- Unicode-grapheme `ProjectedCaretStops` for one visible horizontal step per movement
- `HitBias` / `ProjectedHit` for projected-byte interaction snapping
- `ProjectedSelectionEndpoint` / `ProjectedSelection` for direction-preserving source/viewport selection mapping
- `ReflowAnchor` for before/after projected tracking of a canonical source anchor
- `LayoutPosition` / `ReflowMeasurement` / `ScrollAdjustment` for unit-agnostic caret scroll compensation

No winit, cosmic-text, glyphon, wgpu, egui, or other view/runtime type is exposed by this crate.

## Safe default

`RevealPolicy::SourceVisible` is the default.

That mode produces an identity projection: all Markdown source remains visible while semantic style spans are still available. This provides the first rendering milestone without concealment risk.

`RevealPolicy::ConcealInactive` enables source-derived Live Preview concealment.

## Concealment

Concealment consumes the exact source marker spans already proven by Phase 4 `DelimiterResolver`.

Currently this can conceal resolved delimiters for:

- headings
- emphasis
- strong
- strikethrough
- inline code
- block quotes
- list items
- links
- autolinks

The projection does not independently scan punctuation. If a delimiter was not proven by the syntax layer, Live Preview does not guess.

Adjacent concealed markers are merged only for coordinate mapping. Their original `ConcealSpan` and reveal-group ownership remain available to downstream consumers.

## Reveal groups

Every resolved delimiter is attached to the smallest matching syntax node that owns it. This is important for nested Markdown such as:

```markdown
***nested***
```

The inner and outer constructs remain distinct reveal groups even though their adjacent hidden markers collapse to a single display boundary.

Reveal resolution is specificity-aware.

- `RevealPolicy::SourceVisible` still reveals every group.
- A caret or zero-length IME composition reveals the most specific nested group containing that source point.
- If two candidate groups have the same source extent, neither is discarded only because of nesting order.
- A non-empty selection/composition uses half-open overlap semantics, so merely touching the start/end boundary of an adjacent construct does not reveal it.
- When a non-empty interaction lies completely inside a nested child, that child shadows its containing parent.
- When an interaction crosses out of the child and genuinely spans both child and parent content, both groups reveal.
- Adjacent constructs remain independent unless the interaction actually overlaps both.

For example:

```markdown
**outer *inner* tail**
```

a caret inside `inner` reveals only the emphasis source markers:

```markdown
outer *inner* tail
```

while a caret in outer-only content reveals only the strong source markers:

```markdown
**outer inner tail**
```

Compact combined delimiters follow the same syntax-tree specificity. With `***x***`, a caret in `x` reveals the inner strong pair as `**x**`; a caret on the outer delimiter reveals the outer emphasis pair as `*x*`.

The IME rule remains expressed only in source coordinates. The native input layer remains independent from `mdedit-live`; a host/view adapter supplies the current composition range through `RevealContext`.

## ProjectionMap

Concealed source ranges collapse to display positions.

For:

```markdown
**bold**
```

the inactive projection is:

```text
bold
```

Both source offsets inside each hidden delimiter map to the same projected boundary.

Mapping back from that boundary is inherently ambiguous, so `projected_to_source` requires an explicit `ProjectionBias::Before` or `ProjectionBias::After`. This preserves both sides of a collapsed source interval rather than silently choosing one.

The map also publishes visible source/projected span pairs for later layout and hit-testing adapters.

## Structural padding

Structural layout padding is now modeled separately from syntax delimiters through `StructuralPaddingSpan`.

That distinction keeps Phase 4 delimiter ownership exact while allowing an inactive Live Preview projection to remove source-only spacing that would otherwise remain visible after a marker is concealed.

The current conservative rules are:

- ATX heading indentation before the marker is collapsed when it is only allowed heading indentation
- horizontal whitespace after an ATX heading prefix is collapsed
- horizontal whitespace before a proven closing ATX hash sequence is collapsed
- Setext underline indentation and the underline line remainder/newline are collapsed so the hidden underline does not leave a blank projected line
- one optional space/tab after each block-quote marker is collapsed
- list-marker padding of one to four spaces is collapsed
- when more than four spaces follow a list marker, only the first separator space is collapsed so four-space code indentation remains represented
- a list-marker tab separator is collapsed as one separator

Every structural padding span belongs to the same reveal group as the delimiter that caused it. Revealing that construct therefore restores both the Markdown marker and its source spacing.

For example:

```markdown
# Heading
> Quote
-   Item
```

projects while inactive as:

```text
Heading
Quote
Item
```

The `ConcealSpan` values still describe only actual Markdown delimiters. Structural padding remains separately observable for later caret, hit-test, and layout logic.

## Projected caret stops

`ProjectedCaretStops` derives the logical horizontal caret boundaries of the projected text from Unicode extended grapheme clusters.

This layer exists because multiple canonical source offsets can collapse to one projected position. Hidden delimiter bytes and structural padding therefore must not become invisible intermediate caret steps.

For an inactive construct:

```markdown
**bold**
```

the projected text is:

```text
bold
```

and the logical projected stops are only:

```text
|b|o|l|d|
```

There are no extra stops for the hidden `**` bytes.

Each projected stop stores both source edges of a collapsed boundary:

- `source_before`
- `source_after`

Entering a collapsed boundary from the right/backward direction selects the source edge adjacent to visible content after an opening marker. Entering from the left/forward direction selects the source edge adjacent to visible content before a closing marker. This avoids landing inside hidden marker bytes while preserving canonical source coordinates.

The model is grapheme-based, so Korean syllables, emoji ZWJ sequences, and combining sequences move as one visible unit.

If an externally supplied source offset maps inside a projected grapheme rather than exactly onto a caret stop, movement snaps once in the requested direction instead of consuming a second key press.

This is intentionally a logical projection model, not a shaped BiDi visual-order engine. A future view adapter combines these stops with cosmic-text (or another shaper) visual cell order. The Phase 3 rule remains: affinity-only state changes at the same visible location must not consume an additional Left/Right press.

## Projected hit testing and selection

The projection layer now exposes a one-dimensional interaction contract for a future native view. It still does not know about pixel x/y coordinates or cosmic-text layout objects.

`ProjectedCaretStops::hit_test` accepts a projected UTF-8 byte position plus an explicit `HitBias::Before` or `HitBias::After`.

- a byte position inside a visible grapheme snaps to the valid caret stop on the requested side
- a hit outside the projected document is rejected
- at a collapsed delimiter/padding boundary, the same bias selects `source_before` or `source_after` rather than guessing a canonical source offset
- the returned `ProjectedHit` records both the requested byte position and the snapped projected/source position

Selection mapping is built from the same caret-stop contract.

`ProjectedSelectionEndpoint::from_source` converts a core `Anchor` to a valid projected endpoint. If the source anchor already equals one of the exact canonical edges of a collapsed boundary, that edge is preserved. If a source offset falls inside hidden bytes or inside a projected grapheme, its core `Affinity` chooses the side to which it snaps. The endpoint keeps source affinity separately from hit bias so representable round trips retain anchor semantics.

`ProjectedSelection::from_source` maps the core selection anchor and head independently into projected anchor/focus endpoints. The inverse `to_source` mapping restores a `SelectionRange`. Because anchor and focus are never sorted, selection direction is preserved even when both endpoints collapse to the same projected byte position.

The helper operates on one `SelectionRange` at a time and does not own or normalize `SelectionSet`. A host can therefore map every range in a multi-selection set without moving input/session ownership into `mdedit-live`.

Pixel hit testing remains a later view-layer operation:

```text
mouse x/y
  -> shaped layout cell
  -> projected byte position
  -> mdedit-live hit helper
  -> canonical source offset
```

## Reflow and caret scroll compensation

Reveal/conceal changes can alter projected text width, line wrapping, caret position, and total document height. The projection layer now provides a contract for keeping a chosen canonical anchor visually stable without owning a viewport or a layout engine.

`ReflowAnchor` records:

- the canonical source anchor before reflow
- the canonical source anchor after reflow
- the corresponding projected selection endpoint before reflow
- the corresponding projected selection endpoint after reflow

For reveal/conceal changes on the same canonical source, `ReflowAnchor::same_source` tracks one `Anchor` through both projections.

For edits that also change the canonical document revision, `ReflowAnchor::mapped_source` accepts distinct before/after source anchors. The host remains responsible for mapping those anchors through `ChangeMap`; `mdedit-live` does not merge revision mapping into `ProjectionMap`.

After the native view shapes the old and new projected text, it measures the tracked endpoints in the same document-layout coordinate system and supplies two finite `LayoutPosition` values. These positions use logical `inline` and `block` axes rather than windowing or renderer types.

`ReflowMeasurement::scroll_adjustment` returns:

```text
inline adjustment = after.inline - before.inline
block adjustment  = after.block  - before.block
```

The view adds the requested adjustment to its current scroll offset to keep the tracked anchor at the same viewport location. The view owns policy:

- whether inline compensation is used or only block-axis compensation is applied
- scroll-bound clamping
- viewport dimensions and wrap width
- cosmic-text or other shaping/layout objects
- device-pixel/logical-pixel conversion
- follow-caret margins and ordinary caret auto-scroll

Non-finite layout measurements and overflowing deltas are rejected rather than propagated. If a source anchor cannot be represented by either projection, no compensation anchor is produced.

A native integration pass can therefore follow this sequence:

```text
canonical caret/head
  -> optional ChangeMap across document revisions
  -> ReflowAnchor before/after projected endpoints
  -> native shaped layout measures both endpoints
  -> ReflowMeasurement
  -> ScrollAdjustment
  -> view applies/clamps scroll
```

This keeps reflow continuity in the projection contract while leaving actual scrolling and layout ownership outside `mdedit-live`.

## Semantic styling

The projection currently publishes project-owned style spans for:

- heading level
- block quote
- code block
- emphasis
- strong
- strikethrough
- link
- inline code

Style spans preserve both source and projected ranges.

## Fallback

Projection input snapshots must agree on revision and source length.

If the syntax snapshot is already a raw fallback, if a delimiter cannot be associated with its owning syntax node, or if conceal spans overlap unexpectedly, the projection falls back to source-visible text instead of hiding uncertain bytes.

## Native view integration status

The Phase 3 IME lab now consumes the Phase 5 projection directly.

Integrated paths include:

- projected display text rendered through cosmic-text / glyphon / wgpu
- source-to-display caret mapping
- display hit testing back to canonical source anchors
- horizontal and vertical visual movement through projected coordinates
- drag selection through projected hit testing
- IME preedit overlay without mutating canonical source
- reveal/conceal reflow measurement with block-axis scroll compensation
- selection highlight mapping from canonical source to projected display
- AccessKit text and selection exposure in projected display coordinates
- AccessKit selection actions mapped back to canonical source coordinates

Accessibility full-value replacement is intentionally rejected while the display projection conceals Markdown source markers. Replacing the entire projected value cannot be losslessly mapped back to canonical Markdown without deleting hidden syntax. Selection-based edit actions remain available through the projection mapping.

## Remaining Phase 5 work

- native Live Preview manual acceptance on Windows and macOS
- Narrator / VoiceOver manual acceptance against projected Markdown semantics
- fix any platform-specific regressions found by those acceptance passes
