# Phase 8 Projection Hotspots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attribute projection costs, remove the measured repeated containment work, and publish verified same-policy comparisons.
**Architecture:** Opt-in private build-stage timing; build-local range/owner searches selected from measured attribution; independent equivalence/scalability oracles.
**Tech Stack:** Rust 1.90, current workspace dependencies, PowerShell 7 runner.
**Spec:** `docs/superpowers/specs/2026-11-28-phase-08-projection-hotspots-design.md`.

## Global Constraints

- Base `5cee82218ed27052aa3803535985017ca65d47e6`; branch `feature/phase-08-projection-hotspots`, draft stacked above #21; no merges.
- Preserve public projection signatures, complete projection semantics, fallback/errors, traversal ties and source bytes; no native view/parser/scanner change.
- Default builds have no profiling clocks/logs; no dependencies or wall-clock CI thresholds.
- Do not edit historical numerical rows; retain untracked root Cargo.lock; distinguish profiled/normal binaries, sample counts and censored prefixes.

## Review Focus

- Nested/equal/touching ranges and equal-length owner ties must preserve first traversal choice, including empty ranges and zero/maximum endpoints.
- Callout/embed owners must retain active/inactive suppression of contained built-in widgets and delimiters without suppressing unrelated or partially overlapping ranges.
- New build-local indexes must have bounded repeated-query work on broad trees and retain correct nested owners; no revision/configuration cache can leak across builds.
- Profiling markers must associate only completed stages with their serial outer invocation, retain interrupted prefixes, and introduce no default-build clocks or public API changes.
- Before/after evidence must preserve policies/full projected output, distinguish marker overhead, avoid comparing incompatible censored protocols, and state remaining scanner/layout/native costs.

## Task 1 — Attribute existing projection stages

**Files:** `crates/mdedit-live/Cargo.toml`, `src/projection.rs`, private `src/projection/profile.rs`; runner `BinaryPath` option for separately frozen normal/instrumented binaries; profiling report/raw evidence and ignored trace audit.
**Interfaces:** unchanged public build methods; opt-in `projection-profiling` feature; stderr `projection-begin,<call>,<stage>` / `projection-end,<call>,<stage>,<ns>` inside the existing outer `begin,...,projection` record.

- [x] Verify the existing release fixture has no internal markers (observed RED for the required stage transcript).
- [x] Implement private feature-gated stage timing; verify complete ordered 10 KB load/edit transcripts and unchanged metadata; default build has no markers.
- [x] Freeze instrumentation-only commit and binary/lock hashes; run serial internal 10/100 KB and 1 MB cohorts with bounds and record dominant stages before selecting an optimization.

## Task 2 — Optimize measured containment work

**Files:** `src/projection/ownership.rs`, its independent tests, projection integration/reference tests, and prior `tests/mapping_equivalence.rs` fixture assertions.
**Interfaces:** private `OwnerLookup::new(&SyntaxNode)`, `smallest_containing(SyntaxKind, TextRange) -> Option<&SyntaxNode>`; `ContainmentSet::new(impl IntoIterator<Item=TextRange>)`, `contains(TextRange) -> bool`. Keep previous traversal helpers only under tests as an independent reference.

**Attribution gate (before optimization):** instrumentation-only `87cf927`; 1 MB edit: reveal groups 13,529.6509ms, shadow filtering 3,001.7534ms, extension candidates 431.7182ms, normalization 60.7152ms. Together these repeated containment passes dominate; normal feature-off baseline cohorts were freshly measured as well.

**Selected algorithm:** Build one balanced interval index of syntax nodes, using effective ancestor-intersection bounds, subtree maximum ends, original node ranges and preorder tie ranks. Search containing nodes only, choose smallest original range then first preorder. Charge index construction to `reveal_groups` and reuse it for callout lookup. A sorted-start/prefix-maximum-end `ContainmentSet` answers any-containing queries for extension normalization and shadow suppression in O(log N), retaining exact active/inactive conditions. All data is local to the current build.

**Required tests:** exhaustive small range subsets against brute-force any-containing scans (including empty/touching/max endpoints); mixed-kind unsorted/nested/equal-size node entries against independent full scans; 4,096 disjoint broad nodes with all-point query visit bound; parsed nested/repeated-owner trees against the original traversal on every range-edge combination; complete `Projection` equality against the retained reference on a Unicode/CRLF/widget/callout corpus under both policies and reveal contexts; explicit projected/widget assertions in the prior public fixture.

- [x] Record measured dominant passes and refine this task to exact range/index interfaces; choose the smallest local optimization covering those passes.
- [x] Add independent brute-force equivalence and deterministic visit-count tests; observe absent index/old repeated work RED before implementation.
- [x] Implement selected index/searches; verify all edge/corpus assertions and targeted incorrect containment/tie/shadow mutations fail; restore originals.
- [x] Run fmt/check/clippy/workspace/rustdoc/examples and profiled-feature tests/builds; freeze measured optimized commit.

## Task 3 — Measure and publish

**Files:** `docs/phase-08-projection-hotspots.md`, portable raw evidence directory, README/roadmap links.
**Interfaces:** Task 1 markers and Task 2 unchanged semantic outputs; feature-off total timings remain comparable to prior cohorts.

- [ ] Run serial profiled after and normal before/after cohorts; keep 10 MB bounds explicit; verify every summary and semantic metadata key, and preserve raw data/manifests.
- [ ] Report internal attribution, total observed improvement and remaining costs; commit results.
- [ ] Obtain one fresh whole-branch review; fix Important findings RED→GREEN plus full suite; publish a stacked draft above #21 and confirm final-head push/PR Windows/macOS/Ubuntu CI.

Review/CI outcomes after commits live in the PR and ignored execution ledger; avoid status-only source commits.
