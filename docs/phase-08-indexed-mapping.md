# Phase 8 indexed mapping and caret traversal

This is Slice 02, above the profiling branch/PR #20 at `6603de6`. The parent remains unmerged. It implements the first experiment from [the original measured baseline](phase-08-performance.md) without changing public map/caret APIs, native rendering, parser/scanner behavior or reveal policy.

## Change and contract

Random source/projected queries now binary-search the existing sorted visible spans. No new prefix-index storage is added. A private common bounds helper preserves leading Before=0, trailing After=source length, both endpoints of interior collapsed ranges, and empty/fully concealed maps. Byte offsets remain byte offsets, including callers querying inside a UTF-8 scalar.

Caret generation supplies ordered grapheme ends to a forward span cursor, sharing the same bounds logic. It creates stops directly rather than collecting/sorting/deduplicating a separate boundary vector or searching twice per stop. Random lookup is O(log V); mapping ordered G boundaries is O(G+V), excluding segmentation/output allocation. A crate-private debug assertion checks the iterator's monotone-input contract.

## Correctness evidence

Tests use an independent mathematical oracle: projected positions count retained bytes; Before/After are the minimum/maximum source positions in each inverse fiber. All 2,047 concealment masks up to ten bytes cover empty/fully hidden, leading/trailing/interior gaps, adjacent/duplicate/unsorted claims, out-of-range queries, repeated and sparse ordered boundaries. A near-u32-limit case validates arithmetic without allocating a giant document.

Public consumers check literal combining/CJK/emoji/ZWJ-family/flag/CRLF caret edges, a grapheme joining across hidden markup, and all byte positions across real Markdown/widget projections with caret/selection/composition reveal contexts. Wrong boundary bias, a single-step sparse cursor and scalar-based segmentation each produced an observed assertion failure; originals were restored before full checks. No clock thresholds are used in CI.

## Comparison protocol

Use the same release fixture generator, Obsidian dialect, one-byte insertion at source zero, concealment policy, fixed base-text layout and dependency resolution as the parent. Run small 10/100 KB five-sample cohorts and the parent's zero-warmup single-sample 1 MB cohort. A core-only single-line control checks the nonfragmented path. Larger cases retain explicit process deadlines and censored outcomes. Do not mistake per-stage prefixes for complete frames or project these CPU elapsed calls into full native latency.

New profiling metadata schema 2 keeps `meta-setup`, `meta-warmup` and `meta-sample` distinct, with a setup `schema_version=2` row. Seven CSV columns and timed `sample` rows remain unchanged, so the current runner summary logic is unchanged. Historical baseline rows stay in legacy format; use their ordering to associate `kind=meta` with its originating iteration. This resolves the parent review's metadata identity minor for new data.

## Results and remaining work

Measured code is `14994398d4e3eb0cffbe67bdd1261905345a866f`, above parent `6603de6`. The comparison uses the parent's archived engine baseline `0df1961` with harness `c037bfd`; parent follow-up commits did not change the engine. Both cohorts use the same dependency snapshot, Intel i5-3570/4 logical processors, Windows runtime `10.0.26200`, Rust 1.90 MSVC, and 1000x700 SansSerif 22/31px Word-wrap layout. Exact identities, settings, all raw rows and comparison groups are in [the evidence directory](performance/2026-11-28-indexed-mapping/README.md).

These are elapsed edited-frame API calls in milliseconds, not native frame latency. Five-observation values are medians; the 1 MB row is one zero-warmup observation. Archived-before and new-after cohorts were sequential, without affinity/power isolation or randomized order; ratios are observations on this machine, not confidence bounds or cross-platform predictions.

| Fixture / protocol | n before / after | Caret before | Caret after | Before / after | Projection before | Projection after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mixed 10 KB, layout, 1 warmup | 5 / 5 | 3.3684 | 0.2429 | 13.87x | 1.7427 | 1.4436 |
| Mixed 100 KB, layout, 1 warmup | 5 / 5 | 352.3991 | 2.3785 | 148.16x | 159.9240 | 110.2336 |
| Mixed 1 MB, layout, 0 warmups | 1 / 1 | 44,685.3859 | 23.0461 | 1,938.96x | 22,892.0572 | 19,987.7769 |
| Million-scalar line, no layout, 1 warmup | 5 / 5 | 67.1727 | 60.0176 | 1.12x | 2.6980 | 3.0989 |

Dense mixed-fixture caret generation now grows approximately 9.8x and 9.7x per 10x bytes in these observations, consistent with the forward traversal. The single-line control has one visible span and had no long span search to remove; its small changes, including higher projection time, are not evidence of a general speedup. Projection still includes owner/range work outside this slice; the observed projection difference is not isolated internal-stage attribution.

The complete zero-warmup 1 MB load/edit child process fell from 97.63s to 43.08s. Its observed OS peak working set was 130.1 MiB before and 132.9 MiB after. The single-line core-only process was 1.70s both times, with 33.5 MiB before and 25.8 MiB after. Those peaks include setup and allocations across the whole child, rather than retained heap measurements; this slice adds no index storage and removes an intermediate caret-boundary vector, but these process counters cannot establish the exact allocation savings.

The bounded 10 MB attempt requested zero warmups and one sample with a 90s whole-process budget. It timed out after 90.25s during the first load projection, with 689.1 MiB observed peak. The preceding scan completed in 14.92s. There is no completed projection, edit or caret result for this fixture. The original 10 MB baseline also timed out during first load projection, but its first iteration was a warmup; no paired timing or memory improvement is claimed for those censored protocols.

All 82 new summary groups were checked against raw sample rows (counts/min/median/max), and all five fixture files had unique schema2 keys. All 281 common semantic metadata rows from the three paired cohorts matched after reconstructing legacy iteration identity from row order: source bytes/scalars, blocks, matches, delimiters, widgets, projected bytes, fallback, layout-run and caret counts. The 1 MB edit still has 47,409 concealed ranges, 47,410 visible spans and 456,944 caret stops, with no raw fallback. Aggregate metadata supports reproducibility; the independent byte/Unicode tests above establish the mapping contract.

Windows workspace verification: **299 passed, 0 failed, 1 ignored**. Format, workspace/all-target check, clippy/rustdoc with warnings denied, all three public examples and the release profiler build passed. The ignored test requires a native graphics adapter. Final independent branch review and current-head three-OS CI are recorded in the PR and execution ledger.

Next, instrument or sample projection owner/range passes before selecting their optimization; the 1 MB projection still takes about 20 seconds per edit. Giant-line shaping/wrapping remains a separate priority from the baseline's approximately 23-second call; layout was deliberately omitted from the new single-line control and that native/layout path was not remeasured. This slice does not claim a frozen 1.0 API, new native/IME/accessibility acceptance or an entire Phase 8 completion. The profiling parent and this slice remain an unmerged PR stack.
