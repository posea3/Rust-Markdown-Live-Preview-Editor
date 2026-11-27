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

Comparison results, exact code/environment identities and portable raw data will be recorded before publication. The source owner/range work inside projection and giant-line shaping remain separate measured bottlenecks. This slice does not claim a frozen 1.0 API, new native/IME/accessibility acceptance or an entire Phase 8 completion.
