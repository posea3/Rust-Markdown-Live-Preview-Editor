# Phase 6 Callout / Custom Embed Native Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 6 Slice 06 by rendering Obsidian callouts and standalone custom embeds as source-mapped native widgets with stable block reservation and reveal/conceal behavior.

**Architecture:** Keep canonical Markdown in `EditorSession`; `mdedit-markdown` supplies extension ranges, `mdedit-live` supplies typed widget payloads, and the IME lab owns geometry, block reservations, native rectangles, and glyphon text buffers. Reuse the existing `ReservationOverlay`, `widget_anchor_geometry`, rectangle renderer, and auxiliary text-area pattern used by Table.

**Tech Stack:** Rust 1.90 baseline, winit, wgpu, cosmic-text, glyphon, existing `mdedit-*` crates.

**Spec:** `docs/phase-06-widgets.md`

## Global Constraints

- Canonical Markdown remains authoritative; widgets never mutate source directly.
- No JavaScript, HTML, WebView, or new rendering dependency.
- Callout and standalone embed source reveals whenever caret or selection touches the owned source range.
- Inline embeds remain source-visible until an inline-width contract exists.
- Image, Math, Table, Checkbox, Rule, IME, and history behavior must remain unchanged.
- Windows native GUI/IME acceptance remains deferred to the final Phase 6 integration pass.

## Review Focus

- Callout source ranges containing CRLF or a trailing newline preserve the same vertical footprint when concealed.
- Nested Markdown in a callout body displays visible text without corrupting canonical source.
- A callout containing an image/embed does not produce overlapping nested widgets while inactive.
- Long titles and bodies remain clipped inside the widget rather than painting over following text.
- Standalone custom embeds with missing display/fragment values produce a safe label and never panic.

---

### Task 1: Callout and embed geometry plus reservation

**Files:**
- Modify: `examples/ime-lab/src/main.rs`
- Modify: `examples/ime-lab/src/live_preview.rs`

**Interfaces:**
- Consumes: `ProjectedWidget::callout()`, `ProjectedWidget::custom_embed()`, `WidgetBlockReservation::new`.
- Produces: `CalloutBlockLayout`, `callout_block_layout`, `callout_block_rect`, `custom_embed_rect`, and canonical-source-footprint reservations.

- [ ] Write failing geometry and reservation tests for callout and custom embed.
- [ ] Verify RED in CI: missing geometry/reservation helpers or wrong display-line offsets.
- [ ] Implement minimal layout helpers and extend `ReservationOverlay` eligibility to Callout/CustomEmbed.
- [ ] Use canonical source newline counts for Callout reservation; keep CustomEmbed to one visual row unless the card contract requires more.
- [ ] Verify fmt/check/clippy/workspace tests on Windows, macOS, and Ubuntu.
- [ ] Commit the GREEN implementation.

### Task 2: Native Callout card and text

**Files:**
- Modify: `examples/ime-lab/src/main.rs`

**Interfaces:**
- Consumes: `CalloutWidgetData`, Task 1 geometry.
- Produces: `CalloutTextSpec`, `callout_text_specs`, type palette, background/accent/title/body rendering.

- [ ] Write failing tests for palette fallback, title/body geometry, clipping, and visible text order.
- [ ] Verify RED.
- [ ] Draw native background, accent bar, header separator/icon marker, and fold indicator through `RectRenderer`.
- [ ] Render title and body through auxiliary cosmic-text/glyphon buffers, clipped to the callout card.
- [ ] Verify all tests and existing widget regressions.
- [ ] Commit.

### Task 3: Native standalone Custom Embed card

**Files:**
- Modify: `examples/ime-lab/src/main.rs`

**Interfaces:**
- Consumes: `CustomEmbedWidgetData`, Task 1 geometry.
- Produces: display-label fallback and a native one-row card/placeholder.

- [ ] Write failing tests for display/path/target label fallback and card bounds.
- [ ] Verify RED.
- [ ] Draw a native embed card with icon marker, primary label, and optional fragment/path detail.
- [ ] Keep inline embeds source-visible and preserve canonical source reveal.
- [ ] Verify all tests and existing widget regressions.
- [ ] Commit.

### Task 4: Acceptance fixture, documentation, and PR gate

**Files:**
- Modify: `examples/ime-lab/fixtures/live-preview-acceptance.md`
- Modify: `docs/phase-06-widgets.md`
- Modify: `README.md`
- Modify: `docs/roadmap.md`
- Modify: PR #12 body

**Interfaces:**
- Consumes: completed native Callout and Custom Embed rendering.
- Produces: reproducible macOS acceptance instructions and an accurate branch status.

- [ ] Add callout fixtures covering type/title/fold/body/nested Markdown and standalone embeds with/without display text.
- [ ] Add reveal/conceal, overlap, viewport, IME, and prior-widget acceptance checks.
- [ ] Run final 3OS CI and record the run.
- [ ] Keep PR #12 Draft until macOS manual acceptance passes.
