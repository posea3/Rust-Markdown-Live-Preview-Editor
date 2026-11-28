# Phase 8 parse and projection attribution and optimization

Slice 05 follows [extension scan optimization](phase-08-extension-hotspots.md), above draft PR #23 at `933c16f`. It measures remaining parse/projection work and changes local tree storage and ownership indexes. The stack remains proposed and unmerged; these are headless Windows release measurements.

## Attribution and implementation

Instrumentation-only before commit: `c81f6b34e607a516964e5d0ed8d83fe987edd72c`. The opt-in `mdedit-markdown/parse-profiling` feature separates source copying, parser initialization, lazy event decoding and syntax-tree construction. It **buffers events before constructing the tree**, unlike the unchanged default streaming path. Buffer allocation, changed interleaving, marker I/O and an untimed structural-statistics traversal affect its outer parse/process measurements. Its decomposition does not estimate normal parser substage latency. Separate feature-off binaries establish the comparison below.

The existing opt-in projection feature now separates owner-index construction, delimiter grouping and reveal-group materialization. Its other stages remain distinct, giving 17 stages per completed projection. A compile-time macro removes these new labels, closures and clocks from default builds. Both diagnostic features keep public APIs unchanged. The historical extractor's projection15 preset is retained; new data explicitly supplies projection17, and the new parse preset has four stages.

The before 10 MB diagnostic edited call identified tree construction at 344.9857ms, event decoding at 396.4424ms and initialization at 266.7008ms; projection grouping at 286.0357ms and owner indexing at 114.8764ms. Parse statistics recorded 1,982,739 nodes, 4,317,352 allocated child slots and 517,236 single-child nodes. This attribution selected two local changes, measured at **`cdd83e5584c4ced4d400a33c854936599e5aef0e`**:

- `SyntaxNode::push_child` gives commonly narrow containers an initial capacity of one, and list/table containers two, retaining ordinary geometric growth thereafter. Paragraph/document/list-item storage keeps normal Vec growth. Nodes, ranges, metadata, child order and `const new` remain unchanged. Wide containers can require another allocation; the hint is not a fixed arity constraint.
- `OwnerLookup::for_kinds` builds separate interval trees only for requested delimiter-owner kinds, always including BlockQuote for extension-only callouts. Traversal still intersects every ancestor's bounds, and relative preorder still resolves same-kind/equal-length ties. Indexing avoids unused text/container nodes; grouping queries prune within one kind's tree. Construction/sorting and transient storage are charged to the current projection call; nothing is cached across revisions/configurations. Storage is O(selected nodes), construction O(all nodes × requested kinds + selected sorting), and queries retain output-sensitive behavior rather than a universal logarithmic bound.

The edited diagnostic tree keeps the same node/event/single-child counts and uses **2,679,441 child slots**, 37.94% fewer. Load uses 2,679,438. This is allocated element capacity, not exact heap bytes or retained-memory attribution. A controlled lookup test also retains fewer than half the generic index entries while checking queries against the parent lookup.

## Normal release comparison

Both sides use Rust 1.90 MSVC, the existing dependency snapshot/generator, Obsidian/ConcealInactive, insertion of ASCII `x` at source zero and identical base-text layout. Machine: Intel i5-3570, four logical processors, Windows runtime `10.0.26200`. Runs were serialized after compilation, without randomized order, affinity or power isolation.

| Edited call | n before / after | Parse before / after ms | Projection before / after ms |
| --- | ---: | ---: | ---: |
| Mixed 10 KB, layout, 1 warmup | 5 / 5 | 0.5381 / 0.6246 | 0.6274 / 0.5099 |
| Mixed 100 KB, layout, 1 warmup | 5 / 5 | 5.8397 / 4.9330 | 8.7600 / 5.2698 |
| Mixed 1 MB, layout, 1 warmup | 5 / 5 | 58.0352 / 54.4269 | 78.6155 / 57.8548 |
| Mixed 10 MB, layout, 0 warmups | 1 / 1 | 608.6171 / 591.4770 | 927.0777 / 653.2208 |
| Million-scalar line, no layout, 1 warmup | 5 / 5 | 4.1699 / 3.8858 | 2.5733 / 2.6042 |

Five-observation rows are medians; 10 MB is one observation, not a tail estimate. Projection fell 26.41% on the warmed 1 MB case (before range 78.2664–80.0054ms; after 57.2457–58.7298ms) and 29.54% on the bounded 10 MB case. Normal parse gains were modest: 6.22% at 1 MB and 2.82% in the single 10 MB observation. **The 10 KB parse median rose 16.08%** (before range 0.5033–0.8272ms; after 0.5948–0.6793ms), and control projection rose 1.20%. These data do not establish a universal parse speedup or a statistical significance claim.

The normal 10 MB child completed in **9.0355s before / 8.3632s after**. Its observed OS peak working set was **1,063.8 / 851.2 MiB**. The buffered diagnostic child completed in 10.6280 / 8.6558s, with peak 1,340.9 / 1,134.4 MiB. All main cohorts completed inside 90s / 1.5GB bounds. Process totals include setup, untimed digests and teardown; peaks include other phases and cannot isolate exact index/tree heap savings. The first before-profile 10 KB child had a 16.1209s startup-inclusive total; it remains in the archive.

Matched diagnostic 10 MB edit observations below are milliseconds, one sample per side:

| Internal stage | Before | After |
| --- | ---: | ---: |
| Parse: initialization | 266.7008 | 259.8741 |
| Parse: event decoding | 396.4424 | 280.5798 |
| Parse: tree construction | 344.9857 | 268.5687 |
| Projection: owner index | 114.8764 | 42.8502 |
| Projection: delimiter grouping | 286.0357 | 152.0308 |
| Projection: extension candidates | 73.7940 | 40.8685 |
| Projection: styles | 134.6304 | 75.0053 |

Vendor initialization/decoding and style/candidate traversal algorithms were not rewritten. In particular, the faster diagnostic decode observation cannot be attributed to the later child-capacity change: environment/cache effects and changed tree memory behavior remain confounders. Do not subtract diagnostic stages or featured/unfeatured totals to infer streaming parser work or feature overhead.

The optimized normal 10 MB edit still spends projection 653.2208ms, parse 591.4770ms, caret construction 237.7460ms and scan 185.0326ms. Diagnostic vendor initialization/decoding remain substantial. Further parser or incremental-work changes need their own measurements and semantic/fallback plan. Giant-line wrapping/shaping, native styled/widget layout, GPU/presentation, IME/accessibility and exact memory attribution remain separate; this does not complete Phase 8 or freeze a 1.0 API.

## Semantics and verification

- **768 full syntax snapshot equalities** against the frozen parent's streaming parser span CommonMark/GFM/Obsidian, generated markup, metadata, Unicode and CRLF. Full projection values continue to match the earlier independent parent/reference implementation across reveal policies, caret, selection and composition contexts.
- Malformed UTF-8 ranges, unbalanced/mismatched/unclosed events preserve the first fallback reason and early consumption in the builder. A terminal invalid event leaves later events unconsumed.
- Storage tests keep const construction, 257-child order/growth and bounded aggregate capacity for 512 single-child containers. The original implementation failed with 2,048 slots against the 1,024-slot bound.
- Requested-kind owners match generic/brute-force owners, including parsed boundaries and the existing synthetic clipped/tied/extreme-range cases. An explicit extension-only callout with no delimiter requests compares the entire projection with the parent.
- Removing the callout selection, reversing the owner tie and changing fallback reason each caused an assertion failure; restored code passed all checks. The missing requested-kind interface also observed RED before implementation.

Windows workspace/default and all-features runs each passed **315 tests, 0 failed, 1 ignored** (native graphics-adapter test). fmt/check, default/all-feature clippy with warnings denied, rustdoc with warnings denied, three public examples, **21 extractor regressions** and both release builds passed. CI adds parse-feature clippy/tests on all three OSes. Independent final review and exact-head CI outcomes are recorded in the PR and ignored ledger.

Independent review reproduced three interrupted-tail ambiguities: a `p` from the other subsystem, a `parse-` statistics prefix after a complete parse, and a `parse-` end prefix with a known stage pending. All three observed RED. Context-aware prefix handling preserves the completed measurements and only censors a known pending stage; explicit extra/nested begin records remain rejected. All 21 regressions pass. Regenerating current traces with the correction leaves all 61 numerical CSV hashes unchanged; measured engine binaries remain unchanged. The manifest retains the producer-era extractor hash and adds the corrected extractor hash.

The harness adds untimed streaming FNV-1a Debug digests for complete syntax/projection values, as two u32 halves, alongside the existing scan digest and counts. They avoid a large temporary String and are specific to this source/toolchain, not cryptographic, stable serialization or a public cache key. Full-value reference tests provide semantic proof; matching digests support the measurements.

[Portable evidence and audit](performance/2026-11-28-parse-projection/README.md) validate **352 outer / 339 internal summary groups, 343 paired timing groups, 19 unique-key fixture files and 2,745 matching metadata rows**. Every one of the **152 completed profiled outer parse/projection invocations**, including warmups, has a complete, ordered matching inner call. No completed benchmark projection used raw fallback. An actual before-profile 10 MB/no-layout/1s interruption retained three completed parse stages and a censored `build_tree`, with no invented tree duration. The two preceding deferred minors remain: stale derived censor CSVs after replacing raw inputs in place, and extension-profile labels/enumeration retained only in default debug libraries. Fresh directories and frozen normal release measurements avoid those limitations here.
