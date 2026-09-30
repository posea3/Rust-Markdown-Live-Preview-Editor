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

The current Phase 3 slice includes a native winit + cosmic-text + glyphon/wgpu acceptance app and a framework-independent `mdedit-input` crate. Markdown parsing and Live Preview are intentionally not enabled yet.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

The lab is intended for Windows/macOS IME acceptance. Test Korean composition, selection replacement, mouse caret placement/drag selection, clipboard actions, grapheme deletion, and undo/redo before Markdown Live Preview work begins.

Development plans and architecture are maintained under `docs/`.
