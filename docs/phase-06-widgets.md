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

### Resource limits and upload failures

- local image headers are checked against the active device's maximum 2D texture dimension before pixel decoding
- encoded files, decoded pixel data, and RGBA uploads each have a 64 MiB resource budget
- the shared image/math upload path rejects zero/oversized dimensions and mismatched RGBA buffers before creating a GPU texture
- rejected image and math uploads enter their failure state and retain canonical Markdown; they do not become Ready or retry every frame
- automated regression coverage includes device-limit boundaries, invalid buffers, byte-length overflow, decode limits, and normal fixture decoding

The native GPU regression is opt-in because CI runners may not expose a graphics adapter:

```bash
cargo test -p mdedit-ime-lab gpu_upload_rejects_oversized_image_and_accepts_next_valid_image -- --ignored --nocapture
```

This regression passed on Windows with Intel UHD Graphics 630 / DX12: an image wider than the device limit was rejected, and the next valid image uploaded successfully. The broader Windows GUI/IME and macOS integration retests remain pending as recorded below.

### Image block-height reservation / reflow

Status: **implemented; Windows/macOS/Ubuntu fmt/check/clippy/test PASS; macOS manual acceptance PASS; Windows native manual acceptance deferred to the Phase 6 integration pass**.

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

Final deterministic delayed retest at `426b673` directly observed the one-row Loading placeholder → multi-row Ready block transition. The initial placeholder, transition to the decoded 96×48 image, canonical caret stability, unchanged `scroll=43:21`, viewport stability, and text-below-block placement all **PASS**. The Image block-height/reflow slice is therefore closed on macOS.

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

## Slice 04 - math

Status: **implemented; Windows/macOS/Ubuntu CI PASS; macOS Display Math, checkbox-history viewport, and CJK native IME acceptance PASS; Windows native manual acceptance deferred to the Phase 6 integration pass**.

### Display Math contract

- parser math support is enabled through the project-owned Markdown dialect and preserves exact source ranges plus TeX content for `SyntaxKind::DisplayMath` and `SyntaxKind::InlineMath`
- inactive display math becomes `WidgetKind::DisplayMath` with a canonical source range and `MathWidgetData` payload
- the full `$$...$$` source range is concealed only while the widget is inactive
- entering the display-math source range with caret, selection, or composition reveals the original Markdown source
- inline math remains source-visible until an inline-width reservation contract exists
- the native IME lab renders TeX through the pure-Rust `latex-rust` path; no JavaScript, HTML, WebView, or browser math renderer is used
- math rendering runs off the UI thread and uploads decoded RGBA output through the existing native texture host
- loading/missing math uses a one-row native placeholder
- invalid TeX fails safely to a native failure placeholder without mutating canonical source
- successful display math reserves native block height through the same source-mapped reservation overlay used by standalone images
- small formulas are enlarged to a legible minimum height when space permits
- rendered math is capped to the available editor width / 720 px and 240 px height, then centered within the reserved block
- resize/reveal/conceal recompute reservation geometry without inserting synthetic bytes into canonical Markdown or `mdedit-live::Projection`

### Regression coverage

Automated coverage includes:

- exact parser source range and content metadata for inline/display math
- inactive display math projects to a collapsed native widget with the expected TeX payload
- caret interaction reveals canonical `$$...$$` source
- inline `$...$` remains source-visible
- native TeX rendering produces non-empty RGBA output
- invalid TeX returns an error rather than a fake render
- math texture keys are namespaced
- block layout enlarges small renders for legibility, reserves multiple rows, and caps oversized renders
- loading/failure placeholder remains within one text row
- earlier image, task-checkbox, horizontal-rule, Phase 5 navigation, selection, reflow, and IME tests remain enabled

CI at branch head `e3db3db` completed successfully on macOS, Windows, and Ubuntu (workflow run `40172991191`).

### macOS manual acceptance

Run:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

In **Math widget scratch**, verify:

- `$$\\frac{a}{b} + \\sqrt{x} = \\sum_{i=1}^{n} i$$` renders as a native centered math block while inactive
- the `$$...$$` source is not visible while the widget is inactive
- the sentence below the formula stays below the full reserved block and never overlaps it
- clicking or navigating into the math block reveals the exact Markdown/TeX source
- leaving the source range restores the native math block
- repeat reveal/conceal at least five times and confirm no unexpected viewport jump
- resize the window narrower/wider and confirm the block remains centered, bounded, and source-safe
- selection across the math block remains stable and arrow navigation can cross it
- inline math `$x^2 + y^2$` remains editable source text
- invalid display math `$$\\definitely_unknown_command{x}$$` fails safely without crash/panic or source corruption
- Horizontal Rule, Task Checkbox + undo/redo, and standalone Image still render and interact correctly
- Korean/Japanese/Chinese IME and `**한글**` remain regression-free
- no wgpu validation error or panic appears in the terminal

### macOS acceptance result

Display Math visual acceptance passed on macOS 15.7.5 / Apple M1. A checkbox-history viewport regression was then found during integration: Undo could restore the checkbox state while moving a manually scrolled viewport back to the document start.

The first mitigation only disabled caret-follow when history preserved the selection, but a focused retest at `6c97d4c` reproduced the jump three times from approximately `scroll=64:21` to `scroll=0:0`. Redo preserved the viewport. Korean, Japanese, and Chinese native IME input all passed at that head, including Korean `**한글**`, with no viewport movement.

The regression was fixed on `feature/phase-06-widget-history-viewport-fix` by preserving the exact pre-history cosmic-text `Scroll` whenever Undo/Redo leaves the selection unchanged, then restoring that viewport after projection/layout rebuild. Ordinary history that restores a different selection keeps existing caret-follow behavior.

TDD and automated verification:

- RED: test-only commit `61d3a27`; temporary PR #10 failed as expected before the history viewport-anchor helpers existed
- GREEN: implementation branch CI passed on Windows, macOS, and Ubuntu
- Rust 1.98 Clippy compatibility issues reported during acceptance were also removed without changing the Rust 1.90 project baseline

Focused macOS retest at `55a0556722a152605abe9edf14950345a1a46a60`:

- **PASS**: `cargo fmt --all -- --check`
- **PASS**: `cargo check --workspace --all-targets`
- **PASS**: `cargo clippy --workspace --all-targets -- -D warnings`
- **PASS**: `cargo test --workspace` — 201 passed, 0 failed, 1 ignored
- **PASS**: document-start caret + manually scrolled `scroll=64:21`, three checkbox toggle/Undo/Redo cycles with viewport unchanged
- **PASS**: checkbox-near caret, three toggle/Undo/Redo cycles with viewport unchanged
- **PASS**: checkbox state restoration on every Undo/Redo transition
- **PASS**: Korean 2-set `한글` preedit/commit
- **PASS**: Korean `**한글**` rendering
- **PASS**: IME viewport stability at `scroll=19:0`
- Japanese and Chinese native IME retain the prior `6c97d4c` PASS result

Phase 6 macOS integration for slices 01-04 is therefore **PASS**. Proceed to **Slice 05 - table**. Windows native GUI/IME/manual acceptance remains batched for the final Phase 6 integration pass.



## Slice 05 - table

Status: **implemented; Windows/macOS/Ubuntu CI PASS; macOS Table acceptance PASS including non-overlapping block reservation and stable reveal/conceal viewport; Windows native manual acceptance remains deferred to the final Phase 6 integration pass**.

### Table contract

- GFM/Obsidian table syntax remains canonical Markdown source
- an inactive `SyntaxKind::Table` becomes `WidgetKind::Table` with a `TableWidgetData` payload
- the payload preserves column alignment plus header/body visible cell text
- the complete table source range is concealed only while the table widget is inactive
- entering the table source range reveals canonical Markdown for editing; leaving restores the native table
- when an inactive table owns a range, nested delimiter concealment and structural padding inside that range are shadowed so strong/link/code syntax cannot force an overlapping-concealment RawFallback
- inline Markdown inside a native table cell is flattened to its visible text for this slice; canonical markers remain intact and reappear on source reveal
- the native IME lab reserves one visual text row per header/body table row through the existing source-mapped reservation overlay
- table width is centered and capped to the available editor width / 720 px
- columns partition the native table block without overlap
- the header row receives a native background; outer and inner grid lines use the existing native rectangle renderer
- cell text is rendered through native cosmic-text/glyphon buffers, clipped to its cell, with GFM Left/Center/Right alignment applied
- long-cell wrapping and rich inline styling inside the native table are not introduced by this slice; source reveal remains lossless

### TDD / regression coverage

RED → GREEN cycles covered:

1. **Projection/data contract**
   - RED: missing `WidgetKind::Table` / `ProjectedWidget::table()`
   - GREEN: inactive table widget payload, alignment/header/row data, source reveal
2. **Native geometry/reservation**
   - RED: missing table block/cell geometry
   - GREEN: block-height reservation, centered width cap, non-overlapping cell partition, native grid/header geometry
3. **Cell text/alignment**
   - RED: missing `table_cell_layouts`
   - GREEN: header/body text, header role, Left/Center/Right alignment, native glyphon/cosmic-text cell text areas
4. **Nested inline Markdown**
   - RED: strong/link/code inside a table caused the whole table projection to fall back because nested concealment overlapped the table-owned range
   - GREEN: inactive widget ownership shadows internal concealment; native cells receive visible text without Markdown marker bytes

Latest code CI completed successfully on Windows, macOS, and Ubuntu before the acceptance-fixture/documentation-only commits.

### macOS manual acceptance

Run:

```bash
git fetch origin
git switch feature/widgets-phase-06-table
git pull

cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

In **Table widget scratch**, verify:

- the first 3-column table is a native grid while inactive; Markdown pipes and separator markers are not visible
- header/body text appears inside the correct cells
- `Name` is left aligned, `Score` is right aligned, and `State` is centered
- `Ada / 42 / ready` and `한글 / 7 / native` stay in their expected columns
- the sentence after the table stays below the full table block without overlap
- the second table displays `strong link code` as visible cell text without exposing `**`, link destination syntax, brackets, or backticks
- clicking or arrow-navigating into either table reveals the exact canonical Markdown table source
- leaving the source range restores the native table
- repeat reveal/conceal at least five times and confirm no unexpected viewport jump
- resize narrower/wider and confirm the table remains bounded/centered and cell boundaries do not overlap
- selection and arrow navigation can cross the table safely
- Horizontal Rule, Task Checkbox + Undo/Redo viewport preservation, Image, and Display Math remain regression-free
- Korean `한글` native IME input and `**한글**` still keep the viewport stable
- no wgpu validation error, panic, or crash appears

Keep PR #11 draft until this macOS acceptance passes. Windows native GUI/IME/manual acceptance remains batched for the final Phase 6 integration pass.


### macOS acceptance result — first pass FAIL, reservation fix ready for focused retest

First macOS acceptance at `54e048d9da2927a088cbc244bce270b35d235c37`:

- **PASS**: `cargo fmt --all -- --check`
- **PASS**: `cargo check --workspace --all-targets`
- **PASS**: `cargo clippy --workspace --all-targets -- -D warnings`
- **PASS**: `cargo test --workspace` with 0 failures and the native-graphics-adapter test ignored
- **PASS**: native grid, header/body text, Left/Right/Center alignment, both data rows
- **PASS**: inline Markdown cell displayed `strong link code` without source markers
- **PASS**: canonical table source reveal and native-grid restore
- **PASS**: Rule/Image/Math and Checkbox Undo/Redo smoke
- **PASS**: Korean preedit/commit and `**한글**` with stable `scroll=23:5`
- **FAIL**: surrounding explanatory text overlapped the native grids
- **FAIL**: repeated reveal/conceal moved the viewport between approximately `scroll=75:21` and `scroll=79:21`
- narrow-width drag acceptance was not completed because the macOS window edge drag did not resize during that run
- a macOS LaunchServices connection warning was printed, but no wgpu validation error, panic, or crash was observed

Root cause:

- `update_block_widget_reservations()` correctly created Table reservations
- `ReservationOverlay::apply()` accepted only Image and DisplayMath widget kinds, so Table reservation rows were silently ignored
- the grid was therefore painted at multi-row height while the text layout still occupied the collapsed one-row position, causing overlap
- reveal then restored the full Markdown table lines without corresponding inactive reservation rows, explaining the multi-line reflow jump

TDD regression:

- RED commit `d310478b92ae5e64b11c8c6da0ed75bccfe45ea0`
- `table_block_reservation_adds_layout_rows_without_changing_source_mapping` failed because the displayed newline count was 3 instead of the expected 5
- `adding_table_reservation_produces_reflow_offsets_for_same_source_anchor` failed because the after offset stayed 14 instead of advancing to 16
- GREEN commit `50e8f5947c01afa1bdc985dd4de7d9e2f3cd1df0`
- minimal production change: include `WidgetKind::Table` in the existing reservation-overlay widget-kind filter
- CI run `40382566358`: Windows, macOS, and Ubuntu all PASS for fmt/check/clippy/test

The macOS window is created with the default resizable winit attributes; there is no `with_resizable(false)` or fixed minimum-size setting in the IME lab. The failed edge-drag observation is therefore kept as an acceptance item rather than treated as part of this reservation fix.

Focused macOS retest should verify:

- neither table overlaps the explanatory text above/below it
- the sentence after each table remains below the complete native grid
- reveal/conceal five times no longer produces the previous four-line viewport jump
- canonical source reveal/restore, alignment, inline-marker concealment, and Korean IME remain smoke-PASS
- if window edge resize works in the retest, narrow/wide table bounding can be closed; otherwise keep only that resize check open separately
- no wgpu validation error, panic, or crash appears

Keep PR #11 draft until this focused macOS retest passes.


### macOS focused retest — overlap PASS, two-line reveal footprint drift remained

Focused macOS retest at `98427c32fac9cc8d4491fe47825ea579397848dd`:

- **PASS**: both native tables showed all rows without overlapping explanatory text
- **PASS**: `Name` Left, `Score` Right, `State` Center
- **PASS**: inactive inline-Markdown cell displayed only `strong link code`
- **PASS**: canonical Markdown structure revealed inside the table; placing the caret in `strong` revealed the `**` markers
- **PASS**: Image and Math smoke
- **PASS**: Korean preedit/commit and `**한글**` with stable `scroll=41:5`
- **NOT RETESTED**: Checkbox click toggle, workspace fmt/check/clippy/test, narrow-window resize
- **FAIL**: first table reveal/conceal repeatedly moved `scroll=76:21 <-> 78:21`
- **FAIL**: second table also moved around `78:21 -> 80:21 -> 77:21`
- no wgpu validation error, panic, or crash; one macOS IMK system warning was observed

Second root cause:

- the first reservation fix made Table reservations actually participate in layout, removing overlap and reducing the previous four-line movement
- however Table reservation size was still derived from **native visual rows**: header + body rows
- canonical Markdown additionally contains the GFM alignment separator row and the Table source range conceals its newline bytes
- for the first fixture table, native rendering has 3 visual rows while the canonical source range conceals 4 newline bytes
- using `visual_rows - 1 == 2` synthetic lines therefore did not preserve the same vertical source footprint across conceal/reveal

TDD:

- RED commit `bbe31b632f75a9c0361cab658dbee505340b5e09`
- the new test requires the first fixture table's reservation to equal the **4 concealed source newline bytes**, independently of the 3 native visual rows
- macOS CI failed as expected because `table_reservation_extra_lines` did not exist
- GREEN implementation began at `c0b2782`; CI then caught a borrow/type error before behavior verification
- corrected GREEN commit: `a6bfe889e7162df546d43f95f1e07e3dcc449f79`
- Table now counts `\n` inside its canonical `source_range` and uses that exact count for reservation
- Image and DisplayMath keep their existing `rows.saturating_sub(1)` policy unchanged
- CI run `40483731958`: Windows, macOS, and Ubuntu all PASS for fmt/check/clippy/test

Next focused macOS retest:

- confirm both tables still have no text/grid overlap
- repeat first-table reveal/conceal at least five times and confirm the previous `76:21 <-> 78:21` two-line movement is gone
- repeat the second table and confirm it no longer walks among `78:21 / 80:21 / 77:21`
- confirm canonical source reveal/restore and `strong` marker reveal still work
- smoke Image/Math/Checkbox and Korean IME
- narrow-window resize remains separately open if edge drag is still unavailable
- no wgpu validation error, panic, or crash

Keep PR #11 draft until the reveal/conceal viewport retest passes.


### macOS final focused retest — PASS

Final macOS focused retest at `e433b27fc0bc45b4a49ebb63c6abe6f3e29a12a8`:

- **PASS**: first table canonical source reveal and native-grid restore
- **PASS**: after reaching the stable baseline `scroll=78:21`, five reveal/conceal cycles remained `78:21 -> 78:21`
- **PASS**: second table canonical source revealed `**strong**`, link, and code markers correctly
- **PASS**: second table returned to native grid after leaving the source range
- **PASS**: five second-table reveal/conceal cycles remained `79:21 -> 79:21`
- **PASS**: both tables remained separated from surrounding explanatory text
- **PASS**: no abnormal blank vertical space appeared around either table
- **PASS**: Image preview and following text remained separated
- **PASS**: Math block and following text remained separated
- **PASS**: Checkbox state could be toggled and restored during smoke verification
- **PASS**: Korean preedit/commit and `**한글**` kept `scroll=45:21`
- test input was undone back to the fixture baseline `source=3443 bytes`
- logs showed Metal initialization and the expected invalid-Math fallback only; no wgpu validation error, panic, or crash was observed

The two Table regressions are therefore closed:

1. missing Table reservation application caused grid/text overlap and the original multi-line reflow jump
2. native visual-row-based reservation did not preserve the concealed canonical source footprint, causing the remaining two-line reveal/conceal drift

The final reservation contract is:

- Image and DisplayMath keep their existing render-height-derived reservation policy
- Table reserves exactly the newline count concealed by its canonical source range
- inactive and revealed Table states therefore retain the same vertical source footprint while source remains canonical

Slice 05 — Table is **macOS PASS** and can be merged. Narrow-window edge-drag acceptance was not required to close this regression because the widget remained bounded in the exercised window states and the IME lab window is configured as resizable by default. Windows native GUI/IME/manual acceptance remains batched for the final Phase 6 integration pass.

## Slice 06 - callout / custom embed

Status: **integrated through PR #12 at `064bc40`; macOS focused acceptance and long-label clipping PASS; final PR Windows/macOS/Ubuntu CI PASS; Windows native manual acceptance remains deferred**.

### Contract

- Obsidian extension scanning remains opt-in and feeds an `ExtensionSnapshot` into Live Preview projection
- an inactive callout owns its complete containing blockquote source range
- Callout payload preserves type, custom title, fold marker, and visible body text
- nested Markdown markers in the inactive body collapse to visible text while canonical source remains unchanged
- the native host draws a type-colored background, accent bar, divider, title/fold text, and wrapped body text
- Callout reservation counts canonical newline bytes in the owned source range so reveal/conceal preserves the vertical footprint
- an inactive standalone `![[...]]` becomes a native one-row Custom Embed card
- Custom Embed payload preserves target, path, fragment, and display text
- inline embeds remain source-visible until an inline-width reservation contract exists
- entering either owned range reveals exact canonical Markdown; leaving restores the native widget
- callout/embed text is clipped to widget bounds and never paints over following text

### macOS acceptance

Run:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

In **Callout and custom embed widget scratch**, verify:

- warning callout renders as a native card with a distinct accent and the custom title
- body displays `Strong body with link and code` plus the Korean line without Markdown marker bytes
- callout never overlaps the text before or after it
- clicking or arrow-navigating into title/body reveals the full canonical blockquote and callout marker
- leaving restores the card; repeat five times with no viewport jump
- `Architecture · #Overview` and `missing-note` appear as native one-row embed cards
- the additional long-label embed remains clipped to its one-row card at narrow widths, with the following sentence below the card
- entering each standalone embed reveals exact `![[...]]` source and leaving restores the card
- the inline embed remains source-visible
- resize wider/narrower keeps cards bounded and text clipped
- Rule, Checkbox + Undo/Redo, Image, Math, and Table remain regression-free
- Korean `한글` and `**한글**` keep the viewport stable
- no wgpu validation error, panic, or crash appears

PR #12 was merged after the focused macOS results and final-head CI passed; measured scope and limitations are recorded below.

### Recorded macOS focused retest

Code head: `d834fe8e44e422d3ea090b94fb5bcff2dd85f143`.
These results were observed in the macOS GUI during the 2026-10-28 acceptance session; they are not new runs inferred from automated tests.

| Check | Observed result |
| --- | --- |
| First Callout source → card transition | `scroll=91:11` → `scroll=91:11` |
| Five subsequent Callout reveal/conceal cycles | `scroll=91:11` throughout |
| Both checkboxes, center and near-border clicks | Native checked/unchecked state and canonical `[x]` / `[ ]` changed together |
| Single-step checkbox Undo / Redo | State restored/reapplied; `scroll=68:16` throughout |
| Resize wide → narrow → wide | Callout and both standalone embeds stayed within the editor; title, fold marker, body, and following text did not overlap |
| Five Callout reveal/conceal cycles after resizing | `scroll=105:0` throughout |
| Terminal diagnostics | No wgpu validation error, panic, or crash observed |

The old fixture contained only `Architecture · #Overview` and `missing-note`, so this run did not establish long-label clipping. The acceptance fixture now includes a deliberately long English/Korean embed label and a following sentence to make that check reproducible.

Local fmt/check/clippy/workspace tests passed in that session. The graphics-adapter-dependent regression remained ignored in the default workspace test run. Cross-platform CI is a separate gate: [run `40599668706`, attempt 2](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/40599668706/attempts/2) for `d834fe8` originally awaited maintainer approval rather than failing a test; after approval on 2026-11-26, Windows/macOS/Ubuntu checks all passed.

### Long-label macOS closeout check — 2026-11-26

The renderer code remains at `d834fe8`; only the acceptance fixture and documentation changed for this check. A new release build loaded the expanded fixture at `source=4560 bytes`.

- **PASS**: at approximately 1920 px and 500 px window widths, the long embed label was clipped to its one-row card. No label text painted past the card or over the following sentence.
- **PASS**: entering the long embed revealed its full canonical source, including the final `한글 레이블]]`; leaving restored the native card. Source length stayed at 4560 bytes.
- **PASS**: Callout title/fold/body and the three embed cards remained within the editor during wide → narrow → wide resizing, with surrounding text below the owned blocks.
- **PASS**: after resizing to approximately 900 px, the first observed Callout source → card transition and five additional reveal/conceal cycles all preserved `scroll=112:21`. Full Callout source markers were visible together.
- **PASS**: the inline embed stayed source-visible.
- Local fmt/check/clippy/workspace tests and release build passed: **230 passed, 0 failed, 1 ignored** in the default workspace suite.
- The GUI used Apple M1 / Metal. Logs contained the expected invalid-Math fallback only, with no wgpu validation error, panic, or crash observed.

The long embed's wrapped source has a different visual height from its one-row card. At approximately 500 px the measured reveal/conceal scroll sequence was `120:21 → 123:52 → 120:21`; the sentence below stayed separated from the source/card. This establishes clipping and source preservation, not an unchanged-scroll guarantee for soft-wrapped embed source. CJK IME and the full earlier widget acceptance suite were not rerun for this fixture/documentation-only change.

## Final Windows native integration acceptance

Status: **Windows native GUI/IME checklist and fresh Windows/macOS/Ubuntu CI passed for acceptance PR #14 (`b04dce1`) on 2026-11-27; patch integration remains pending**.

Use the integrated revision being considered for platform acceptance and record the full commit SHA, Windows version, GPU/driver, and selected wgpu backend with the result. If this check is performed after later-phase changes, test that then-current integrated revision rather than relying on the earlier Phase 6 baseline alone.

```powershell
git rev-parse HEAD
$env:MDEDIT_LIVE_PREVIEW_ACCEPTANCE = "1"
cargo run -p mdedit-ime-lab --release
```

- Verify native Rule, both Checkbox states, Image, Display Math, Table, Callout, and standalone Custom Embed rendering.
- Enter and leave each widget through mouse and arrow navigation; verify canonical source reveal and native restoration without an unexpected viewport jump.
- Check text above/below Image, Math, Table, and Callout for overlap; repeat reveal/conceal five times and record `scroll=` before/after.
- With the caret at document start, scroll to each checkbox without moving the caret; test center/edge clicks plus one Undo and one Redo. Repeat with the caret near the checkbox and verify viewport preservation.
- Resize wide → narrow → wide; verify card/grid bounds and long-label clipping, then repeat Callout reveal/conceal.
- Check selection across widgets, navigation across logical lines and viewport boundaries, scrolling, clipboard, and ordinary text Undo/Redo.
- Verify native Korean preedit/commit and `**한글**`, Japanese conversion/commit, Chinese candidate selection/commit, and focus loss/refocus. Record viewport stability during composition.
- Confirm DX12 startup and absence of wgpu validation errors, panic, or crash. Record any `WGPU_BACKEND` override rather than treating an alternate backend as the default-path result.
- Undo temporary acceptance edits and record the final source baseline.

Phase 6 implementation is integrated. Windows native testing subsequently passed with local host fixes; the unchanged upstream head does not inherit that result until those fixes are integrated and verified. On 2026-11-26 the user explicitly authorized starting Phase 7 on macOS with Windows verification deferred. Phase 7 hardens the extension API; its scanner tests are independent of native acceptance. Narrator/VoiceOver acceptance remains separately deferred.

### Windows native results — 2026-11-26–27

Baseline: `87a5ffac6fd83777ebb4310320cdff8a2cf470ec` (PR #13), including the integrated Phase 6 base `064bc40130f9a1c61adaab1168180d791e5b1416`. Native testing exposed defects in the lab's CRLF shaping and widget layout synchronization. The results below include local fixes in `examples/ime-lab/src/main.rs`; they must not be attributed to the unchanged PR #13 head.

Final source-file SHA-256: `4786871b21b0a4f8b6c4870c66413d8e83ea6a30ee590cd29f2de01815ed7ccb`. Release executable SHA-256: `98be7ee39c1fd8d865543d4939f81605788d19e110ec37f8f3f89c54cfcea137`.

Environment: Windows 11 Home `10.0.26200`, x64; Rust/Cargo `1.90.0`; NVIDIA GeForce GTX 1060 3GB, driver `32.0.15.6094`. Startup selected **DX12 / FIFO** with no `WGPU_BACKEND` override. Microsoft Korean, Japanese, and Simplified Chinese input methods were used. The fixture in this checkout has CRLF endings and is **4707 bytes** (4560 bytes with LF endings).

#### Defects reproduced and corrected

- CR bytes were included in shaped line text by the in-place update path. Clicking an empty projected widget row could therefore resolve inside its CRLF ending and enter the following blank line. Rule, missing-note, and long-label embed clicks now resolve into their canonical source ranges.
- The preedit fallback used a different paragraph parser, producing extra rows for CRLF and visibly moving the Korean scratch area during the second syllable. Rebuild and in-place paths now share LF-based row boundaries, preserve CRLF endings separately, and apply preedit attributes using canonical byte offsets. An LF followed by an empty CRLF row is also preserved as two boundaries.
- Pointer/visual-navigation layout refreshes could consume geometry before block-widget reservations were restored. Near the lower viewport edge, table concealment moved the viewport backward from `99:50` to `96:26`. Rendering, hit testing, checkbox activation, and visual navigation now refresh the same reserved layout. The focused retest changed `99:46` to `99:55`, matching the existing small downward drag-margin scroll rather than jumping backward several logical rows.

Four new regression tests were observed failing before their corresponding fixes, then passing. They check shaped CRLF text/endings, empty-row hit offsets, fallback row/attribute/visible-content consistency, and mixed LF/CRLF boundaries.

#### Automated verification

| Check | Local result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --locked --workspace --all-targets` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked --workspace` | **247 passed, 0 failed, 1 ignored**; baseline was 243 passed |
| `cargo build --locked --release -p mdedit-ime-lab` | PASS |
| Native DX12 oversized-image rejection followed by valid-image upload | **1 passed**, separately from the default suite; GPU module unchanged by the final layout refactor |
| Public `extension_scanners` example | PASS; Korean reference, `#42`, and protected-inline-code diagnostic observed |

The baseline [PR CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/45743140198) and [push CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/45743096065) passed on Windows, macOS, and Ubuntu. Those runs do **not** cover the subsequent acceptance patch. Its separate [PR #14](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/14) received fresh [CI run 45976582080](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/45976582080): **Windows, macOS, and Ubuntu all passed** for `b04dce1d24147cd1bbc86dfaae7223c5a42e2066`.

#### Observed native behavior

The initial checklist was run on the baseline; failures were reproduced and corrected, then affected cases were repeated on the fixed builds. The final widget-layout build received the table, selection/clipboard, resize, Callout, embed, grapheme, viewport-boundary, Korean/Japanese/Chinese IME, and focus loss/refocus retests recorded here.

| Case | Observed result |
| --- | --- |
| Rule mouse/arrow entry and exit | Canonical `---` revealed; click at source offset 1753 is within the owned 1750..1753 range |
| Both checkboxes; center/edge clicks with remote and nearby caret | All eight click cases changed native/canonical state together; single-step Undo/Redo restored/reapplied each toggle without viewport movement |
| Image loading → ready | Directly observed with delayed loading; `scroll=66:19`, source selection 2193, and following-text separation preserved |
| Five Image reveal/conceal cycles | `scroll=75:19` throughout on the CRLF-fixed build |
| Five Display Math reveal/conceal cycles | `scroll=86:19` throughout; invalid TeX used the expected safe fallback |
| Both tables, five mouse reveal/conceal cycles each | Final build preserved `scroll=108:24`; alignment, Korean cells, and native restoration verified |
| Selection and clipboard across a table | Full inline markers became visible; copied 92 bytes exactly matched canonical bytes 3313..3405, including CRLF; paste/Undo/Redo/Undo restored source length 4707 |
| Wide → 493 px → 1920 px | Grids/cards stayed bounded; Callout title/fold/body and following text remained separated; long label clipped to one row |
| Five Callout reveal/conceal cycles after resizing | Final build preserved `scroll=119:0` throughout |
| Architecture, missing-note, and long standalone embeds | Mouse/arrow entry exposed exact source; leaving restored native cards; inline embed stayed source-visible |
| Long embed at 493 px | Full source retained final `한글 레이블]]`; reveal/conceal measured `128:22 → 132:22 → 128:22`, reflecting wrapped source height |
| Unicode and viewport navigation | One Shift+Right selected the complete 25-byte family ZWJ grapheme; Down across the lower viewport edge kept the caret visible |
| Korean, Japanese, Chinese, and focus loss/refocus | PASS on the final layout build at 493 px; `scroll=41:0` throughout |

On the final layout build, Korean `**한글**` kept `scroll=41:0` throughout. With the opening `**` inserted, preedit left source at 4709 bytes; committing `한` raised it to 4712 while `글` remained preedit. Closing markers committed the remaining syllable and produced the exact 4717-byte document; leaving concealed both marker pairs. Japanese `nihongo` converted to `日本語` and Chinese `zhongwen` selected `中文`; their preedit left source at 4707 bytes, commits added 9/6 bytes, and one Undo restored each commit. Focus loss committed the active Korean `하`, refocus left no stale composition, and a new `ㅎ` composition worked and could be cleared. All temporary edits were undone to 4707 bytes. These were physical native IME input events, not pasted CJK text. The earlier CRLF-fixed build also passed at `scroll=45:19`.

The final semantic trace replayed **178 events**, including 44 preedit and 10 IME-commit events. Every preedit preserved canonical source. The replayed final document matched the original CRLF fixture **byte for byte**, not only by length. This trace also includes input-mode probes and the manual mode-setup interval. The main test window was left open at the restored baseline, and the temporary focus helper was closed.

Local evidence is retained under `target/windows-acceptance/`: command logs, RED/GREEN regressions, semantic traces and replay, action/scroll measurements, OS/GPU metadata, the exact code diff, and the resolved `Cargo.lock`. Its SHA-256 is `0f267d4da3dc5124c724452be328c13c4c0fced20a390a7a462a16be976f7c6e`; the generated lockfile is retained as local evidence rather than added to the repository.

Local native acceptance is complete for the tested patch. Native session logs contain no wgpu validation error, panic, device-loss error, or crash; invalid TeX produced only its expected fallback. The fixes and results are proposed for integration through Windows acceptance PR #14, whose fresh three-OS CI passed at `b04dce1`. Narrator/VoiceOver acceptance remains separately deferred.
