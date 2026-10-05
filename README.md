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
- Phase 5: Live Preview core — native Windows/macOS acceptance complete; Narrator/VoiceOver manual acceptance deferred
- Phase 6: Widgets — in progress; horizontal rule and task checkbox macOS PASS, standalone native image widget implemented with Windows/macOS/Linux CI PASS

Phase 3 includes a native winit + cosmic-text + glyphon/wgpu acceptance app, a framework-independent `mdedit-input` crate, shaped visual navigation, drag-selection auto-scroll, AccessKit text-editor semantics, semantic trace/replay, Windows DX12 startup diagnostics, macOS IME hardening, and the affinity-only horizontal caret-stop fix. Windows Korean/Japanese/Chinese and macOS Korean/Japanese/Chinese core IME acceptance passed, including macOS focus loss/refocus. The known first Korean syllable split immediately after switching the macOS input source remains an upstream `winit #3095` blocker; no editor-layer retry/queue workaround is carried. Narrator and VoiceOver manual acceptance are deferred to the later Markdown/Live Preview accessibility validation pass. Phase 4 adds a source-mapped Markdown syntax layer, project-owned semantic metadata, stable block identity, exact delimiter resolution, file-backed CommonMark/GFM/Obsidian compatibility fixtures, a generic extension scanner/merge layer, and parse snapshot reconciliation. An opt-in Obsidian compatibility scanner is included without making Obsidian behavior part of the core contract. Phase 5 now introduces the framework-independent `mdedit-live` projection/mapping foundation. The safe default keeps source visible; opt-in inactive concealment uses only delimiter ranges proven by Phase 4, structural source padding is tracked separately, Unicode-grapheme projected caret stops prevent hidden source offsets from consuming extra horizontal movement steps, nested reveal resolution opens only the most specific syntax layer being edited, projected hit-test/selection helpers map view-facing projected byte positions back to canonical source edges, and the reflow contract preserves a canonical caret anchor across projection/layout changes while leaving actual viewport state and shaping geometry to the native view.

## Run the native IME lab

Use Rust 1.90 or newer, then run:

```bash
cargo run -p mdedit-ime-lab
```

On Windows, the IME lab defaults wgpu to DX12 because automatic multi-backend startup produced a native access violation on the tested Intel UHD Graphics 630 system. Manual acceptance confirmed DX12 startup on that adapter. Advanced testing can still override this with `WGPU_BACKEND`; FIFO versus Mailbox was also tested and was not the cause of the former two-key horizontal caret symptom.

The lab remains available for Windows/macOS IME regression acceptance. Core Korean/Japanese/Chinese IME cases are accepted.

For the Phase 5 Markdown/Live Preview acceptance document, enable the built-in fixture:

Windows PowerShell:

```powershell
$env:MDEDIT_LIVE_PREVIEW_ACCEPTANCE = "1"
cargo run -p mdedit-ime-lab --release
```

macOS:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

Set `MDEDIT_LIVE_PREVIEW_TRACE=1` when projection/reflow diagnostics are needed. Windows/macOS native Live Preview acceptance passed, including IME viewport stability and cross-line/cross-viewport arrow navigation. Narrator/VoiceOver manual acceptance remains deferred. See `docs/phase-05-native-acceptance.md`.

Development plans and architecture are maintained under `docs/`.
