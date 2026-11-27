# Phase 8 Indexed Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove repeated linear map searches while preserving every mapping/caret contract and measure the effect.
**Architecture:** Binary search existing visible spans for random queries; share boundary logic with a monotone bulk iterator used by caret generation.
**Tech Stack:** Rust 1.90, current workspace, no new dependencies.
**Spec:** `docs/superpowers/specs/2026-11-28-phase-08-indexed-mapping-design.md`.

## Global Constraints

- Base `6603de6dd8a3b03742ec7bc8e782bf66470ea5fb`, branch `feature/phase-08-indexed-mapping`, stacked above draft #20.
- Preserve public API, source bytes, Before/After edge semantics, reveal/widget contracts and Unicode grapheme behavior; no native view/parser/scanner/geometry change.
- Reuse existing span storage; no extra prefix-index allocation or timing assertions in CI.
- Keep changed metadata format and before/after cohort identities explicit; do not edit historical CSV.

## Review Focus

- Empty/fully concealed documents and leading/trailing/interior collapsed endpoints must retain both biases and valid out-of-range results.
- Sparse or repeated monotone boundaries and graphemes spanning multiple map segments must advance correctly without losing source endpoints.
- Arbitrary byte offsets and near-u32 arithmetic must not introduce overflow, UTF-8-only restrictions or changed source-range mapping.
- New indexed/bulk paths must agree with independent mathematical oracles across real reveal/widget projections, not merely with each other.
- Performance and memory claims must match same-policy raw rows/sample counts; remaining owner/layout costs, metadata schema differences and unmerged status remain explicit.

## Task 1 — Mapping and caret algorithms

**Files:** `crates/mdedit-live/src/projection.rs`, `src/projection/mapping_tests.rs`, `src/caret.rs`, `tests/mapping_equivalence.rs`.
**Interfaces:** current public map/caret APIs, shared private source-bounds helper and ordered projected-boundary iterator.

- [x] Write exhaustive/literal/public consumer tests and observe missing bulk API RED.
- [x] Implement indexed queries and ordered caret mapping without extra index/boundary arrays; pass focused tests.
- [x] Verify boundary/jump mutations fail; restore and run full fmt/check/clippy/workspace/rustdoc/examples.

## Task 2 — Comparison and publication

**Files:** profiling example metadata emitter; performance report/README/roadmap; portable comparison data; new results report as needed.

- [x] Retain metadata iteration kind and validate actual release CSV keys; archive format remains documented.
- [x] Commit measured code identity, compare 10/100 KB five-sample runs and a 1 MB same-protocol observation; attempt larger/core-only cases with explicit bounds where useful.
- [x] Preserve source/projection semantic metadata, raw data, counts and environment; report remaining costs and source-based hypotheses honestly.
- [ ] Commit, obtain one independent branch review, address Important findings with RED/GREEN and full suite, publish draft above #20 and verify final-head CI.

Post-commit review/CI outcomes are tracked in the PR and ignored ledger, without a status-only source commit.

Local Windows verification before performance comparison: **299 passed, 0 failed, 1 ignored**. Format, check, clippy/rustdoc with warnings denied, all three public examples and optimized profiler build passed. All three targeted mutations failed as expected; restored tests passed. Actual release schema2 smoke confirmed unique metadata keys and distinct iteration kinds. Evidence is retained in ignored `target/phase-08-indexed-mapping`.
