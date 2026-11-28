# Phase 8 extension scan hotspots design

Continue Slice 04 above draft PR #22, parent `36a6dffeb2ccc3f98e45a0387ecf2651f05fa871`. The completed 10 MB workload now spends about 10.56s in edited extension scanning. Attribute that call before changing its algorithm, then preserve complete scanner results and failure behavior while removing measured repeated work.

## Constraints and choices

Keep public scanner/registration/snapshot APIs, configuration provenance, callback order/count, prevalidation, fallible error and panic propagation, protected-syntax priority, candidate validation, match/attribute ordering and every issue/reported first conflict unchanged. Use Rust 1.90 and current dependencies; retain the untracked root Cargo.lock. Work in `feature/phase-08-extension-hotspots`; publish a stacked draft above #22 and verify final-head three-OS CI without merging parents.

Use an opt-in private `mdedit-markdown/extension-profiling` feature, following the preceding projection work. Time validation/source/protected collection, each registered scanner callback and candidate filtering, pending sort, conflict merge and final sort. Feature-off builds contain no profiling clocks/counters/logs. A feature-only scanner suffix keeps callback/filter identities distinct without allocating formatted labels in default builds. Private instrumentation is preferable here to a new timing API or an unestablished external sampler; marker I/O remains part of featured outer calls, so total comparisons use separately frozen feature-off binaries.

Measure first and select only the dominant passes. Source hypotheses are repeated protected-range searches and accepted-conflict searches; scanner callback work remains a separate candidate if attribution shows it dominates. A full incremental scanner/cache or changed overlap policy would alter lifecycle/semantics beyond this slice. Build-local search data selected from the trace is the preferred bounded change; retain an independent frozen reference and deterministic work tests rather than timing assertions.

## Evidence and correctness

Use the existing release harness/fixtures/layout/policy/edit. Fresh 10/100 KB and 1 MB pairs use one warmup/five samples; 10 MB uses zero warmups/one sample with a 90s process budget and 1.5GB observed working-set ceiling. Core-only million-scalar line remains a control. Capture separate profiled and normal before/after identities, raw rows, dependency/source/binary hashes, counts and completed/censored prefixes. Add untimed issue counts and a streaming deterministic diagnostic digest of ordered matches/issues to the harness; the digest supports comparison, while full snapshot/oracle tests establish correctness.

A reusable stage extractor preserves prior interruption corrections (unsigned newline-terminated durations, torn tails, censored-only output). Match every completed outer scan, including warmups, with one inner invocation; reject wholly absent invocation blocks, closing the relevant preceding audit minor. Preserve historical numerical archives.

Independent tests must cover arbitrary priorities/registration and candidate order, duplicates, equal/crossing/strictly contained/touching ranges, mixed overlap policies/namespaces, UTF-8/CRLF and invalid boundaries, protected-syntax first traversal choice, AllSource, stale metadata/prevalidation, and declared failure/panic callback traces. Compare full ExtensionSnapshot values against a frozen parent reference on deterministic generated inputs and real Markdown, with work-count bounds for large disjoint cases. Native projection/rendering/IME/accessibility and giant-line shaping remain separate; process peaks are not exact retained heap measurements.
