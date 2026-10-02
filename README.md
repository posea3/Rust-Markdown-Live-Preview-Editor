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
- Phase 3: native input/IME lab — Windows Korean/GPU/caret acceptance passed; remaining platform acceptance pending
- Phase 4: Markdown syntax engine — in progress on a stacked branch

The current Phase 3 slice includes a native winit + cosmic-text + glyphon/wgpu acceptance app, a framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, AccessKit text-editor semantics, semantic trace/replay, Windows DX12 startup diagnostics, and the affinity-only horizontal caret-stop fix. Windows Korean IME, one-key-per-visible-step Left/Right navigation, sustained movement, and Intel UHD Graphics 630 DX12 startup have passed manual acceptance. Windows Japanese/Chinese IME, macOS Korean/Japanese/Chinese IME, Narrator, and VoiceOver remain pending. Phase 4 now adds a source-mapped Markdown syntax layer, stable block identity, exact delimiter resolution, and a generic extension scanner/merge layer. An opt-in Obsidian compatibility scanner is included so Obsidian-flavored files can be recognized without making Obsidian behavior part of the core contract. Live Preview projection remains intentionally disabled.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

On Windows, the IME lab defaults wgpu to DX12 because automatic multi-backend startup produced a native access violation on the tested Intel UHD Graphics 630 system. Manual acceptance confirmed DX12 startup on that adapter. Advanced testing can still override this with `WGPU_BACKEND`; FIFO versus Mailbox was also tested and was not the cause of the former two-key horizontal caret symptom.

The lab is intended for Windows/macOS IME acceptance. Test Korean composition, selection replacement, mouse caret placement/drag selection, wheel/trackpad scrolling, soft-wrapped Up/Down movement, BiDi left/right movement, clipboard actions, grapheme deletion, undo/redo, and Narrator/VoiceOver text and selection reporting before Markdown Live Preview work begins.

Development plans and architecture are maintained under `docs/`.
