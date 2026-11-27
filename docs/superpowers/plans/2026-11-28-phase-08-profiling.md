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

- [ ] Write fixture-generation tests, observe RED, implement deterministic fixtures and stage measurement example.
- [ ] Add hidden-child Windows runner with raw outputs, peak-memory sampling and explicit resource limits.
- [ ] Run focused checks and a small release smoke measurement; inspect real output/schema/semantics.

## Task 2 — Measurement, report and publication

**Files:** `docs/phase-08-performance.md`, portable results under `docs/performance/`, README/roadmap/Phase 7 integration records.

- [ ] Measure all five fixtures; preserve/report any incomplete stages and run evidence-driven controls where needed.
- [ ] Analyze raw completed rows, counts/scaling and limits; choose next optimization priorities from observations only.
- [ ] Update Phase 7 merged/main-CI status and close its deferred CommonMark wording minor.
- [ ] Run fmt/check/clippy/workspace tests, rustdoc/examples as appropriate; record totals and exact profile/harness identity.
- [ ] Commit, obtain one independent branch review, address Important issues with RED/GREEN, publish draft and verify current-head CI.

Review/final CI outcomes are recorded in the PR and ignored ledger without a status-only source commit.
