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
- Phase 3: native input/IME lab — implementation complete; Windows/macOS Korean/Japanese/Chinese core IME acceptance passed
- Phase 4: Markdown syntax engine — core implementation complete on a stacked branch

Phase 3 includes a native winit + cosmic-text + glyphon/wgpu acceptance app, a framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, AccessKit text-editor semantics, semantic trace/replay, Windows DX12 startup diagnostics, macOS IME hardening, and the affinity-only horizontal caret-stop fix. Windows Korean/Japanese/Chinese and macOS Korean/Japanese/Chinese core IME acceptance passed, including macOS focus loss/refocus. The known first Korean syllable split immediately after switching the macOS input source remains an upstream `winit #3095` blocker; no editor-layer retry/queue workaround is carried. Narrator and VoiceOver manual acceptance are deferred to the later Markdown/Live Preview accessibility validation pass. Phase 4 adds a source-mapped Markdown syntax layer, project-owned semantic metadata, stable block identity, exact delimiter resolution, file-backed CommonMark/GFM/Obsidian compatibility fixtures, a generic extension scanner/merge layer, and parse snapshot reconciliation. An opt-in Obsidian compatibility scanner is included without making Obsidian behavior part of the core contract. Live Preview projection remains intentionally disabled until Phase 5.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

On Windows, the IME lab defaults wgpu to DX12 because automatic multi-backend startup produced a native access violation on the tested Intel UHD Graphics 630 system. Manual acceptance confirmed DX12 startup on that adapter. Advanced testing can still override this with `WGPU_BACKEND`; FIFO versus Mailbox was also tested and was not the cause of the former two-key horizontal caret symptom.

The lab remains available for Windows/macOS IME regression acceptance. Core Korean/Japanese/Chinese IME cases are accepted; Narrator/VoiceOver text, selection, and edit-action acceptance is deferred until the Markdown/Live Preview accessibility pass.

Development plans and architecture are maintained under `docs/`.
