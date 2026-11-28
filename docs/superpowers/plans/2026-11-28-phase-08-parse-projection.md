# Phase 8 Parse and Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attribute remaining parse/projection work, optimize measured local costs, and publish semantic/performance evidence.
**Architecture:** Flat optional substage markers; diagnostic event-buffer split for lazy parser work; build-local ownership/allocation changes selected after measurements.
**Tech Stack:** Rust 1.90, current workspace dependencies, PowerShell 7.
**Spec:** `docs/superpowers/specs/2026-11-28-phase-08-parse-projection-design.md`.

## Global Constraints

- Base `933c16fc3b04f8e734bf835040849a177fd82192`, branch `feature/phase-08-parse-projection`, stacked draft above #23; no merges.
- Preserve public APIs/const construction, complete syntax/projection values and fallback/error/reveal/widget/map semantics; no dependency changes or cross-build cache.
- Default parsing remains a single-pass event consumer; buffered diagnostics have separate timing/memory scope.
- Keep root Cargo.lock untracked and historical numerical archives unchanged; no wall-clock CI assertions or native latency claims.

## Review Focus

- Malformed/invalid/out-of-order event streams must keep first fallback reason and early consumption in the normal builder; valid metadata/dialect/source ranges must remain identical.
- Sparse/owned/borrowed metadata and wide or single-child trees must preserve values, order, const construction and safe growth when storage strategies change.
- Delimiter-owner specialization must retain nested/clipped/equal-length traversal ties and include extension-only callout owners, with full source reveal/widget behavior.
- Optional diagnostics must remain outside default execution, clearly distinguish buffering costs, and validate complete/open traces including interruptions and warmups.
- Before/after claims must use matching normal cohorts, complete semantic values and original bounds; small regressions, transient index/buffer storage and remaining vendor/native costs stay explicit.

## Task 1 — Deeper attribution

**Files:** parser feature/private profile; projection stage splits/compile-time wrapper; profiler syntax/projection metadata; extractor parse preset/tests.
**Interfaces:** parse markers with `source_text,parser_init,decode_events,build_tree`; projection17-stage preset via explicit ExpectedStages, retaining historical projection15 default. Diagnostic parse buffers events; normal path remains unchanged. Root source copy and index construction included in measured stages.

- [x] Observe missing parse/deep projection transcript RED; add extractor parse-preset smoke/strict interruption regression before support.
- [x] Implement feature-only diagnostics and streaming semantic metadata; validate complete inner/outer links, normal marker absence and object equality on fixture smoke.
- [x] Freeze instrument-only code/binaries; serial profiled/normal warmed small/1MB and bounded10MB runs; record target stages before selecting code changes.

## Task 2 — Measured local optimization

**Files:** selected parser metadata/child-storage and projection ownership helpers; frozen parent parser; independent semantic/work tests; CI feature steps.
**Interfaces:** `OwnerLookup::for_kinds(root:&SyntaxNode,kinds:impl IntoIterator<Item=SyntaxKind>)->Self` partitions only requested kinds while retaining effective ancestor bounds and relative preorder; include BlockQuote for extension-only owners. Existing generic constructors remain test references. `SyntaxNode::push_child` keeps its signature/const new, using private kind-biased first-allocation capacities (1 for inline/heading/quote/code/cell containers, 2 for list/table row/head/table, otherwise normal Vec growth). No node merging or metadata change.

**Attribution gate:** before `c81f6b34e607a516964e5d0ed8d83fe987edd72c`; 10 MB diagnostic edit parse decode396.4424ms/tree344.9857ms/init266.7008ms; projection grouping286.0357ms/index114.8764ms, styles134.6304ms, built-in candidates118.7728ms. Tree1,982,739nodes/4,317,352allocated child slots/517,236single-child nodes; profilepeak1,340.9MiB. Optimize local tree storage and owner index scope; vendor init/decode remain. Buffered diagnostic times are not normal parser timings.

**Required tests:** frozen parent parser equality on hundreds of generated/dialect/metadata/Unicode fixtures; malformed event fallback/consumption; const construction plus wide-node growth and aggregate allocated-slot bounds; requested-kind lookup equality against generic/brute-force owners, extension-only callout inclusion, tie/clipped/sparse boundaries; full projection reference and digests; targeted storage/owner/fallback mutations.

- [x] Record measured dominance and exact selected algorithms/interfaces, with normal versus buffered diagnostic limits.
- [x] Add full syntax/projection reference tests and deterministic work/allocation tests; observe missing/old behavior RED.
- [x] Implement minimal selected changes and verify targeted ownership/storage/fallback mutations fail; restore originals.
- [x] Run fmt/check/clippy, workspace and all profiling-feature tests, rustdoc, examples, extractor regressions and normal/profile release builds; freeze optimized identity.

## Task 3 — Compare and publish

**Files:** `docs/phase-08-parse-projection.md`, raw evidence/manifests/audit, README/roadmap links.
**Interfaces:** syntax/projection digests and full-object tests; parse/projection collectors with exact stage presets.

- [x] Run matched normal/profiled after cohorts and controls; audit raw summaries, complete calls, metadata/digests, bounds and identities.
- [x] Report measured changes, remaining costs and limits; commit portable evidence.
- [ ] Obtain one fresh whole-branch review, fix Important findings RED→GREEN/full suite, publish stacked draft above #23 and verify exact-head push/PR three-OS CI.

Final review/CI outcomes belong in the PR and ignored ledger, avoiding status-only source commits.
