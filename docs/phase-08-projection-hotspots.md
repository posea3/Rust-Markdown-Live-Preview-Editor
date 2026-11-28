# Phase 8 projection attribution and containment optimization

Follow-up: [Slice 04 extension scan attribution and optimization](phase-08-extension-hotspots.md) addresses the next measured scan bottleneck. Values below remain this slice's original observations.

Slice 03 continues above draft PR #21 (`5cee822`), following [indexed mapping/caret traversal](phase-08-indexed-mapping.md). It measures projection internals before selecting the change. The PR stack remains unmerged; this is headless Windows CPU evidence, with native rendering/input and giant-line shaping still separate.

## Attribution before optimization

Instrumentation-only commit: `87cf92732707300f0121de7d6c6e061565575863`. The opt-in `mdedit-live/projection-profiling` feature marks 15 existing build stages on stderr with `projection-begin,<call>,<stage>` and `projection-end,<call>,<stage>,<ns>`. Default builds contain no profiling clocks, counters or logs, and public projection signatures are unchanged. Existing outer `begin,<fixture>,<kind>,<sample>,<phase>,projection` records associate inner markers in the serial headless harness. A begin without an end is censored work, not a duration.

The first measured 1 MB edited projection was dominated by repeated containment searches. The following feature-enabled observations are milliseconds; each side is one zero-warmup sample. Index construction is charged to `reveal_groups`; shadow/normalization measurements also include their new index construction. No index cost is hidden in setup.

| Internal stage, 1 MB edit | Before | After |
| --- | ---: | ---: |
| Reveal groups / owner lookup | 13,529.6509 | 35.6938 |
| Widget shadow filtering | 3,001.7534 | 2.4736 |
| Extension widget candidates | 431.7182 | 6.3882 |
| Widget ownership normalization | 60.7152 | 0.9114 |

These rows come from separate instrumented before/after runs. Marker I/O is outside each inner interval but inside the outer projection call. Host variation also affects these elapsed observations: do not subtract feature-enabled and feature-off results to infer instrumentation overhead. The total-call comparison below uses only feature-off binaries.

## Change and semantics

One build-local `OwnerLookup` indexes syntax nodes in a balanced interval tree. Effective bounds intersect ancestor ranges, matching the original traversal's pruning; original node range length chooses the smallest owner, and preorder breaks equal-length ties. Sorted starts and subtree maximum ends prune disjoint branches. The same index is reused for callout lookup; no revision/configuration cache persists between builds. Building costs O(N log N) and O(N) storage; querying is output-sensitive when many overlapping/nested nodes contain a range. The broad-tree work test establishes a bounded disjoint-query case rather than claiming every nested input has logarithmic cost.

Build-local `ContainmentSet` instances sort range starts and retain prefix maximum ends. Any-containing queries use a binary upper-bound in O(log W). They replace repeated scans during extension ownership normalization and delimiter/padding shadow filtering. Active callout/embed owners still suppress contained built-in widgets and conceal spans as before; partial overlaps and unrelated ranges remain distinct. Both structures use existing dependencies and references to existing syntax nodes, without cloning node metadata.

## Feature-off total-call comparison

Before is the unchanged algorithm at `87cf927`; after is `dc8415e585e526d77076ca8cf3408d63ab59b021`. Both variants were built and frozen separately from the same Rust 1.90 MSVC toolchain and dependency snapshot. All runs use Obsidian / ConcealInactive, one ASCII `x` inserted at source zero, and the existing fixture generator. Layout-enabled runs use 1000x700 SansSerif 22/31px Word-wrap/Advanced shaping. Environment: i5-3570 / four logical processors, Windows runtime `10.0.26200`. Runs were sequential on one foreground machine without randomized order or affinity/power isolation.

| Edited projection call | n before / after | Before ms | After ms | Before / after |
| --- | ---: | ---: | ---: | ---: |
| Mixed 10 KB, layout, 1 warmup | 5 / 5 | 1.3938 | 0.6338 | 2.20x |
| Mixed 100 KB, layout, 1 warmup | 5 / 5 | 98.3747 | 7.1503 | 13.76x |
| Mixed 1 MB, layout, 0 warmups | 1 / 1 | 14,064.1557 | 88.0626 | 159.71x |
| Million-scalar single line, no layout, 1 warmup | 5 / 5 | 2.6364 | 2.6238 | 1.00x |

Five-observation rows are medians; the 1 MB row is an individual observation, not a tail estimate. The independent warmed **after-only** 1 MB run completed all five load/edit samples: edited projection median 80.4580ms, min 78.2391ms, max 82.2298ms. It is not paired with the zero-warmup before observation. The core-only line is an unchanged-policy control with few syntax nodes, not evidence that giant-line layout was fixed.

The matched 1 MB zero-warmup whole child fell from 29.73s to 2.07s. Process peaks were 133.3 MiB before and 137.7 MiB after; the new build-local indexes add storage. These OS working-set peaks include setup, allocations and teardown, not exact retained heap or index allocation measurements. Small-case process totals need not track projection calls: 10 KB was 1.52s before and 1.94s after, despite its smaller projection call. No whole-application latency or general process/memory speedup is inferred.

## Bounded 10 MB outcome and remaining costs

The optimized 10 MB run, zero warmups / one sample / 90s process deadline / 1.5 GB working-set ceiling, completed its load/edit pair in **26.82s**, with **1,077.7 MiB (about 1.05 GiB)** observed process peak. Its edit calls were scan 10,563.4590ms, projection 901.2296ms, parse 744.5082ms and caret stops 234.8382ms. No raw fallback occurred.

The [previous same-policy 10 MB attempt](performance/2026-11-28-indexed-mapping/large-bounded/processes.csv) stopped during first load projection at the 90s budget. It has no complete pair or projection duration, so there is no paired speedup ratio. Its censored peak cannot be compared to the now-completed child's peak as an allocation regression or saving.

Extension scanning is now the largest measured 10 MB edit stage. Instrument its scanning/protection/merge work before choosing that optimization; these outer timings do not attribute its internals. Giant-line shaping/wrapping and native styled-line/widget layout, GPU/presentation, IME and accessibility remain separate acceptance work. This slice does not complete Phase 8 or declare a frozen 1.0 API.

## Correctness and verification

- Exhaustive any-containing equivalence across all 32,768 subsets of the small-range universe, including reversed/duplicated/empty/touching queries and u32 endpoints.
- Independent brute-force node oracle covering mixed kinds, unsorted/nested/clipped bounds, equal lengths and duplicate ranges, with pointer identity pinning first traversal choice.
- All 4,096 disjoint broad-tree queries visit at most 52 index nodes. Removing subtree pruning fails this deterministic work assertion, without a wall-clock threshold.
- Parsed owner queries and **70 complete Projection comparisons** against the frozen parent's brute-force builder, covering Unicode/CRLF, widgets, nested/repeated callouts, both policies and caret/selection/composition contexts. Equality includes text/status, IDs/ranges, blocks/styles, widgets/payload/actions, map, reveal groups, concealment and padding.
- Two inactive overlapping-callout fixtures already produced `RawFallback(OverlappingConcealment)` in the parent. Their literal fallback statuses remain pinned; the other corpus contexts stay projected. Improving those overlaps is a separate behavior change.
- Wrong inclusive containment, lost preorder tiebreak, changed active-extension shadowing and removed subtree pruning each produced an observed assertion failure; correct code was restored. PR #21's deferred public-fixture minor is closed by explicit non-fallback/four-widget/nonempty-concealment assertions.

Windows workspace: **303 passed, 0 failed, 1 ignored** (native graphics-adapter test). Format, workspace/all-target check, default and profiling-feature clippy with warnings denied, profiling-feature tests, rustdoc with warnings denied, all three public examples and both release variants passed. CI now checks profiling-feature clippy and tests separately on all three operating systems. Final independent review and exact-head CI outcomes are recorded in the PR and execution ledger.

Independent review found two trace-extraction interruption defects: a torn empty duration could become a measured zero, and a first-stage-only interruption lost its censored export. Both reproduced before correction. The extractor now requires complete unsigned duration records, preserves torn final completions as censored, and emits header-only timing CSVs when zero stages finish. Seven regression cases pass and run in Windows CI; regenerating the archive leaves all measured values unchanged. A separate minor—cross-checking outer warmup invocations against entirely missing inner blocks—is recorded for follow-up audit hardening.

[Portable raw evidence, manifests, protocol and audit helpers](performance/2026-11-28-projection-hotspots/README.md) retain all 316 outer and 182 internal summary groups, 225 paired timing groups and 1,129 matching metadata-row comparisons. All 17 fixture CSVs have unique schema2 keys. An actual three-second interrupted unoptimized run retained two completed internal stages and one censored reveal stage; a mismatched completion ID was also rejected by the trace extractor. Aggregate metadata is reproducibility evidence; the independent full-field/oracle tests establish semantic equivalence.
