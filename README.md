# Rust Markdown Live Preview Editor

A source-first, native Rust Markdown editing engine with Live Preview.

## Goals

- Pure Rust application/editor stack: no JavaScript, HTML, WebView, Electron, or CodeMirror runtime.
- Markdown source remains the canonical document.
- Reusable across multiple Rust applications rather than tied to a single host app.
- Native editing foundations first: transactions, position mapping, selection, IME, layout, then Live Preview.
- Windows and macOS are Tier 1 targets.

## Current status

- Phase 0: repository foundation — complete
- Phase 1: document/transaction/history core — complete
- Phase 2: selection, Unicode navigation, and grapheme-safe deletion — complete
- Phase 3: native input/IME lab — in progress

The current Phase 3 slice includes a native winit + cosmic-text + glyphon/wgpu acceptance app, a framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, and AccessKit text-editor semantics. Markdown parsing and Live Preview are intentionally not enabled yet.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

On Windows, the IME lab defaults wgpu to DX12 because automatic multi-backend startup produced a native access violation on the tested Intel UHD Graphics 630 system. Advanced testing can still override this with `WGPU_BACKEND`.

The lab is intended for Windows/macOS IME acceptance. Test Korean composition, selection replacement, mouse caret placement/drag selection, wheel/trackpad scrolling, soft-wrapped Up/Down movement, BiDi left/right movement, clipboard actions, grapheme deletion, undo/redo, and Narrator/VoiceOver text and selection reporting before Markdown Live Preview work begins.

Development plans and architecture are maintained under `docs/`.
