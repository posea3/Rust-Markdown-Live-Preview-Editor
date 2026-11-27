# Phase 7 Extension Configuration Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent hosts from reusing cached extension data from an obsolete or unrelated scanner configuration.

**Architecture:** A lazily allocated process-local identity belongs to each observed `ExtensionSet` configuration and is copied into scan snapshots. A new projection builder checks that identity before applying cached metadata. Existing fresh-scan APIs remain available.

**Tech Stack:** Rust 1.90, standard-library OnceLock/AtomicU64, current mdedit workspace.

**Spec:** `docs/superpowers/specs/2026-11-27-phase-07-config-design.md`.

## Global Constraints

- Rust 1.90; no new dependency.
- Preserve existing method signatures and the const `ExtensionSet::new()` constructor.
- No canonical-source, native input, or widget-rendering change.
- Configuration identity is process-local; document identity and parser epochs remain host responsibilities.
- Execute natively on `feature/extension-api-phase-07-config`, based on integrated main `7d46db3`.

## Review Focus

- Equal document revisions/lengths must not permit stale configuration to restore concealed widgets.
- Unrelated sets, including empty sets and equal scanner namespaces, must not share a cache identity.
- Duplicate registration must preserve the current identity and accepted snapshot, while successful registration invalidates it.
- Interior scanner-setting changes require explicit host invalidation; fresh scan and cloned snapshots must carry the correct identity.
- ID allocation must not wrap/reuse IDs, and the checked builder must retain revision/length validation after accepting configuration.

## Task 1 — Configuration provenance and checked projection

**Files:** modify `crates/mdedit-markdown/src/extension.rs`, its `lib.rs`, and `crates/mdedit-live/src/projection.rs`; add consumer tests in `crates/mdedit-markdown/tests/extension_configuration.rs` and `crates/mdedit-live/tests/extension_configuration.rs`.

**Interfaces:** `ExtensionConfigId`; `ExtensionSet::configuration_id() -> ExtensionConfigId`; `ExtensionSet::invalidate_configuration(&mut self)`; `ExtensionSnapshot::configuration_id() -> ExtensionConfigId`; `Projection::build_with_current_extensions` adds `&ExtensionSet` after the snapshot argument; mismatch error carries `expected` and `snapshot` IDs.

- [x] Write consumer tests for the metadata and stale-projection contracts and observe the absent API before implementation.
- [x] Add the lazy identity, checked allocator, invalidation/stamping, and snapshot accessor; test last-ID allocation and exhaustion using an isolated counter.
- [x] Add the checked builder and configuration mismatch error; retain existing revision/length checks and fresh-snapshot behavior.
- [x] Run focused tests; disable the new mismatch guard temporarily to prove stale/cross-set regression tests fail, then restore and rerun GREEN.
- [x] Run fmt/check/clippy/workspace tests, rustdoc with warnings denied, and public examples; inspect all results.

## Task 2 — Integration record and publication

**Files:** update README, roadmap, Phase 6/7 status, and the existing scanner example with cache-key/invalidation guidance.

- [x] Record the three merged PRs and integration CI; make Windows acceptance status revision-specific.
- [x] Document cache identity, explicit invalidation, checked projection, snapshot equality, and migration from unchecked cached reuse; retain remaining failure-policy/renderer work.
- [ ] Obtain an independent whole-branch review and address important findings with RED/GREEN verification.
- [ ] Commit/publish a focused draft PR against main; record final-head CI results in the PR and execution ledger.

## Local evidence — 2026-11-27

- Integrated baseline: **252 passed, 0 failed, 1 ignored**; main CI `45979821603` passed all three operating systems.
- New public configuration tests: **10 passed**; allocator boundary test: **1 passed**.
- Mutation RED: removing the projection configuration guard failed all three stale/cross-set tests; wrapping allocation failed the exhaustion test. Both implementations restored before final verification.
- Final local suite: **263 passed, 0 failed, 1 ignored**.
- fmt/check/clippy/rustdoc/example: PASS. The example rescanned after registration without any source edit and retained its Unicode/reference/issue/protected-code output.
- CI and review outcomes are recorded in the PR and execution ledger so the tested code head remains unchanged by status-only edits.
