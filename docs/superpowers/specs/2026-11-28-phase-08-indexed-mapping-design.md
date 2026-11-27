# Phase 8 Slice 02 — Indexed coordinate mapping

The user requested the next step after the measured baseline. Base is `6603de6` (draft PR #20, still unmerged). Measured dense-concealment cursor construction is 352 ms at 100 KB and 44.7 s in a 1 MB observation. This slice implements the report's first experiment, preserving current public APIs and all byte/bias/grapheme semantics.

## Chosen approach

Reuse `ProjectionMap::visible_spans`, already sorted by source and projected coordinates, for logarithmic random coordinate queries. A new prefix array would duplicate information and allocate per concealed range. A cursor-only workaround would leave random queries in block/style/widget mapping linear. Reusing existing spans improves both query directions without an added index allocation.

For projected queries, find the first span whose projected end is not before the position. A private common bounds helper returns the source endpoints on both sides of a collapsed boundary: leading Before is zero, trailing After is source length, interior Before/After are the adjacent visible source endpoints. Entirely concealed and empty documents retain their endpoint behavior. Source queries find the first visible source span ending at/after the position and clamp a concealed-gap position to that span's projected start; after all spans return projected length.

Caret positions are already monotonically increasing grapheme ends. Add a crate-private ordered boundary iterator that advances through visible spans once and shares the same bounds helper. Construct stops directly without sorting/deduplicating/materializing an intermediate boundary vector. Random lookup is O(log V); bulk mapping is O(G+V), apart from segmentation/output allocation. Public shapes, source/projection metadata and native-view behavior remain unchanged.

## Required evidence

- Exhaustively compare all concealment masks for small maps against a mathematical prefix/inverse oracle; include leading/interior/trailing collapse, empty/fully hidden maps, adjacent/overlapping/unsorted claims, out-of-range offsets and sparse/duplicate ordered queries.
- Exercise near-u32-limit arithmetic without giant allocation. Public consumer tests cover exact combining/CJK/emoji/ZWJ/flag/CRLF boundaries and real core/Obsidian projections under reveal contexts. Preserve arbitrary byte-offset mapping behavior, not only UTF-8 boundaries.
- Observe absent bulk API RED, GREEN after implementation; mutation checks must expose boundary-bias and ordered-jump defects. No elapsed-time CI thresholds.
- Fix the prior profiling metadata iteration-identity minor when producing comparison data: new metadata rows retain their setup/warmup/sample identity; preserve archived legacy rows and document the format transition.
- Run full Windows checks and release measurements on identical fixtures/policies, recording actual sample counts, censored cases and exact engine/harness identities. Reuse baseline results; do not rerun expensive old code without need. Native input/GPU/accessibility and giant-line shaping remain separate work.
- Publish a stacked draft above #20, one fresh independent review, important-finding fix pass if needed, and exact-head Windows/macOS/Ubuntu CI. Do not merge #20 implicitly.

Remaining projection owner/range and giant-line layout costs are reported honestly; this slice does not promise that all large-document frames fit a latency budget.
