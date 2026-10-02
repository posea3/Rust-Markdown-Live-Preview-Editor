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

## Remaining Phase 3 hardening

- Korean IME still requires manual Windows/macOS acceptance testing
- Japanese/Chinese IME still requires manual acceptance testing
- focus-loss and IME disable/re-enable edge cases still require manual acceptance
- Narrator/VoiceOver text, selection, and edit actions still require manual acceptance

These limitations are intentionally recorded rather than hidden. They are the next Phase 3 hardening work before Markdown begins.

## Capturing a platform regression trace

Set `MDEDIT_TRACE_FILE` when running the IME lab. The lab rewrites that file after every semantic editor input, so an interrupted or crashed acceptance session still leaves the latest complete trace.

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
