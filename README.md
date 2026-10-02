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
- Phase 3: native input/IME lab — automated hardening complete; manual platform acceptance remains
- Phase 4: Markdown syntax engine — in progress on a stacked branch

The Phase 3 branch contains the native winit + cosmic-text + glyphon/wgpu acceptance app, the framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, AccessKit text-editor semantics, and semantic input trace/replay. Phase 4 now adds a source-mapped Markdown syntax layer on top; Live Preview projection is still intentionally disabled.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

The lab is intended for Windows/macOS IME acceptance. Test Korean composition, selection replacement, mouse caret placement/drag selection, wheel/trackpad scrolling, soft-wrapped Up/Down movement, BiDi left/right movement, clipboard actions, grapheme deletion, undo/redo, and Narrator/VoiceOver text and selection reporting before Markdown Live Preview work begins.

Development plans and architecture are maintained under `docs/`.
