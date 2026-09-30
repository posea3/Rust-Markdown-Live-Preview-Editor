# Implementation Roadmap

## Phase 0 - Repository foundation

Status: **complete**

Deliverables:

- Cargo workspace
- Rust 1.89 baseline
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

Status: **implemented on feature/selection-navigation-phase-02; CI validation required**

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

Next.

Create a deliberately Markdown-free native editor executable.

Stack:

- winit
- cosmic-text
- glyphon/wgpu
- arboard
- AccessKit hooks

Implement:

- EditorInput / PlatformRequest boundary
- caret
- selection
- pointer hit testing
- drag selection
- scrolling
- soft wrapping
- clipboard
- Windows/macOS IME
- ephemeral CompositionState
- IME candidate rectangle

Do not start Live Preview until Korean IME, undo/redo, selection, and mouse editing are stable here.

## Phase 4 - Markdown syntax engine

Add `mdedit-markdown`.

Implement:

- MarkdownParser trait
- pulldown-cmark adapter
- SyntaxSnapshot
- SyntaxNode / SyntaxKind
- block identity/cache boundary
- MarkdownDialect
- DelimiterResolver
- raw-source failure mode
- extension scanner/merge policy

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
