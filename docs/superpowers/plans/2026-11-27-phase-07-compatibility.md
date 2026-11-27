# Phase 7 Compatibility Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the composed public extension lifecycle and publish a precise Phase 7 compatibility/readiness audit.

**Architecture:** Add real exported-API integration tests and documentation; keep existing engine signatures and behavior. Review cumulative changes from integrated main, then publish a stacked draft with exact-head CI.

**Tech Stack:** Rust 1.90, standard library and existing workspace crates.

**Spec:** `docs/superpowers/specs/2026-11-27-phase-07-compatibility-design.md`.

## Global Constraints

- Base `4ee95b9c8071092f67050702f9a3734606dcad92`; branch `feature/extension-api-phase-07-compatibility`, above PR #18.
- No dependencies, public signatures, engine behavior, native input, or native renderer changes.
- No parent PR merge, frozen 1.0 claim, new native acceptance claim, or unprofiled Phase 8 optimization.
- Keep proposed implementation/review readiness distinct from integrated-main verification.

## Review Focus

- Legacy scanner/constructor and no-extension consumers must remain usable beside fallible scanners and checked projection.
- Scanner failures and configuration/document changes must not silently reuse stale concealed metadata in host composition.
- Renderer routing/failure/preflight must preserve namespaces, all-widget coverage, canonical ranges and concrete error chains.
- Returned actions must be applied only by host transactions; Unicode/CRLF canonical source and refresh obligations must survive the edit.
- Migration and status documentation must identify tightened duplicate semantics, new exhaustive-match variants, cache responsibilities and unmerged/native/1.0 limits accurately.

## Task 1 — Public integration and deferred coverage

**Files:** add `crates/mdedit-live/tests/extension_pipeline.rs`; update `tests/host_renderers.rs` and `src/host_renderer.rs` documentation only.

**Interfaces:** existing scanner/snapshot/checked-projection/registry APIs, `WidgetAction`, `Document::apply`, and a concrete host error newtype.

- [x] Add four integration cases and the later-unsupported-widget preflight case.
- [x] Add heterogeneous error guidance and a compiling rustdoc example; exercise the error chain through real dispatch.
- [x] Observe mutation failures for early-only preflight, removed configuration guard, and swallowed scanner errors; restore and pass focused tests.

## Task 2 — Audit, verification and publication

**Files:** add `docs/phase-07-compatibility-review.md`; update README, roadmap and Phase 7 guide.

**Interfaces:** consume Task 1 evidence and cumulative `7d46db3..HEAD` API diff; produce migration matrix and measured readiness status.

- [x] Audit all Phase 7 public changes and link representative consumer evidence; resolve the two old minors explicitly.
- [x] Run fmt/check/clippy/full workspace tests, rustdoc with warnings denied and all three examples; record actual totals.
- [ ] Commit and obtain one fresh-context cumulative independent review; address only important findings in one RED/GREEN fix pass.
- [ ] Publish a draft above PR #18; verify fresh exact-head push and PR CI on all three operating systems; record final outcomes in PR and ignored ledger.

Post-commit review/CI outcomes are recorded in the PR and execution ledger, without an extra status-only source commit.

Local verification on 2026-11-27, Windows: **291 passed, 0 failed, 1 ignored**. All named checks/examples passed. Each critical mutation produced the expected targeted failure, followed by restored focused tests and compiling rustdoc example. Evidence is retained in ignored `target/phase-07-compatibility`.

## Cumulative review correction

The review of `7d46db3..c57407c` identified an important delayed-action guidance error. Added a regression that reproduced canonical-source corruption with the old latest-revision recipe; using the originating projection revision makes the core guard return `StaleRevision`. Updated the immediate consumer recipe, the host example's queued transactions, and WidgetAction/renderer/lifecycle documentation. No public API or engine behavior change was needed. One CommonMark fixture-wording minor remains deferred in the PR/ledger.

Post-review Windows verification on 2026-11-28: **292 passed, 0 failed, 1 ignored**, with fmt/check/clippy/rustdoc and all three examples PASS. The new regression is recorded RED/GREEN. The correction is committed in the same stacked PR and requires fresh final-head CI; no repeat reviewer or status-only source commit is needed.
