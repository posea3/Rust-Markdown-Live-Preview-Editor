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

A group is revealed when:

- `RevealPolicy::SourceVisible` is active
- a caret lies within or on the construct
- a selection intersects/touches the construct
- an IME composition range intersects/touches the construct

The IME rule is expressed only in source coordinates. The native input layer remains independent from `mdedit-live`; a host/view adapter supplies the current composition range through `RevealContext`.

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

## Remaining Phase 5 work

- caret-stop model over projected coordinates
- reveal-policy refinement for nested/adjacent constructs
- projected selection/hit-test helpers needed by the view layer
- reflow/caret scroll compensation contract
- native acceptance integration in the editor view
