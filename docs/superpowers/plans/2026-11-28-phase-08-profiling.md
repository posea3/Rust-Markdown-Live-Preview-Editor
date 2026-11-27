# Phase 8 Measured Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver reproducible Windows measurements for all five roadmap fixtures after integrated-main CI passes.
**Architecture:** A release Rust example records real engine and cosmic-text stages; a bounded Windows runner captures CSV and process counters; documentation reports observations and limits.
**Tech Stack:** Rust 1.90, existing workspace/cosmic-text dependencies, PowerShell 7, no new dependency.
**Spec:** `docs/superpowers/specs/2026-11-28-phase-08-profiling-design.md`.

## Global Constraints

- Base `0df1961f5cd4f33dd0c8a32749ef82c59d7b75d5`, branch `feature/phase-08-profiling`; main CI `46023467296` passed before Phase 8 work.
- No core/editor/native-view behavior or dependency changes, optimization, new native acceptance, or input-to-photon claims.
- Exact fixture sizes, warmup/measured counts, fixed layout, resource bounds and censored-run reporting follow the spec.
- Raw CSV measurements must distinguish warmup from measured rows and identify load/edit/stages.

## Review Focus

- Fixture truncation, Unicode counting, CRLF handling and edit offsets must preserve the declared workloads and valid source.
- Release timings must measure real work with results consumed; font startup, logging, warmups, teardown and native GPU/presentation limits must remain explicit.
- Load versus edit must use the intended BlockCache/buffer lifetime and fresh validated snapshot/projection metadata.
- Timeout/memory-limited children must preserve partial results without becoming success or a fabricated percentile; no unrelated process is stopped.
- Summary/status claims must match raw rows, actual sample counts, profiled revision, merged PR order and final-head CI evidence.

## Task 1 — Profiler and bounded runner

**Files:** `examples/ime-lab/examples/phase8_profile.rs`, `scripts/profile-phase8.ps1`.
**Interfaces:** existing mdedit document/scanner/projection/caret APIs and cosmic-text Buffer; CSV and per-fixture process records.

- [x] Write fixture-generation tests, observe RED, implement deterministic fixtures and stage measurement example.
- [x] Add hidden-child Windows runner with raw outputs, peak-memory sampling and explicit resource limits.
- [x] Run focused checks and a small release smoke measurement; inspect real output/schema/semantics.

## Task 2 — Measurement, report and publication

**Files:** `docs/phase-08-performance.md`, portable results under `docs/performance/`, README/roadmap/Phase 7 integration records.

- [x] Measure all five fixtures; preserve/report any incomplete stages and run evidence-driven controls where needed.
- [x] Analyze raw completed rows, counts/scaling and limits; choose next optimization priorities from observations only.
- [x] Update Phase 7 merged/main-CI status and close its deferred CommonMark wording minor.
- [x] Run fmt/check/clippy/workspace tests, rustdoc/examples as appropriate; record totals and exact profile/harness identity.
- [ ] Commit, obtain one independent branch review, address Important issues with RED/GREEN, publish draft and verify current-head CI.

Review/final CI outcomes are recorded in the PR and ignored ledger without a status-only source commit.

Evidence-driven controls: baseline attempted all five fixtures under `c037bfd`, with 1 MB/10 MB/single-line runs censored by the whole-process budget. A no-warmup one-sample 1 MB run completed under a 180-second budget, and a core-only single-line run completed all five samples. Added a SourceVisible control and map-span metadata to separate concealment growth; its different display semantics must remain explicit. Full local verification after the control path: 294 passed, 0 failed, 1 ignored; fmt/check/clippy and optimized build PASS. Initial rustdoc and public examples also PASS.

Measurement/data review: SourceVisible controls under `21ffaf4` completed five samples for 10/100 KB and one no-layout sample for 1 MB. All 157 raw-to-summary groups were checked against their row counts/medians; the dependency snapshot hash matches the protocol. Report and portable evidence are committed separately from the measured harness identities, so those identities remain unambiguous. Final independent review and current-head CI are tracked by the profiling PR and ignored ledger.

Independent branch review verified all 157 groups including minima/maxima and identified three Important runner lifecycle issues. Fix pass: atomically claim an empty evidence directory, guarantee owned-child cleanup with `finally`, and require PowerShell 7 before parameter execution. Actual evidence-preservation/cancellation regressions and a 5.1 parser requirement check were observed RED/GREEN. The initial 5.1 file probe was blocked by local execution policy; no bypass was attempted. Measurement code/data are unchanged. One metadata iteration-identity minor remains deferred in the PR/ledger. Post-fix workspace and final-head CI outcomes are recorded there without another review cycle.
