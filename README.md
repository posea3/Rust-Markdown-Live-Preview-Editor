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
- Phase 3: native input/IME lab — implementation complete

Phase 3 includes a native winit + cosmic-text + glyphon/wgpu acceptance app, a framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, AccessKit text-editor semantics, semantic trace/replay, Windows DX12 startup diagnostics, macOS IME hardening, and the affinity-only horizontal caret-stop fix. Windows Korean and macOS Korean/Japanese/Chinese core IME acceptance passed, including macOS focus loss/refocus. Windows Japanese/Chinese manual acceptance is deferred because those IME packs were unavailable on the test machine. Narrator and VoiceOver manual acceptance is deferred to a later Markdown/Live Preview validation pass. The known macOS first-Korean-syllable split immediately after switching input source is tracked as upstream winit issue #3095 rather than duplicated with an editor-layer workaround.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

On Windows, the IME lab defaults wgpu to DX12 because automatic multi-backend startup produced a native access violation on the tested Intel UHD Graphics 630 system. Manual acceptance confirmed DX12 startup on that adapter. Advanced testing can still override this with `WGPU_BACKEND`; FIFO versus Mailbox was also tested and was not the cause of the former two-key horizontal caret symptom.

The lab is intended for Windows/macOS IME acceptance. Test Korean composition, selection replacement, mouse caret placement/drag selection, wheel/trackpad scrolling, soft-wrapped Up/Down movement, BiDi left/right movement, clipboard actions, grapheme deletion, undo/redo, and Narrator/VoiceOver text and selection reporting before Markdown Live Preview work begins.

Development plans and architecture are maintained under `docs/`.
