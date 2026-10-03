# Phase 5 Native Live Preview Acceptance

Branch: `feature/live-preview-phase-05`

This checklist validates the native Phase 5 projection integration after the framework-independent projection contracts have been wired into the Phase 3 IME lab.

## Run the acceptance fixture

The normal IME lab keeps its Phase 3 document. Enable the Markdown-focused fixture explicitly.

Windows PowerShell:

```powershell
$env:MDEDIT_LIVE_PREVIEW_ACCEPTANCE = "1"
cargo run -p mdedit-ime-lab --release
```

macOS:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

Use an optimized build for subjective caret/input latency checks.

When debugging projection or reflow behavior, enable the projection trace as well.

Windows PowerShell:

```powershell
$env:MDEDIT_LIVE_PREVIEW_TRACE = "1"
```

macOS:

```bash
MDEDIT_LIVE_PREVIEW_TRACE=1 MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

The projection trace records source/display lengths, canonical and projected primary selection offsets, preedit display range, and before/after reflow offsets whenever the projected display changes.

The fixture lives at:

```text
examples/ime-lab/fixtures/live-preview-acceptance.md
```

## A. Initial concealment

Expected when the caret is outside each construct:

- ATX heading marker and structural separator are concealed.
- strong, emphasis, strikethrough, and inline-code delimiters are concealed.
- link destination/delimiters are concealed while linked text remains visible.
- block quote and list markers plus their structural separator padding are concealed.
- Setext underline is concealed without leaving a blank projected line.
- source text is never rewritten just because syntax is concealed.

Failure examples:

- visible stray `**`, `#`, `>`, list marker padding, or Setext underline while inactive
- missing ordinary content
- extra blank line where the Setext underline was hidden

## B. Reveal specificity

Move the caret through:

```markdown
**outer *inner* tail**

***combined emphasis and strong***
```

Expected:

- caret inside the inner emphasis reveals the inner syntax layer without unnecessarily opening the parent layer
- caret in outer-only text reveals the outer layer
- compact combined delimiters reveal the syntax layer currently being edited
- adjacent constructs do not reveal merely because a selection endpoint touches their boundary

## C. Horizontal navigation

Repeatedly press Left/Right through concealed markers, Korean text, Japanese text, Chinese text, the family emoji, and the combining sequence.

Expected:

- one key press produces one visible caret step
- hidden Markdown bytes never consume an extra key press
- grapheme clusters are not split
- no two-press affinity-only regression reappears

## D. Vertical navigation and reflow

Move Up/Down between wrapped plain text, headings, lists, and revealed/concealed constructs.

Expected:

- sticky preferred-X behavior remains usable
- reveal/conceal reflow does not cause unexpected viewport jumps
- the tracked caret remains visually stable when projection width/height changes
- caret auto-scroll still keeps the active caret visible

## E. Mouse and drag selection

Click near concealed prefix/suffix boundaries and drag across multiple constructs.

Expected:

- clicking a projected position maps to a canonical source edge, never into an invalid UTF-8 byte
- selection direction is preserved
- selection can cross concealed constructs without becoming stuck
- selection highlight matches the projected text on screen

## F. IME

Use the fixture's Korean, Japanese, and Chinese scratch lines.

Expected:

- preedit appears in the projected display
- preedit does not mutate canonical source before commit
- candidate window follows the projected caret
- commit becomes one canonical edit
- visible composition forces the affected syntax construct open when needed
- Backspace/Delete/candidate interaction remains owned by the native IME while preedit is active

The previously documented macOS Korean first-syllable issue immediately after switching input sources remains an upstream `winit #3095` item and should not be reclassified as a Phase 5 editor regression unless behavior changes outside that known case.

## G. Scroll continuity

Scroll so the caret is away from the first viewport, then move into and out of syntax that changes projected width or line wrapping.

Expected:

- no large jump to document start/end
- block-axis compensation keeps the tracked caret near the same viewport location
- ordinary follow-caret behavior still applies when the caret would otherwise leave the viewport

## H. Narrator - Windows

Run the fixture with Windows Narrator.

Verify:

- the editor is exposed as a multiline text input
- Narrator reads projected Live Preview text instead of hidden Markdown marker bytes
- text selection reported by Narrator matches the projected selection
- accessibility selection actions map back to the intended canonical source
- selected-text replacement edits source without deleting unrelated hidden Markdown syntax
- focus can leave and return to the editor without corrupting selection or IME state

Do not expect accessibility `SetValue` to replace the entire document while syntax is concealed. The lab intentionally rejects that lossy operation because the projected value cannot represent hidden Markdown bytes losslessly.

## I. VoiceOver - macOS

Run the fixture with VoiceOver.

Verify the same projected-text, projected-selection, source-mapped edit, focus, and IME expectations as the Narrator pass.

## J. Regression smoke

Before marking Phase 5 acceptance complete, also run the normal Phase 3 document without the environment variable and confirm:

- Korean/Japanese/Chinese native IME behavior remains unchanged
- Left/Right remains one visible step
- clipboard, undo/redo, drag selection, scroll, and focus handling still work
- Windows DX12 startup behavior remains unchanged

## Result recording

Record each platform independently:

```text
Windows native Live Preview: PASS / FAIL / BLOCKED
Windows Narrator: PASS / FAIL / DEFERRED

macOS native Live Preview: PASS / FAIL / BLOCKED
macOS VoiceOver: PASS / FAIL / DEFERRED
```

For failures, include:

- OS/version
- input language/IME
- exact fixture line
- action sequence
- expected result
- actual result
- whether canonical source was corrupted
- `MDEDIT_TRACE_FILE` trace when input behavior is involved
- `MDEDIT_LIVE_PREVIEW_TRACE=1` log when projection, hit mapping, or reflow is involved
