# Phase 8 extension scan attribution and optimization

Slice 04 follows [projection containment optimization](phase-08-projection-hotspots.md), above draft PR #22 at `36a6dff`. The unmerged stack now attributes and optimizes the large extension scan call. These are headless Windows release measurements, with native rendering/input, giant-line shaping and exact heap attribution still separate.

## Attribution and selected change

Instrumentation-only before commit: `35b384174c973a3d3b0d2927f29de45edb7d1236`. The private opt-in `mdedit-markdown/extension-profiling` feature records validation, source text, protected collection, each scan callback and candidate filter, pending sort, conflict merge and final sort. The example registers one Obsidian scanner, so its per-scanner names are `scanner_0` and `candidate_filter_0`. Feature-off builds contain no profiling clocks/logging or formatted labels. Optimized release removes profiling-only literals/registration enumeration; debug libraries still retain those minor artifacts. Public scanner APIs stay unchanged.

The measured 10 MB edited call identified two dominant repeated searches. Feature-enabled internal observations below are milliseconds, one zero-warmup sample per side:

| Internal stage | Before | After |
| --- | ---: | ---: |
| Conflict merge | 8,213.5283 | 48.2303 |
| Candidate validation / core protection | 2,958.1232 | 41.8405 |
| Obsidian scan callback | 121.8878 | 142.4816 |
| Protected-range collection | 44.2962 | 47.1895 |

Callback implementation and collection traversal are unchanged. The feature-off total-call comparison below uses separate binaries: internal timers exclude marker I/O, while featured outer calls include it. Configuration capture, scan_mode dispatch and other bookkeeping also remain in outer timing. Do not subtract featured/unfeatured observations to estimate instrumentation overhead.

A build-local `OverlapIndex` uses a balanced topology over the known ranges. Activation updates subtree minimum payload ordinal, minimum active start and maximum active end. Queries prune disjoint/later subtrees and select the earliest original ordinal satisfying the unchanged conflict predicate. Protected payload ordinals retain first syntax traversal precedence; accepted payload ordinals retain first acceptance precedence, independently of source order. Only accepted candidates activate conflict slots, so a rejected candidate's tail cannot suppress later claims.

Protection indexing is lazy after the first valid MarkdownText candidate; AllSource/no-valid-candidate paths avoid that index construction. It is shared across registrations in the current scan. Construction/update cost is charged to candidate filtering; dynamic topology and activation cost are charged to conflict merge. No revision/configuration state is cached between calls. Building is O(N log N), activation O(log N), with O(N) extra storage. Queries remain output-sensitive; heavily nested AllowContained claims can still require many predicate checks. The disjoint work bound below is not a universal logarithmic guarantee.

## Feature-off comparison

Optimized measured code: `245b41cf30917e2e0980cbf920b8808b6abd3165`. Before/after use the same Rust 1.90 MSVC toolchain, dependency snapshot, generator, Obsidian/ConcealInactive policy, insertion of ASCII `x` at source zero, and fixed base-text layout (1000x700, SansSerif 22/31px, Word wrap/Advanced shaping). Environment: Intel i5-3570, four logical processors, Windows runtime `10.0.26200`. Cohorts were serial, without randomized order or affinity/power isolation.

| Edited scan call | n before / after | Before ms | After ms | Before / after |
| --- | ---: | ---: | ---: | ---: |
| Mixed 10 KB, layout, 1 warmup | 5 / 5 | 0.1772 | 0.2142 | 0.83x |
| Mixed 100 KB, layout, 1 warmup | 5 / 5 | 1.9794 | 2.0258 | 0.98x |
| Mixed 1 MB, layout, 1 warmup | 5 / 5 | 88.7765 | 20.5512 | 4.32x |
| Mixed 10 MB, layout, 0 warmups | 1 / 1 | 11,192.4673 | 281.1563 | 39.81x |
| Million-scalar single line, no layout, 1 warmup | 5 / 5 | 5.7412 | 5.9675 | 0.96x |

Five-observation rows are medians; the 10 MB row is one observation, not a tail estimate. Small/control cases did not improve: new lookup work is not a universal speedup claim. The warmed 1 MB after range was 20.2337–21.4416ms, versus 87.9085–90.6441ms before. Whole 1 MB child totals were essentially unchanged (5.17s/5.16s); other stages, setup, diagnostics and teardown remain significant.

The complete zero-warmup 10 MB load/edit child fell from **27.22s to 7.55s**. Observed OS peak working set was **1,078.4 MiB before / 1,061.5 MiB after**; both completed within 90s / 1.5GB bounds. New indexes allocate transient O(N) storage. Whole-process peaks include other phases and cannot establish exact index allocation costs, retained heap savings or a memory improvement. The profiled 10 KB before child also had a 15.26s startup-inclusive total; its recorded rows remain in the archive rather than being discarded as an outlier.

The optimized 10 MB edit now measures projection 1,051.3447ms, parse 749.7563ms, caret construction 290.3267ms and scan 281.1563ms. These are individual API wall calls, not native frame latency. Projection/parse now exceed scan in this observation. Giant-line shaping/wrapping and native styled/widget layout, GPU/presentation, IME/accessibility and memory attribution retain their separate profiling/acceptance scope; this does not complete Phase 8 or freeze a 1.0 API.

## Semantics and verification

- All 32,768 small-range activation subsets, reversed activation order, predicate filters, empty/touching ranges and u32 endpoints match an independent strict-overlap/first-ordinal scan.
- All 4,096 disjoint reverse-activated queries visit at most 80 tree nodes, with no wall-clock test threshold.
- **512 generated full ExtensionSnapshot equalities** against the frozen parent's scanner, plus six real/custom corpora, pin namespace/priority/range/policy/attribute ordering, UTF-8 invalid ranges, protected syntax and complete issues/configuration provenance.
- Explicit diagnostics pin first acceptance rather than first source position, first protected syntax kind, invalid-range-before-protection precedence and rejected-tail independence.
- Declared errors, panics and prevalidation compare complete callback mode/scan traces with the parent; later callbacks stay uncalled after failure, and existing error identity/side effects remain observable.
- Inclusive overlap, first traversal hit instead of earliest ordinal, rejected-range activation and disabled pruning each caused a real assertion failure. Correct code was restored before full verification.

Windows workspace: **309 passed, 0 failed, 1 ignored** (native graphics-adapter test). fmt/check, default and extension-feature clippy with warnings denied, both extension/projection feature tests, rustdoc with warnings denied, three public examples and both release builds passed. CI adds extension-feature clippy/tests on all three OSes and the generic extractor regressions on Windows. Final independent review/current-head CI outcomes are recorded in the PR and ignored ledger.

The harness adds untimed issue counts and a streaming FNV-1a diagnostic digest of ordered Debug matches/issues, exported as two u32 halves. It avoids a large temporary formatting String. The digest is pinned to this source/toolchain, excludes configuration/revision identity, is not cryptographic or a persistent cache key, and supports reproducibility rather than proving equivalence; full snapshot/oracle tests provide that evidence. Diagnostic computation is outside timed calls and inside whole-process totals on both sides.

The reusable [extractor](../scripts/extract-phase8-stages.ps1) retains torn-tail/unsigned-duration/censored-only handling and now matches **every completed outer invocation, including warmups**, to one complete inner invocation. A whole missing warmup test observed RED before correction, alongside the seven preceding interruption cases; all eight pass. This closes the preceding relevant audit minor for the new workflow without rewriting historical archives. An actual unoptimized 10 MB / no-layout / 3s control retained four completed stages and one censored candidate filter, with no fabricated filter duration.

Independent review reproduced three additional extractor gaps: accepting a shortened completed call after a later timeout, accepting an unordered/changed-ID unfinished prefix, and losing observations at a torn begin marker. Six added regressions observed RED; all **14 cases pass** after full completed-call validation, ordered single-ID open prefixes and conservative unknown-tail preservation. A partial begin name is counted as unidentified, without inventing a censor identity. Regenerating all current traces leaves numerical values unchanged. Two minors remain deferred: stale derived censor files when replacing raw logs in the same directory, and debug-only profiling literals/enumeration. Use fresh directories for changed inputs; the current archive has no stale output and measured normal release omits the diagnostic labels.

[Raw evidence, manifests, protocols and audit helper](performance/2026-11-28-extension-hotspots/README.md) retain 353 outer / 132 internal summary groups, 239 paired timing groups, 19 unique-key fixture files and 2,089 equal metadata-row comparisons, including ordered result digests. All 76 completed profiled outer calls, including warmups, have exactly one full inner invocation. Completed benchmark projections reported no raw fallback.
