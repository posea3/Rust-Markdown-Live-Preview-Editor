# Phase 8 measured CPU baseline

Follow-up: [Slice 02 indexed mapping/caret traversal and measured comparison](phase-08-indexed-mapping.md) implements the first experiment below. This report and its archived rows retain the original baseline values.

Phase 7 PRs #16 -> #17 -> #18 -> #19 merged into main in order. Integrated main `0df1961` passed fresh [three-OS CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/46023467296) before this work began. This slice measures existing engine behavior; it does not implement an optimization.

## Reproduce on Windows

Use PowerShell 7 (`pwsh`); the runner declares this requirement before launching a child.

```powershell
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
pwsh -NoProfile -File ./scripts/profile-phase8.ps1 -OutputDirectory target/phase-08-profiling/my-run
```

Use a fresh/empty output directory: each invocation claims it atomically and rejects prior contents, including another disjoint fixture run's aggregate files. It launches only its own child processes hidden and captures their stdout CSV, stage-progress stderr, observed process peak working set, completion/limit status, and measured-row summaries. A `finally` block stops and disposes any owned child still running during normal PowerShell interruption/error unwinding. Force-killing the host itself cannot execute that cleanup. Interrupted runs can retain partial files without final aggregate records; missing completion is not success. Defaults: one warmup, five samples, a nominal 90-second whole-child budget and 1.5 GB observed working-set ceiling. Polling/startup can overshoot the nominal deadline. Resource-limited data remains partial and is not counted as a complete frame.

For the recorded dependency resolution, start in a fresh checkout and copy [`Cargo.lock.snapshot`](performance/2026-11-28-windows/Cargo.lock.snapshot) to `Cargo.lock` before the locked build. The root lockfile is otherwise untracked by this repository. Hardware/fonts and foreground machine activity still affect repeated results.

## Protocol

- Deterministic mixed Markdown: exactly 10,000, 100,000, 1,000,000 and 10,000,000 bytes (decimal KB/MB), with Unicode/CRLF and styles/tasks/tables/math/code/Obsidian metadata. Padding uses ASCII without cutting a UTF-8 scalar.
- Single line: exactly 1,000,000 Unicode scalar values, 2,666,665 UTF-8 bytes, repeating Latin/Korean/emoji with no line endings.
- Initial load and one-byte insertion at source offset zero are separate phases. Each sample starts a fresh document/block cache/buffer; the cache and buffer persist from its load into its edit. The process FontSystem is reused across warmup/samples, with initialization reported separately.
- Timed exported APIs: document creation/transaction, snapshot, parse, opt-in extension scan, block reconciliation, delimiter resolution, checked concealment projection and edited-frame caret stops. Buffer creation/text replacement and advanced first-viewport shaping use current cosmic-text, SansSerif, 22/31 px metrics, Word wrap, 1000 x 700 viewport.
- `Instant` and consumed results measure elapsed wall time of CPU-side calls; logging, source generation and teardown are outside stage timings. Stage sums are not end-to-end frame/input latency or sampled CPU-stack attribution. Report completed stage counts and median/min/max, with no tail-percentile claim from five samples.
- The headless buffer omits the native view's complete styled-line synchronization, widget reservations/resources, GPU rendering, presentation, IME and accessibility paths. Those need separate native profiling.

## Measured Windows results — 2026-11-28

Environment: Intel Core i5-3570 @ 3.40 GHz, 4 logical processors; OS runtime string `Microsoft Windows 10.0.26200`; Rust 1.90.0, x86_64 MSVC; cosmic-text 0.19.0, fontdb 0.23.0, pulldown-cmark 0.13.6, unicode-segmentation 1.13.3. This is one foreground machine, without affinity/power isolation. Font discovery is a separate setup row; it is not included in the following edited-frame timings.

Engine: integrated main `0df1961`. Baseline, supplemental 1 MB and core-only measurements used harness `c037bfd`; policy controls used `21ffaf4`, which adds only benchmark policy selection and untimed map-count metadata. Core editor implementation and dependencies were unchanged. Exact commits, source/runner/binary hashes, per-cohort settings, dependency snapshot and all raw rows are in [the evidence directory](performance/2026-11-28-windows/README.md).

Edited-frame call durations below are milliseconds. Five-observation rows show the median; one-observation rows are individual measurements, not reliable tail estimates.

| Input / evidence | Complete edit observations | Parse | Projection | Caret stops | Base-text viewport shaping |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mixed 10 KB, baseline | 5 | 0.633 | 1.743 | 3.368 | 1.103 |
| Mixed 100 KB, baseline | 5 | 5.941 | 159.924 | 352.399 | 1.439 |
| Mixed 1 MB, supplemental no-warmup run | 1 | 66.672 | 22,892.057 | 44,685.386 | 1.258 |
| Mixed 10 MB, baseline | 0 | — | interrupted during warmup load projection | — | — |
| Million-scalar single line, partial baseline | 1 | 3.169 | 1.453 | 74.246 | 22,907.511 |

The 1 MB supplemental load/edit process completed in 97.63 seconds, using zero warmups, one sample and a 180-second budget. It cannot be treated as a five-sample warmed median. All completed projection metadata reported `raw_fallback=0`.

### Whole-process outcomes and memory

| Baseline input (one warmup + five requested samples) | Outcome | Observed peak working set |
| --- | --- | ---: |
| 10 KB | completed all five measured load/edit pairs | 9.5 MiB |
| 100 KB | completed all five measured load/edit pairs | 21.8 MiB |
| 1 MB | 90.13 s timeout, during sample 0 load projection after warmup | 130.2 MiB |
| 10 MB | 91.10 s timeout, during warmup load projection | 691.2 MiB |
| Single line | 90.08 s timeout, during sample 1 load shaping; sample 0 completed | 872.2 MiB |

These are observed OS process peak counters, including setup, warmup and allocations across samples, not retained editor heap sizes. The 1.5 GB ceiling was not hit in the baseline. A timeout is the whole-process budget, not a measured duration of its last stage. Earlier finished stage rows are retained; `summary.csv` counts completed calls **per stage**, so prefixes from unfinished iterations must not be summed into an apparently complete frame.

### Attribution controls

| Control | Completed observations | Edited projection | Edited caret stops | Meaning |
| --- | ---: | ---: | ---: | --- |
| 100 KB SourceVisible, layout enabled | 5 | 81.529 ms | 4.435 ms | Removing concealment changes map/widget semantics; parsing remains 5.465 ms |
| 1 MB SourceVisible, no layout, no warmup | 1 | 11,993.707 ms | 44.053 ms | Cursor cost collapses but projection remains expensive; this also omits font/layout work |
| Single-line core-only, no layout | 5 | 2.698 ms | 67.173 ms | The full requested run completes in 1.70 s, observed peak 33.5 MiB |

SourceVisible controls confirmed zero concealed ranges, zero projected widgets and no raw fallback. They are different display policies, **not equivalent optimizations or application speedup claims**. The 1 MB control also changes layout participation, so do not attribute its total process or memory difference only to policy. The single-line control isolates the base-text shaping path; it does not make the actual native editor fast without layout.

## Findings and next experiments

1. **Prioritize mapping/caret construction for dense concealment.** From 10 KB to 100 KB, parse time grows about 9.4x, projection about 91.8x and caret construction about 104.6x for 10x bytes on this periodic fixture. This is evidence consistent with near-quadratic work, not a general complexity proof. Source inspection shows two linear `projected_to_source` searches per grapheme in [`ProjectedCaretStops`](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/blob/0df1961f5cd4f33dd0c8a32749ef82c59d7b75d5/crates/mdedit-live/src/caret.rs#L56), over the concealed-range list in [`ProjectionMap`](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/blob/0df1961f5cd4f33dd0c8a32749ef82c59d7b75d5/crates/mdedit-live/src/projection.rs#L762). Indexed lookup or monotone traversal is the first experiment, preserving bias/grapheme/reveal contracts and remeasuring these fixtures.
2. **Profile projection owner/range work separately.** SourceVisible still takes about 12 seconds for 1 MB, so removing cursor searches alone is insufficient. [`build_reveal_groups`](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/blob/0df1961f5cd4f33dd0c8a32749ef82c59d7b75d5/crates/mdedit-live/src/projection.rs#L1161) locates owners for every delimiter; other ownership/shadow/style mapping passes also scan ranges. Those are source-based candidates, not measured internal substage attribution. Instrument or sample them before selecting a projection rewrite.
3. **Investigate giant-line shaping/wrapping.** A viewport height limit did not prevent a roughly 23-second base-text shaping call on the million-scalar line, while the same core-only workload completed five samples quickly. Profile cosmic-text line shaping/layout and evaluate segmentation/windowing with Unicode/BiDi/IME correctness intact; then measure the complete native path.

Background parsing remains part of the roadmap, but parser offloading alone would not address the largest measured costs here. No mapping, projection, parser, layout, cache or native-renderer optimization is implemented in this baseline slice.

## Verification

Windows workspace tests: **294 passed, 0 failed, 1 ignored**; fmt/check/clippy with warnings denied, rustdoc, the three existing public examples and the optimized profiler build passed. The two fixture tests ran RED/GREEN and are included in workspace CI. Actual small-run, one-second timeout and low-memory guard checks exercised the runner. Current-head review/CI outcomes are recorded in the profiling PR and ignored ledger. GPU, native input/presentation, macOS performance and accessibility acceptance retain their separate scope.

Independent review recomputed all 157 raw/summary groups, matching counts/minima/medians/maxima and report tables. Three runner corrections were verified RED/GREEN: rejecting disjoint runs in a used directory without modifying prior records, terminating the exact owned child on interruption, and declaring a PowerShell 7 requirement visible to the 5.1 parser. The machine's 5.1 file-execution policy blocked an end-to-end legacy-host probe; it was respected, and compatibility metadata was checked through that host's read-only parser instead. Archived measurements were collected in PowerShell 7, fresh directories and uninterrupted guarded runs, so these lifecycle fixes do not alter their values or provenance. Repeat the runner regressions after the release build with:

```powershell
pwsh -NoProfile -File ./scripts/tests/profile-phase8-regressions.ps1
```
