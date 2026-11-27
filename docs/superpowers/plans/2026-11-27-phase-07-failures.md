# Phase 7 Scanner Failure Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let hosts identify and handle declared scanner failures without receiving partial extension snapshots or breaking legacy scanner implementations.

**Architecture:** Register legacy and fallible scanners through a private shared dispatch enum. The scan engine maps fallible errors to the captured namespace and stops before publishing a snapshot. Successful callbacks use the existing validation and merge pipeline.

**Tech Stack:** Rust 1.90 and existing workspace crates, no new dependencies.

**Spec:** `docs/superpowers/specs/2026-11-27-phase-07-failures-design.md`.

## Global Constraints

- Preserve existing scanner, registration, snapshot and projection method signatures and the const set constructor.
- Strict all-or-nothing reported-error policy; callback panics still propagate.
- No canonical-source, native rendering, or input behavior change.
- Work on `feature/extension-api-phase-07-failures`, based on `bacd562`; do not merge PR #16.

## Review Focus

- The first declared failure must not become a successful partial snapshot or invoke subsequent scanners.
- Failure errors must retain the captured namespace and owned source cause even when a scanner changes its reported ID.
- Both interfaces must share namespace uniqueness and configuration provenance without bypassing protected-source or range validation.
- Metadata validation must happen before callbacks, and scanner side effects before failure are not rolled back.
- Failure/retry and panic behavior must be explicit; hosts must discard failed-attempt cache data and invalidate configuration when settings change.

## Task 1 — API and strict scan execution

**Files:** create `crates/mdedit-markdown/src/fallible.rs`; modify `extension.rs` and `lib.rs`; add public tests in markdown/live `tests/extension_failures.rs`.

**Interfaces:** `FallibleSyntaxExtension::try_scan`; `ExtensionScanError`; `ExtensionSet::{try_register_fallible,register_fallible,with_fallible_extension}`; `ExtensionError::ScannerFailed { extension: ExtensionId, error: ExtensionScanError }`.

- [x] Add consumer regressions and verify the fallible API is absent.
- [x] Implement the trait/error, shared registration/dispatch, captured failure error/source chain, and immediate scan abort.
- [x] Run focused tests; suppress failure propagation temporarily and observe failure-related regressions turn RED, then restore GREEN.
- [x] Run fmt/check/clippy/full workspace tests, rustdoc with warnings denied, and both scanner examples.

## Task 2 — Host guidance, review, and publication

**Files:** add `examples/fallible_scanners.rs` in the markdown crate; update README, roadmap and Phase 7 guidance.

- [x] Demonstrate complete success and explicit failure/source-visible fallback; document error versus candidate diagnostics and unchanged panic policy.
- [x] Clarify parser/dialect changes discard related extension snapshots and require rescanning; do not attribute new GUI runs to this scanner-only work.
- [ ] Obtain an independent whole-branch review and fix important findings with RED/GREEN evidence.
- [ ] Commit and publish a draft PR targeting the Slice 03 branch; track final-head CI in the PR and ledger without status-only source commits.

## Local evidence — 2026-11-27

- Baseline: **263 passed, 0 failed, 1 ignored**.
- New focused tests: **12 markdown + 1 Live Preview passed**.
- Mutation RED: suppressing fallible errors failed three failure-related tests; correct propagation restored.
- Full suite: **276 passed, 0 failed, 1 ignored**.
- fmt/check/clippy/rustdoc and both examples: PASS. The new example produced two complete matches, then identified `example.issues` and printed canonical source for its failure case.
- Independent-review and current-head CI outcomes are tracked in the PR and execution ledger.
