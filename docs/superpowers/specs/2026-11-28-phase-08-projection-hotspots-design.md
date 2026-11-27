# Phase 8 projection hotspots design

## Intent and constraints

Continue the measured Phase 8 work above draft PR #21 (`5cee82218ed27052aa3803535985017ca65d47e6`). Its indexed map/caret changes leave approximately 20s per 1 MB edited projection. Identify internal elapsed costs before changing an algorithm, then preserve complete projection semantics while removing the measured repeated searches. This is Slice 03, not native input/rendering acceptance or a full Phase 8 completion.

Keep public projection signatures, source bytes, fallback/error behavior, reveal group IDs/order, widget ownership/payload/actions, structural padding, styles, blocks and map biases unchanged. Use Rust 1.90/current dependencies. Keep the existing untracked root Cargo.lock; never stage it. Work on `feature/phase-08-projection-hotspots` above #21; publish a stacked draft and verify exact-head three-OS CI without merging the stack.

## Approaches and chosen calls

- An external CPU sampler avoids source markers but this Windows host has no established reproducible sampler setup, and sampling alone does not give completed substage data for censored cases.
- **Choose a private compile-time `mdedit-live/projection-profiling` feature.** Instrument coarse existing build stages with begin/end stderr records, preserving completed prefixes on interruption. Default builds have no clocks/logs or new public profiling API. The serial headless harness's existing outer begin records associate stages with fixture/iteration/phase. Instrumented totals include marker I/O; retain separate feature-off total-call measurements and do not infer native latency.
- A generic incremental cache/projection rewrite would change invalidation and resource lifecycles before attribution. Instead optimize only the dominant measured owner/containment passes using build-local ordered/range data; select the exact pass after the first trace and record the evidence before changing it. If measured dominance differs from source hypotheses, update the plan before implementation.

## Measurement and evidence

Time validation/source materialization, reveal groups, conceal-span collection, structural padding, built-in/extension candidate construction, ownership normalization, shadow filtering, overlap validation, map/materialization, and final blocks/styles/widgets separately. A stderr begin without an end is censored, never a duration. Profile the unoptimized implementation first, freezing a commit and binary identity; retain feature-off same-input comparisons, instrumented before/after traces, settings, raw rows, semantic metadata and dependency identities. 10/100 KB use one warmup/five samples; 1 MB first uses zero warmups/one sample with 180s budget; 10 MB uses explicit 90s bounds. Larger successful optimized runs may add repeated observations, clearly separated from paired cohorts.

## Correctness and scope

Pin index/optimized lookup to a brute-force independent reference, including equal/empty/touching/nested/overlapping/unsorted ranges and first-in-traversal tie behavior. Use deterministic operation counts rather than time thresholds for scalability tests. Preserve all projected fields on a corpus covering Unicode/CRLF, nested Markdown/callouts, widgets, SourceVisible and caret/selection/composition contexts, with explicit non-fallback/widget assertions. Close PR #21's deferred widget-fixture assertion minor in this relevant coverage. Existing exhaustive map/grapheme tests remain mandatory.

Use native inline execution with RED/GREEN evidence and one fresh whole-branch reviewer at the end. Fix Important findings in one verified pass; record deferred minors and decisions. No new dependencies, native GUI/GPU/IME/accessibility claims, giant-line shaping rewrite, or interpretation of process peak working set as exact retained heap savings.
