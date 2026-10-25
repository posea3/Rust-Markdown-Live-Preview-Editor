# Phase 6 Widgets

Branch: `feature/widgets-phase-06`

Phase 6 adds native widgets on top of the Phase 5 source/projection contract. Canonical Markdown is never replaced by widget state.

## Architecture contract

1. `mdedit-markdown` owns syntax and exact source ranges.
2. `mdedit-live` decides whether a widget is rendered or source-visible from the current reveal context.
3. A rendered widget exposes a `ProjectedWidget` with canonical `source_range` and projected anchor/range.
4. The native view maps projected widget positions through shaped text layout and owns geometry/painting.
5. Entering a widget source range reveals the original Markdown for editing.
6. Widget actions are editor/host requests. Widgets never mutate the document buffer directly.

This keeps widget logic reusable across host applications and avoids coupling core crates to wgpu, egui, or another UI toolkit.

## Slice 01 - horizontal rule

Status: **implemented; macOS manual acceptance PASS; Windows manual acceptance deferred to the Phase 6 integration pass**

Behavior:

- CommonMark/GFM `SyntaxKind::Rule` becomes `WidgetKind::HorizontalRule` while inactive.
- Only the rule marker bytes are concealed; its line ending is preserved so block geometry does not collapse.
- The projected widget occupies the collapsed projected anchor for that marker.
- The native IME lab paints a horizontal line through the preserved layout row.
- Caret/selection/composition interaction with the source range disables the widget and reveals the Markdown marker.
- Source-visible/raw-fallback projection never invents a widget.

Regression coverage:

- inactive rule marker is removed from projected text
- line-break count is preserved
- projected widget range is collapsed
- caret inside the rule reveals source and removes the rendered widget
- Phase 5 IME/navigation tests remain in the workspace suite

## Manual acceptance

Run:

Windows:

```powershell
$env:MDEDIT_LIVE_PREVIEW_ACCEPTANCE = "1"
cargo run -p mdedit-ime-lab --release
```

macOS:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

In the `Phase 6 widget scratch` section verify:

- inactive `---` is replaced by a native horizontal line
- the blank layout row remains in document flow
- clicking or navigating onto the rule reveals `---`
- leaving the rule renders the line again
- Up/Down and Left/Right can cross the rule line
- selection across the rule remains source-safe
- Korean/Japanese/Chinese IME behavior from Phase 5 remains unchanged

## Platform acceptance strategy

- macOS: manual acceptance after each widget slice or substantial layout substep
- Windows: fmt/check/clippy/test CI after each slice
- Windows native GUI/IME/manual acceptance: deferred and batched at the end of Phase 6
- Any Windows-only failure found in the final pass is fixed before Phase 6 is closed

The final Windows pass will cover widgets plus the Phase 5 IME, navigation, selection, reflow, undo/redo, mouse, and DX12 regressions in one fixture.

## Slice 02 - task checkbox

Status: **implemented; Windows/macOS/Ubuntu CI PASS; macOS manual acceptance PASS; Windows native manual acceptance deferred to the Phase 6 integration pass**.

Behavior:

- GFM `TaskListMarker { checked }` becomes `WidgetKind::TaskCheckbox` with `WidgetPayload::TaskCheckbox { checked }` while inactive.
- Exact `[ ]` / `[x]` marker bytes are canonical and source-mapped.
- Inactive task markers collapse to projected widget anchors.
- Native rendering draws an unchecked box or checked indicator in the left gutter without changing source text.
- Clicking the checkbox emits `WidgetAction::SetTaskChecked`.
- The host resolves that action through `EditorInput::WidgetReplace` and `TransactionKind::Widget`.
- Widget replacement preserves unrelated caret/selection positions and is one isolated undo/redo transaction.
- Keyboard/caret interaction with the marker reveals its Markdown source for direct editing.
- Trace/replay records widget replacements explicitly.

Manual acceptance on macOS:

- unchecked and checked items render differently
- clicking unchecked changes source to `[x]`
- clicking checked changes source to `[ ]`
- undo restores the previous state in one step
- redo reapplies it in one step
- caret/selection does not jump when a checkbox is clicked
- arrow navigation across task lines remains normal
- IME behavior remains unchanged

## Slice 03 - image

Initial standalone-image slice status: **Windows/macOS/Ubuntu CI PASS; macOS manual acceptance PASS; Windows native manual acceptance deferred to the Phase 6 integration pass**.

The macOS acceptance pass covered native texture rendering, loading-to-ready, mouse/keyboard source reveal, repeated reveal/conceal, inline-source policy, missing-resource fallback, Horizontal Rule, Task Checkbox + undo/redo, navigation, selection, scroll/reflow, Korean/Japanese/Chinese IME, and `**한글**`.

### Image resource/render contract

- standalone CommonMark/GFM image syntax is projected as `WidgetKind::Image`
- destination/title/alt text are preserved in `ImageWidgetData`
- entering the image source range reveals canonical Markdown
- inline images remain source-visible until an inline-width reservation contract exists
- the IME lab owns resource resolution; `mdedit-live` performs no filesystem/network I/O
- the IME lab resolver is sandboxed to its configured image base directory
- remote/data/file URIs are intentionally delegated to a future host resolver
- local image decode runs off the UI thread
- decoded RGBA data is uploaded to a native wgpu texture
- while loading or on failure, a native one-row placeholder is drawn

### Image block-height reservation / reflow

Status: **implemented; Windows/macOS/Ubuntu fmt/check/clippy/test PASS at the last accepted code baseline; macOS manual acceptance BLOCKED only on direct observation of loading → ready; Windows native manual acceptance deferred to the Phase 6 integration pass**.

Decoded standalone images now reserve real document height rather than being squeezed into one text row.

Design:

- decoded intrinsic dimensions are converted to a block layout with aspect ratio preserved
- images are not upscaled above intrinsic size
- native block width is capped at 720 px and available editor width
- native block height is capped at 360 px
- vertical padding is included in the reserved height
- because cosmic-text uses a uniform line height, the host reserves block height with synthetic layout-only newline rows
- synthetic rows never modify canonical Markdown or `mdedit-live::Projection`
- `ReservationOverlay` maps projected offsets through synthetic rows and maps hits inside the reserved block back to the collapsed image source
- IME composition mapping is layered after reservation mapping, so source → projected → reserved layout → preedit display remains explicit
- reflow now records before/after **display** offsets for the same canonical source anchor, allowing decode/reveal/resize changes to preserve viewport position
- image geometry and all later widgets resolve through canonical source → final display mapping, so widgets after a tall image remain aligned
- partially visible images are clipped to the text viewport
- loading/failure placeholders stay one row and do not reserve a large block before a successful decode
- resizing the editor can recompute the number of reserved rows without changing source

Regression coverage includes:

- synthetic image rows increase display layout height without changing canonical source/projection
- hits inside synthetic rows map back to the image source
- adding a reservation produces same-source reflow offsets
- fixture aspect ratio is preserved
- decoded fixture reserves multiple rows
- widget geometry after reservations uses source-to-display mapping
- Phase 5 IME/navigation and earlier Phase 6 widget tests remain enabled

### Block-height macOS acceptance result

Manual run on macOS 15.7.5 / Apple M1 at `60065522`:

- **PASS**: the standalone fixture image occupies several text rows instead of a one-row thumbnail
- **PASS**: text below the image stays below the reserved block during resize and scroll
- **BLOCKED / NOT TESTED**: the exact loading → ready transition happened before it could be observed
- **PASS**: mouse and arrow-key source reveal
- **PASS**: preview restoration and five reveal/conceal repetitions without unexpected viewport jump
- **PASS**: lower viewport clipping, inline-source policy, missing-image one-row fallback, crash/panic and wgpu error checks
- **PASS**: Horizontal Rule, Task Checkbox + undo/redo, arrow navigation, selection, scroll/reflow, Korean/Japanese/Chinese IME, and `**한글**`

The block-height slice therefore remains **BLOCKED**, not failed. The only unresolved acceptance item is direct observation of viewport/caret stability during the one-row Loading placeholder → multi-row Ready block transition.

### Deterministic loading → ready acceptance mode

The IME lab supports an opt-in acceptance delay:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 \
MDEDIT_IMAGE_READY_DELAY_MS=3000 \
cargo run -p mdedit-ime-lab --release
```

When `MDEDIT_IMAGE_READY_DELAY_MS` is non-zero:

- only a successfully decoded image's transition to Ready is delayed; missing/failed resources are not intentionally delayed
- the value is capped at 30 seconds
- the initial caret is placed at the sentence immediately below the standalone fixture image so the relevant block is visible immediately
- normal runs remain unchanged because the default delay is zero

For the focused retest verify:

- the first visible state is the one-row Loading placeholder
- after the delay it changes to the full multi-row native image block
- the caret remains on the same logical text position
- the viewport does not jump unexpectedly
- the sentence below the image moves down with the reserved block rather than overlapping it

### Block-height macOS acceptance target

- the standalone fixture image occupies several text rows instead of a one-row thumbnail
- text after the image is pushed below the reserved block and never overlaps it
- decode transition from one-row placeholder to full block does not jump the viewport/caret
- clicking or navigating into the image block reveals the Markdown source
- leaving the source restores the multi-row image block without a viewport jump
- repeated reveal/conceal stays stable
- scrolling through a partially visible image clips it to the editor viewport
- Horizontal Rule / Task Checkbox geometry stays aligned after the image
- selection across the block stays source-safe
- inline images remain source-visible
- missing resources remain a one-row safe placeholder
- Korean/Japanese/Chinese IME and `**한글**` remain regression-free

After this block-height substep passes macOS acceptance, the next Phase 6 widget is **math**.
