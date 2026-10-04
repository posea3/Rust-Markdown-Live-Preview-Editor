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

Status: **implemented; CI/native acceptance pending**

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

## Next widget

After horizontal-rule acceptance: **task checkbox**.
