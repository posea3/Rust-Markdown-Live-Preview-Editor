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

Status: **implementation complete — Windows/macOS Korean/Japanese/Chinese core IME acceptance passed**

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
- macOS IME empty-preedit, non-Latin shortcut, trailing-line, teardown, selection-highlight, and focus-loss/refocus hardening

Manual acceptance:

- Windows Korean IME: PASS
- Windows Japanese IME: PASS
- Windows Chinese IME: PASS
- Windows Left/Right one press = one visible caret step and sustained movement: PASS
- Windows Intel UHD Graphics 630 DX12 startup: PASS
- FIFO/Mailbox comparison: not the cause of the former two-key caret symptom
- macOS Korean IME: PASS
- macOS Japanese IME: PASS
- macOS Chinese IME: PASS
- macOS focus loss/refocus: PASS
- macOS Korean first syllable immediately after switching input source: UPSTREAM BLOCKER (`rust-windowing/winit#3095`); no local editor-layer workaround is carried
- Narrator manual acceptance: DEFERRED
- VoiceOver manual acceptance: DEFERRED

Phase 3 is closed as an implementation milestone. Core Windows/macOS Korean/Japanese/Chinese IME acceptance is complete. The screen-reader checks are intentionally deferred until Markdown/Live Preview semantics exist, and the macOS Korean cold-start issue is tracked as a windowing-layer dependency item rather than an editor implementation blocker. Phase 4 and Phase 5 work may proceed on this base.

## Phase 4 - Markdown syntax engine

Status: **core complete — syntax, metadata, delimiters, block identity, extensions, compatibility fixtures, and parse reconciliation implemented**

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
- exact source `DelimiterResolver` for the initial Live Preview marker set with conservative unresolved diagnostics
- generic `SyntaxExtension` scanner/merge engine with deterministic overlap policy and protected syntax regions
- opt-in Obsidian compatibility scanner backed by real wikilink/embed/highlight/comment/callout/block-ID syntax
- project-owned semantic metadata for parser information needed by styling/widgets/host integration
- file-backed CommonMark/GFM/Obsidian compatibility fixtures
- parse request/result reconciliation contract for stale revisions, config epochs, superseded requests, and safe raw fallback

Remaining follow-up:

- additional compatibility cases discovered during Phase 5 projection work

First rendering milestone keeps all Markdown markers visible and adds only semantic styles.

## Phase 5 - Live Preview core

Status: **native Windows/macOS acceptance complete — Narrator/VoiceOver manual acceptance deferred**

Implemented in the first slice:

- framework-independent `mdedit-live` crate
- `Projection`
- `ProjectedBlock` / `ProjectedSpan`
- project-owned projected coordinate types
- bidirectional `ProjectionMap` with explicit before/after bias at collapsed boundaries
- semantic `StyleSpan` values
- exact delimiter-backed `ConcealSpan`
- `RevealGroup` / `RevealPolicy`
- caret intersection reveal
- selection intersection reveal
- IME composition force reveal through source-range context
- source-visible safe default
- raw projection fallback on inconsistent syntax/concealment ownership
- separate `StructuralPaddingSpan` modeling for ATX/Setext heading padding, block-quote separators, and list-marker padding
- conservative list padding rule that preserves four-space code indentation after markers
- Unicode-grapheme `ProjectedCaretStops` with direction-aware source-edge selection at collapsed boundaries
- hidden delimiter/padding offsets do not become invisible horizontal movement steps
- specificity-aware reveal resolution for nested constructs
- half-open selection/composition overlap so boundary-only adjacency does not reveal neighbors
- compact combined delimiter refinement (`***x***` reveals only the syntax layer being edited)
- projected-byte `HitBias` / `ProjectedHit` snapping through valid grapheme caret stops
- explicit canonical source-edge choice at collapsed hit boundaries
- `ProjectedSelectionEndpoint` / `ProjectedSelection` source-to-projected and projected-to-source mapping
- anchor/focus direction preservation, including selections whose endpoints collapse to the same projected position
- range-local selection mapping compatible with future `SelectionSet` multi-selection integration without ownership coupling
- `ReflowAnchor` before/after projected tracking for a canonical caret/selection anchor
- same-source reveal/conceal reflow mapping plus externally ChangeMap-mapped cross-revision anchor support
- finite logical `LayoutPosition` measurements on inline/block axes without view-framework types
- `ReflowMeasurement` / `ScrollAdjustment` contract where the view adds after-minus-before layout delta to its scroll offset
- explicit separation of scroll clamping, wrap width, shaped geometry, viewport policy, and auto-scroll into the native view

Initial exact-byte concealment covers resolved markers for:

- headings
- strong
- emphasis
- strikethrough
- inline code
- block quote marker
- list marker
- link syntax / autolinks

Native integration now includes:

- projected display rendering through the Phase 3 cosmic-text/glyphon/wgpu editor
- source/display caret and selection mapping
- projected mouse/drag hit testing and visual navigation
- IME preedit overlay on projected text
- reveal/conceal reflow compensation
- AccessKit projected text/selection exposure with selection actions mapped back to canonical source
- opt-in Markdown acceptance fixture and platform checklist

Native acceptance completed:

- Windows native Live Preview: PASS
- macOS native Live Preview: PASS
- Windows Korean/Japanese/Chinese IME preedit viewport stability: PASS
- macOS Korean printable-delimiter-after-preedit regression: PASS
- Up/Down navigation across viewport boundaries: PASS
- Left/Right navigation across logical line boundaries: PASS
- Windows/macOS/Linux CI: PASS

Deferred:

- Windows Narrator manual acceptance
- macOS VoiceOver manual acceptance

## Phase 6 - Widgets

Status: **in progress — horizontal rule and task checkbox macOS PASS; standalone image widget implementation in progress; Windows native manual acceptance deferred to the final integration pass**

Rules:

- canonical Markdown source remains authoritative
- widgets are derived from syntax + Live Preview projection
- inactive widgets may conceal only their proven source range
- entering a widget's source range reveals editable Markdown
- native views own geometry and painting
- widgets emit editor/host actions; they never mutate the buffer directly

Order:

1. horizontal rule
2. task checkbox
3. image
4. math
5. table
6. callout/custom embed

Widgets emit editor/host actions. They never mutate the buffer directly.

## Phase 7 - Extension API hardening

The public extension/scanner boundary is introduced in Phase 4 so syntax compatibility can be exercised before Live Preview. Phase 7 hardens that API after projection/widgets reveal real integration requirements.

Ship additional sample extensions, compatibility tests, and API-stability guidance.

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
