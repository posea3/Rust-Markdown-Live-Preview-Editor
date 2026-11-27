# Phase 7 Host Renderer Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let applications register view-owned rendering callbacks for immutable widget/extension descriptors while preserving source-first projection and explicit failure handling.

**Architecture:** Add a small generic dispatch registry in `mdedit-live` with distinct built-in/qualified-extension keys, borrowed inputs, duplicate checks, typed dispatch errors, and callback-free widget support preflight. Applications keep all graphics and editor action handling.

**Tech Stack:** Rust 1.90, current mdedit crates, standard library only.

**Spec:** `docs/superpowers/specs/2026-11-27-phase-07-renderers-design.md`.

## Global Constraints

- No dependency, native renderer, input, projection algorithm, or canonical-source change.
- No public GPU/font/window types and no Send/Sync requirement on view callbacks.
- Existing APIs remain unchanged; no implicit extension concealment, handler replacement, or panic catching.
- Base `00de68eeaf119adc08c313c38e16f31bdc72d89b`, branch `feature/extension-api-phase-07-renderers`; parent PR #17 stays unmerged.

## Review Focus

- Qualified extension identities and built-in keys must never dispatch to a foreign handler sharing a local name.
- Missing or failed renderers must return explicit key/range/cause information rather than silently succeeding.
- Preflight must reject unsupported concealed widgets before callbacks, allowing the host to rebuild a source-visible frame.
- Duplicate registration must preserve the original handler; stateful view-thread-only callbacks must remain usable.
- Renderer registration/dispatch must not change source, projection, namespace interpretation, or widget/editor action ownership.

## Task 1 — Registry and public consumer contract

**Files:** add `crates/mdedit-live/src/host_renderer.rs`, export from `lib.rs`, and add `tests/host_renderers.rs`.

**Interfaces:** `HostRenderKey`, `HostRenderInput`, `HostRendererRegistry<C,O,E>`, duplicate registration error, typed `HostRenderError<E>`, and `check_widget_support(&Projection)`.

- [x] Write consumer tests and observe the absent API.
- [x] Implement registration, derived-key dispatch, typed errors/source chain, and callback-free support preflight.
- [x] Run focused tests; temporarily bypass critical guards to observe RED, restore GREEN.
- [x] Run fmt/check/clippy/full workspace tests and rustdoc with warnings denied.

## Task 2 — Host example, guidance, and publication

**Files:** add `crates/mdedit-live/examples/host_renderers.rs`; update README, roadmap and Phase 7 guide; apply the two deferred Slice 04 wording clarifications when touching those files.

- [x] Demonstrate application callbacks/actions and source-visible fallback before painting.
- [x] Document current-input responsibility, foreign-source visibility, native-view ownership, runtime failure handling, and renderer cache ownership.
- [ ] Obtain independent review; address important findings with RED/GREEN evidence.
- [ ] Commit/publish a stacked draft PR, verify current-head CI, and record review/CI outcomes in the PR and execution ledger.

## Local verification snapshot

2026-11-27, Windows: **285 passed, 0 failed, 1 ignored**. Format, workspace/all-target check, clippy with warnings denied, rustdoc with warnings denied, and all three examples passed. Nine new public consumer tests passed. Each of three critical-guard mutations produced its targeted assertion failure, then the restored implementation passed all nine tests. The example's task-list dialect correction was also observed RED/GREEN. Logs are retained under ignored `target/phase-07-renderers`.

The last two checkboxes describe post-commit work. Their final review/publication/current-head CI outcomes are recorded in the PR and execution ledger rather than an additional status-only source commit.
