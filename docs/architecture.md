# Architecture

## Mission

Build a reusable, source-first, native Rust Markdown editing engine with Live Preview. The engine must not depend on JavaScript, HTML, WebView, Electron, or CodeMirror at runtime and must not be tied to NodeNotes or any other host application.

## Non-negotiable principles

1. Markdown source is the only canonical document.
2. Public source coordinates are UTF-8 byte offsets wrapped in project-owned types.
3. All edits pass through `Transaction`; no feature mutates the text buffer directly.
4. Source revision mapping (`ChangeMap`) and source-to-preview mapping (`ProjectionMap`) are separate concepts.
5. IME composition is ephemeral state and is committed only as a transaction.
6. Core crates expose no winit, wgpu, glyphon, egui, GPUI, iced, crop, or pulldown-cmark types.
7. Ambiguous Live Preview syntax falls back to raw Markdown instead of guessing.
8. Correctness precedes incremental performance.

## Target stack

- Buffer: crop
- Markdown parser: pulldown-cmark behind an adapter
- Text shaping: cosmic-text
- GPU text: glyphon
- GPU: wgpu
- Window/input standalone adapter: winit
- Clipboard: arboard
- Accessibility: AccessKit

## Crate plan

```text
mdedit-core
  Document / Snapshot
  Revision
  TextSize / TextRange
  Anchor / Affinity / Selection
  Change / ChangeSet / ChangeMap
  Transaction
  History

mdedit-markdown
  MarkdownParser
  SyntaxSnapshot / SyntaxTree / BlockTree
  MarkdownDialect
  DelimiterResolver
  SyntaxExtension

mdedit-live
  Projection / ProjectionMap
  StyleSpan / ConcealSpan / WidgetSpan
  RevealGroup / RevealPolicy
  raw-source fallback

mdedit-view
  Viewport
  block metrics
  hit testing
  scrolling
  source/projection/layout coordinate bridges

mdedit-cosmic
  cosmic-text adapter

mdedit-glyphon
  glyphon/wgpu renderer adapter

mdedit-winit
  platform input adapter
  IME bridge
  clipboard/accessibility hooks

mdedit
  umbrella crate
```

## Coordinate model

There are three coordinate spaces:

```text
Canonical source UTF-8 byte offset
        |
        | ProjectionMap
        v
Projected position
        |
        | LayoutMap
        v
Pixel position
```

Document revisions use a separate `ChangeMap`:

```text
old source position
        |
        | ChangeMap
        v
new source position
```

These mappings must never be merged into one abstraction.

## Live Preview model

Live Preview is a projection, never a rewritten document.

Example:

```markdown
This is **important**.
```

Canonical source remains unchanged. A projection describes the opening and closing `**` as concealed spans and the content as a strong-styled source span. When the cursor, selection, or IME composition intersects the syntax construct, its reveal group exposes the source markers again.

The parser alone is not sufficient to decide exact marker boundaries. `mdedit-markdown` therefore combines parser source ranges with the original source in a delimiter resolver. If exact boundaries cannot be proven, that construct is rendered as raw Markdown.

## IME

IME is treated as a first-class architecture concern, not a late platform patch.

Preedit text lives in a `CompositionState` overlay. It does not enter history or the canonical document. Only IME commit creates a transaction. The active composition syntax range is forced into revealed-source mode and its reveal state is stabilized until composition ends.

## Parsing strategy

v0.x begins with correctness-first full parsing. Large documents move parsing to snapshots/background work so UI input latency is not proportional to document size.

Incremental block parsing is a later optimization and must always retain a safe full-parse fallback.

## Host boundaries

The editor does not open files, launch browsers, execute links, fetch images, or own application navigation. It emits host actions and receives resources through interfaces. This keeps the library reusable across note apps, graph editors, learning tools, and other Rust applications.
