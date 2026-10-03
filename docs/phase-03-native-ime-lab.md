# Phase 3 - Native IME Lab

Branch: `feature/native-ime-lab-phase-03`

## Purpose

Phase 3 creates the first real native editor window, but deliberately does **not** add Markdown parsing or Live Preview.

The goal is to isolate the hardest platform/editor risks before syntax projection exists:

- Windows/macOS IME
- Korean composition
- canonical document vs ephemeral preedit
- caret positioning
- IME candidate-window position
- selection and mouse hit testing
- clipboard
- undo/redo
- soft wrapping
- Unicode shaping

## Architecture added in this phase

### mdedit-input

A framework-independent crate between platform events and `mdedit-core`.

It owns:

- `EditorSession`
- `CompositionState`
- semantic `EditorInput`
- `PlatformRequest` vocabulary
- insertion/deletion/history orchestration

It contains no winit/wgpu/glyphon types.

### mdedit-ime-lab

A native acceptance executable using:

- winit 0.30
- cosmic-text 0.19 through glyphon
- glyphon 0.12
- wgpu 30
- arboard

The lab is intentionally not an application framework. It exists to validate the reusable engine boundaries.

## IME invariant

Preedit is not canonical source.

```text
Document
  +
CompositionState
  =
Display projection
```

During:

```text
Ime::Preedit("ㅎ")
Ime::Preedit("하")
Ime::Preedit("한")
```

the document remains unchanged.

Only:

```text
Ime::Commit("한")
```

creates a `TransactionKind::ImeCommit` transaction and history entry.

winit may emit an empty Preedit immediately before Commit. Therefore an empty preedit updates the composition overlay but does **not** discard its replace range.

## Multi-selection IME policy

Normal typing can operate on normalized multiple selections.

Native IME composition is constrained to the primary selection. When composition begins, secondary selections are discarded and the primary replacement range is frozen for the lifetime of that composition.

This keeps the platform IME contract deterministic. Multi-cursor replicated IME input is not attempted.

## Native lab interaction

The initial lab supports:

- text typing
- Korean/Japanese/Chinese IME events
- left/right logical grapheme movement
- visual Up/Down movement across soft-wrapped layout lines with sticky preferred X
- shaped visual left/right movement across BiDi text
- Home/End source-line movement
- Shift selection extension
- Backspace/Delete by grapheme
- mouse caret placement
- mouse drag selection
- Ctrl/Cmd+A
- Ctrl/Cmd+C/X/V
- Ctrl/Cmd+Z
- Ctrl/Cmd+Shift+Z and Ctrl+Y
- soft wrapping
- wheel/trackpad scrolling with persistent cosmic-text viewport state
- drag-selection auto-scroll beyond the visible viewport
- caret auto-scroll after edits/navigation via `shape_until_cursor`
- rectangular GPU selection background using cosmic-text BiDi highlight spans
- dedicated GPU caret geometry
- visual preedit color
- caret position feeding `Window::set_ime_cursor_area`
- AccessKit `Window -> MultilineTextInput -> TextRun` semantics
- AccessKit text selection, replace-selected-text, and set-value action bridging
- versioned semantic editor trace capture/replay for IME, focus, selection, navigation, deletion, and history events

## Rendering boundary

The lab uses cosmic-text only for shaping/layout/hit testing and glyphon only for GPU glyph rendering.

The canonical document, selections, history, and composition stay in mdedit crates.

This is intentional:

```text
mdedit-core / mdedit-input
       |
       v
temporary display string
       |
       v
cosmic-text Buffer
       |
       v
glyphon / wgpu
```

cosmic-text is not the source editor model.

## Performance acceptance note

For subjective input-latency comparisons against production editors such as Word or Hangul, use an optimized build:

```bash
cargo run --release -p mdedit-ime-lab
```

The default `cargo run` uses Rust's debug profile and is useful for diagnostics, but it is not representative of final editor latency. The Windows surface also keeps FIFO presentation to avoid tearing while limiting `desired_maximum_frame_latency` to 1 for lower input-to-display latency.

## Horizontal caret acceptance finding

Instrumented Windows runs showed horizontal input handling in roughly sub-millisecond to low-single-digit millisecond CPU time, with no meaningful FIFO vs Mailbox improvement. The remaining perceived slowness was traced to a correctness bug rather than throughput: at an ordinary LTR grapheme boundary the first Left/Right press could change only the cosmic-text cursor affinity at the same line/index, producing no visible movement, and the second press performed the actual grapheme move. Horizontal navigation now treats same-line/same-index affinity changes as the same logical caret stop and skips them. A regression test locks one-key-per-visible-grapheme movement while retaining distinct logical BiDi boundary transitions.

## Input latency diagnostics

Use the optimized build for latency checks:

```powershell
$env:MDEDIT_LATENCY_TRACE="1"
Remove-Item Env:MDEDIT_TRACE_FILE -ErrorAction SilentlyContinue
Remove-Item Env:MDEDIT_PRESENT_MODE -ErrorAction SilentlyContinue
cargo run --release -p mdedit-ime-lab
```

A single non-repeated arrow-key press emits a line like:

```text
[mdedit-latency] arrow-right handle=...ms redraw_wait=...ms render_present=...ms total=...ms
```

The timing ends when the present call returns; it does not include the monitor's final scanout. This distinction is useful: a small CPU-side total with visibly delayed feedback points toward presentation/compositor/vsync latency rather than caret-navigation work.

To compare presentation modes without changing code:

```powershell
$env:MDEDIT_PRESENT_MODE="mailbox"
cargo run --release -p mdedit-ime-lab
```

or:

```powershell
$env:MDEDIT_PRESENT_MODE="auto-no-vsync"
cargo run --release -p mdedit-ime-lab
```

`mailbox` is used only when reported by the surface; unsupported explicit modes fall back safely. `auto-no-vsync` lets wgpu choose Immediate, then Mailbox, then Fifo.

## macOS upstream IME constraints

Current macOS acceptance is affected by open winit 0.30.x AppKit issues in addition to application-level bugs:

- `rust-windowing/winit#3095`: immediately after switching to Korean, the first Jamo can escape as ordinary keyboard input before AppKit starts marked-text composition, splitting the first syllable. The observed `IMKCFRunLoopWakeUpReliable` console message is part of the same cold-start failure. An upstream mitigation exists as PR #4693, currently based on winit 0.31 beta and not directly interchangeable with this project's winit 0.30.x / accesskit_winit dependency set.
- `rust-windowing/winit#4526`: macOS can leak the final key-release event after an IME-owned Backspace clears all preedit.
- `rust-windowing/winit#4626`: dropping a 0.30.13 window with marked text active can abort in a late `insertText` callback. The IME lab disables IME and clears local composition before teardown as the documented application workaround.

The editor layer now ignores stray empty `Ime::Preedit("", None)` events when no composition exists, preserves an empty preedit only when it belongs to an existing composition awaiting commit, and resumes normal raw-key handling after an empty composition boundary.

## macOS focus-loss policy

Manual acceptance showed that Korean, Japanese, and Chinese preedit disappeared when the IME lab lost focus, while no ghost preedit remained and input resumed normally after refocus. That behavior matched the previous implementation because `EditorInput::Focused(false)` unconditionally cancelled composition.

For a text editor, visible marked text should not be silently discarded on focus loss. The macOS path now finalizes a non-empty preedit into the document before disabling IME; an empty preedit is cancelled without mutating source text. If AppKit has already committed the composition before the focus event, there is no remaining local composition and no duplicate commit occurs.

## Phase 3 completion status

**Implementation: COMPLETE**

Phase 3 is closed as an implementation milestone. Core Windows/macOS IME behavior required to continue editor development has been validated, with explicitly documented deferred/platform-owned items below. Windows Japanese IME has now also passed manual acceptance.

Manual acceptance:

- Windows Korean IME composition/commit/editing/selection replacement: **PASS**
- Windows Left/Right one press = one visible caret step and sustained movement: **PASS**
- Windows Intel UHD Graphics 630 DX12 startup: **PASS**
- Windows Japanese IME: **PASS**
- Windows Chinese IME: **DEFERRED / ENVIRONMENT BLOCKED** for the same reason
- macOS Korean normal composition/editing/selection/undo-redo: **PASS**
- macOS Japanese IME composition/candidate/editing/full-preedit deletion: **PASS**
- macOS Chinese IME composition/candidate/editing: **PASS**
- macOS focus loss/refocus: **PASS**; visible non-empty preedit is finalized before IME disable, no ghost preedit remains, and input resumes after refocus
- macOS Korean first syllable immediately after switching input source: **UPSTREAM BLOCKER** (`rust-windowing/winit#3095`); no local workaround is carried in mdedit
- Narrator manual acceptance: **DEFERRED**
- VoiceOver manual acceptance: **DEFERRED**

Accessibility implementation remains present through AccessKit, but Narrator/VoiceOver manual acceptance is intentionally deferred to a later validation pass when Markdown/Live Preview semantics are available. These deferred checks are not treated as passed.

The macOS Korean cold-start issue is intentionally left to the windowing layer. As of Phase 3 closure, winit issue #3095 and PR #4693 remain open. The editor will re-evaluate the dependency when an upstream fix is merged/released or when a stable compatible backport becomes justified by real-world severity. Phase 3 does not duplicate AppKit retry/queue logic in the editor layer.

Two Windows issues found during acceptance were resolved in Phase 3:

1. Automatic multi-backend wgpu startup terminated with `STATUS_ACCESS_VIOLATION (0xc0000005)` on the tested Intel UHD Graphics 630 machine. Windows now defaults to DX12 before applying wgpu environment overrides; `WGPU_BACKEND` can still explicitly override the default.
2. Visual Left/Right initially required two presses for one visible step because the first press could consume an affinity-only cosmic-text caret stop at the same line/index. The horizontal path now skips those redundant ordinary stops while preserving distinct logical BiDi boundary transitions. Manual acceptance confirms one press = one visible step.

macOS acceptance also closed application-level regressions found during testing:

- non-Latin input sources no longer break Cmd/Ctrl shortcuts
- selection highlights are restricted to selected source lines
- ordinary IME-owned editing keys are not consumed by the editor during visible composition
- stray empty preedit events do not create phantom composition
- Japanese full-preedit deletion no longer panics on the rich-text trailing-empty-line path
- IME is disabled before native window teardown to avoid late marked-text callbacks
- focus loss finalizes visible preedit instead of silently discarding it

Phase 4 and later editor work may proceed on this Phase 3 base. The deferred accessibility checks and upstream Korean cold-start issue remain tracked validation/dependency items rather than implementation blockers.

## Capturing a platform regression trace

Set `MDEDIT_TRACE_FILE` when running the IME lab. The lab writes the initial trace header once and then appends one encoded event line per semantic editor input, so an interrupted or crashed acceptance session still leaves the latest complete trace without rewriting the growing file on every key press.

Windows PowerShell:

```powershell
$env:MDEDIT_TRACE_FILE="ime-windows.trace"
cargo run -p mdedit-ime-lab
```

macOS/Linux:

```bash
MDEDIT_TRACE_FILE=ime-macos.trace cargo run -p mdedit-ime-lab
```

The trace stores the initial canonical source/selection plus every `EditorInput` using a versioned, UTF-8-safe text format. A captured trace can be loaded with `EditorTrace::decode` and replayed through `EditorTrace::replay` without winit, cosmic-text, or a platform IME. When a manual acceptance case exposes a bug, keep the trace as a regression fixture and assert its final canonical source, selection, composition state, and undo behavior in CI.


## MSRV adjustment

Phase 3 adds the native glyphon/wgpu dependency graph. The currently resolved graph includes `ordered-float 5.5`, which declares Rust 1.90. The workspace MSRV is therefore raised from Rust 1.89 to Rust 1.90 rather than pinning an older transitive dependency during early development.
