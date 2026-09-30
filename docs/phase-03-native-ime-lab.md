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
- visual preedit color
- caret position feeding `Window::set_ime_cursor_area`

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

## Known limitations for this first Phase 3 slice

- visual up/down movement is not implemented yet
- visual left/right BiDi movement remains in the future view layer
- selection uses text-color differentiation rather than a rectangular selection background
- caret is rendered as a text glyph for the lab rather than a dedicated geometry primitive
- scrolling beyond the initial buffer viewport is not yet wired to mouse wheel
- AccessKit semantic tree is not yet connected
- IME behavior still requires manual Windows/macOS acceptance testing

These limitations are intentionally recorded rather than hidden. They are the next Phase 3 hardening work before Markdown begins.
