# Phase 8 Extension Scan Hotspots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attribute and optimize extension scan work with complete ordered results/errors preserved.
**Architecture:** Private opt-in stage markers; select build-local search indexes from measured dominance; independent reference/oracles and reproducible feature-off comparisons.
**Tech Stack:** Rust 1.90, existing workspace dependencies, PowerShell 7.
**Spec:** `docs/superpowers/specs/2026-11-28-phase-08-extension-hotspots-design.md`.

## Global Constraints

- Base `36a6dffeb2ccc3f98e45a0387ecf2651f05fa871`; branch `feature/phase-08-extension-hotspots`, stacked above draft #22; no merges.
- Preserve scanner APIs/configuration identity, callbacks/prevalidation/failure/panic behavior, protected syntax precedence, candidate validation, complete matches/attributes/issues and first-conflict reporting order.
- Feature-off builds have no profiling clocks/counters/logs; no dependencies, caches across builds or wall-clock CI thresholds.
- Keep root Cargo.lock untracked and archived numerical rows unchanged; distinguish warmed/single samples, instrumented totals, process peaks and native latency.

## Review Focus

- Priority/start/end/kind ties and allowed nesting must keep the earliest conflicting accepted candidate, exact issue sequence and attribute order.
- Protected ranges must retain first traversal precedence, strict overlap/touching semantics, invalid-range-before-protection handling and AllSource bypass.
- Empty/duplicate/nested/crossing/extreme ranges and incremental index activation must not report rejected or future candidates as conflicts; broad disjoint queries need bounded work.
- Fallible errors, panics, stateful scanner callbacks, prevalidation and configuration capture must preserve invocation order/count and observable error identity.
- Trace extraction must handle torn/zero-observation interruptions and detect whole missing warmup invocations; evidence must distinguish featured calls, complete samples, digest limitations and remaining CPU/native costs.

## Task 1 — Attribute unchanged scanner work

**Files:** markdown Cargo features; `src/extension.rs`, private `src/extension/profile.rs`; profiler example metadata; `scripts/extract-phase8-stages.ps1` and regression tests.
**Interfaces:** `extension-begin,<call>,<stage>` / `extension-end,<call>,<stage>,<ns>`; scanner-specific labels `scanner_N` and `candidate_filter_N`; outer stage `scan`. Extractor `-Directory <cohort> -Subsystem extension|projection` emits subsystem timing/summary/censored CSVs and validates completed outer invocations including warmups.

- [x] Observe missing extension stage transcript RED and write extractor tests for prior interruption bugs plus wholly missing warmup.
- [x] Add private feature-only timing and untimed issue/digest metadata; validate ordered stage transcripts, feature-off marker absence, common metadata and all extractor regressions.
- [ ] Freeze instrumentation-only code/binaries; run serial profiled/normal 10/100KB/1MB warmed cohorts and bounded 10MB single observations. Record measured dominance before optimization.

## Task 2 — Optimize measured searches

**Files:** private extension lookup/merge helpers and tests; frozen parent reference; public consumer regression if needed.
**Interfaces:** Preserve original protection/conflict selection results. Exact private index interfaces and input assertions are refined here after Task 1 attribution, before algorithm changes.

- [ ] Record measured target and exact index/merge interfaces; choose the smallest measured change.
- [ ] Add independent full-snapshot and brute-force range/result oracles plus deterministic disjoint-work tests; observe missing/old search RED.
- [ ] Implement selected searches; prove wrong boundary/first-winner/activation/protection mutations fail; restore originals and pass focused/consumer tests.
- [ ] Verify fmt/check/default+profiling clippy, workspace/feature tests, rustdoc, three examples and both release builds; freeze optimized code identity.

## Task 3 — Compare and publish

**Files:** report `docs/phase-08-extension-hotspots.md`, portable raw data/manifests/helpers, README/roadmap links, CI feature/extractor steps.
**Interfaces:** Task 1 markers/metadata and Task 2 unchanged snapshots; old projection archive remains historical.

- [ ] Measure serial normal/profiled after cohorts under matching protocols; verify all summaries, completed invocation links, semantic metadata/digests and source/binary/lock identities.
- [ ] Report stage attribution, total scan observations and remaining costs; commit raw evidence and results.
- [ ] Obtain one fresh whole-branch review, fix Important issues RED→GREEN plus full suite, publish stacked draft above #22 and confirm final-head push/PR three-OS CI.

Final review/CI outcomes live in the PR and ignored ledger; avoid status-only source commits.
