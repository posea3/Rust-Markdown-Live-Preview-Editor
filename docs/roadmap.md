# Implementation Roadmap

## Phase 0 - Repository foundation

Status: **complete**

Deliverables:

- Cargo workspace
- Rust 1.90 baseline
- CI for Windows/macOS/Linux
- project-owned TextSize/TextRange/Revision types
- unsafe forbidden in core
- architecture and roadmap documents

Exit gate:

- workspace builds cleanly
- fmt/check/clippy/test CI passes
- core has no GUI or parser dependency

## Phase 1 - Document core

Status: **complete**

Deliverables:

- crop-backed Document and cheap snapshots
- Revision
- Change / ChangeSet
- Transaction
- ChangeMap old-source <-> new-source mapping
- UTF-8 boundary validation
- inverse transaction generation
- History undo/redo
- SelectionSet model prepared for multiple selections
- property tests for edit/inverse and ChangeMap
- non-overlapping multi-change round trips
- large-document fixture
- explicit CRLF cases
- explicit history grouping metadata and grouped undo/redo

## Phase 2 - Selection and navigation

Status: **complete**

Implemented:

- normalized multi-selection sets
- grapheme movement
- Unicode word movement
- logical source line start/end
- document start/end
- selection-head movement and extension primitive
- grapheme-safe delete backward/forward
- multi-cursor deletion range merging
- documented logical vs visual BiDi boundary

Exit gate:

- emoji ZWJ sequences are not split
- combining sequences are not split
- Korean/Japanese/Chinese source edits remain boundary-safe
- CRLF behaves correctly
- Windows/macOS/Linux CI passes

## Phase 3 - Raw native editor / IME lab

Status: **in progress — Windows Korean/GPU/caret acceptance passed; remaining platform acceptance pending**

Implemented in the first Phase 3 slice:

- framework-independent `mdedit-input` crate
- `EditorInput` / `PlatformRequest` boundary
- ephemeral `CompositionState`
- preedit display projection without canonical source mutation
- IME commit as one document transaction
- winit native IME event bridge in the acceptance lab
- cosmic-text shaping and hit testing
- glyphon/wgpu text rendering
- caret placement and IME candidate rectangle
- mouse caret placement and drag selection
- visual Up/Down movement across soft-wrapped layout lines
- shaped visual left/right movement across BiDi text
- sticky preferred-X vertical navigation
- dedicated GPU caret geometry
- rectangular GPU selection geometry using cosmic-text BiDi highlight spans
- wheel/trackpad scrolling with persistent viewport state
- caret auto-scroll after edits/navigation
- drag-selection auto-scroll outside the visible viewport
- soft wrapping
- clipboard copy/cut/paste
- undo/redo and grapheme deletion integration
- focus-loss / IME-disabled composition cancellation regression tests
- AccessKit window/text-input/text-run semantic tree with selection and edit actions
- versioned semantic EditorInput trace capture/replay for converting platform acceptance failures into CI regressions
- Windows DX12-first wgpu startup with adapter/backend diagnostics and environment overrides
- affinity-only horizontal caret-stop filtering so one Left/Right press produces one visible step
- caret-only redraw/layout/accessibility hot-path reductions validated during Windows acceptance

Manual acceptance now confirmed on Windows:

- Korean IME: PASS
- Left/Right one press = one visible caret step: PASS
- sustained horizontal movement: PASS
- Intel UHD Graphics 630 DX12 startup: PASS
- FIFO/Mailbox comparison: not the cause of the former two-key caret symptom

Remaining Phase 3 hardening:

- Windows Japanese IME acceptance
- Windows Chinese IME acceptance
- macOS Korean/Japanese/Chinese IME acceptance
- manual focus-loss/composition edge-case acceptance
- Narrator/VoiceOver text, selection, and edit-action acceptance

Phase 4 syntax work may proceed on this accepted Windows baseline. Do not start Phase 5 Live Preview until the required IME, undo/redo, selection, mouse editing, and accessibility gates are stable on the intended Tier 1 platforms.

## Phase 4 - Markdown syntax engine

Status: **in progress — parser boundary and source-mapped syntax tree implemented**

Implemented:

- `mdedit-markdown` crate
- `MarkdownParser` trait
- pulldown-cmark adapter behind project-owned types
- `SyntaxSnapshot`
- `SyntaxNode` / `SyntaxKind`
- `MarkdownDialect`
- raw-source failure mode
- full-parse correctness baseline with source-range validation
- stable top-level block identity/cache boundary with content fingerprints and conservative reconciliation

Remaining:

- DelimiterResolver
- extension scanner/merge policy
- richer semantic metadata and compatibility fixtures

First rendering milestone keeps all Markdown markers visible and adds only semantic styles.

## Phase 5 - Live Preview core

Add `mdedit-live`.

Implement:

- Projection
- ProjectedBlock / ProjectedSpan
- ProjectionMap
- Concealed spans
- RevealGroup
- RevealPolicy
- caret-stop model
- selection intersection reveal
- IME force reveal
- reflow/caret scroll compensation

Initial supported concealment:

- headings
- strong
- emphasis
- strikethrough
- inline code
- block quote marker
- list marker
- link syntax

## Phase 6 - Widgets

Order:

1. horizontal rule
2. task checkbox
3. image
4. math
5. table
6. callout/custom embed

Widgets emit editor/host actions. They never mutate the buffer directly.

## Phase 7 - Public extension API

Ship sample extensions and compatibility tests.

Target use cases:

- CommonMark/GFM-only apps
- Obsidian-like wikilinks/highlights/callouts
- application-specific references

NodeNotes-specific behavior remains outside the core repository.

## Phase 8 - Large-document performance

Implement only after profiling:

- background parse coordinator
- block reconciliation
- incremental projection
- block height index
- viewport virtualization
- layout cache
- incremental Markdown parsing with full-parse fallback

Fixtures: 10 KB, 100 KB, 1 MB, 10 MB, and a 1,000,000-character single line.

## Phase 9 - Library hardening / 1.0 gate

- API stability review
- accessibility semantics
- fuzz corpus
- dependency upgrade acceptance suite
- adapter documentation
- Windows/macOS native acceptance
- Korean IME acceptance
- malformed Markdown no-panic guarantee
