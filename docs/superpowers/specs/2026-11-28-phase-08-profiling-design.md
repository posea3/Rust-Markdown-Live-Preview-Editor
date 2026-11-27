# Phase 8 Slice 01 — Measured Windows baseline

The user explicitly authorized merging PRs #16 -> #17 -> #18 -> #19, verifying integrated-main CI, then performing Phase 8 profiling. All four merged in order; integrated main is `0df1961f5cd4f33dd0c8a32749ef82c59d7b75d5`, with successful Windows/macOS/Ubuntu CI run `46023467296`. Its tree equals the previously verified `66fbb0b` tree.

## Measurement choice

Use a release-mode, headless Rust example in the native lab package, reusing current mdedit and cosmic-text dependencies. Instrument exported engine stages and controlled text-buffer shaping with `Instant`/`black_box`. This yields reproducible stage attribution without coupling the engine to a profiler or treating presentation/monitor latency as CPU time. Native input/presentation traces remain a separate follow-up; no GUI/GPU/accessibility acceptance is inferred.

Generate deterministic mixed Markdown at exactly 10,000 / 100,000 / 1,000,000 / 10,000,000 bytes (decimal KB/MB), with Unicode, CRLF, inline styles, tasks, tables, math, code and opt-in Obsidian metadata. The single-line fixture contains exactly 1,000,000 Unicode scalar values with Latin/Korean/emoji, and no line endings. Record actual bytes/chars and projection/fallback/widget/block counts.

Each isolated fixture process uses one warmup and five measured samples. Measure initial document creation, snapshot/parse/extension scan/block reconciliation/delimiter resolution/checked projection, controlled Buffer text replacement and first-viewport advanced shaping, then a one-byte insertion at source offset zero with a full refresh and projected caret-stop construction. BlockCache and the text buffer persist from load to edit; FontSystem is reused within a process. Capture font discovery separately. No speculative optimization or new dependency is introduced.

Text layout uses SansSerif, 22 px / 31 px metrics, Word wrap, a fixed 1000 x 700 viewport. This is base-text CPU shaping, not the lab's complete styled/widget-reservation, resource-rendering, GPU or input-to-display path. Record that distinction. Sum of stage times excludes logging and teardown and is not a frame-latency claim.

A Windows PowerShell runner launches each child hidden, captures raw CSV/stage progress and process peak working set, and limits a fixture to 90 seconds / 1.5 GB observed working set. The child briefly settles after completing measurements so the parent can observe its peak counter. Preserve incomplete data and report timeouts/limits with the last stage; never present censored runs as completed samples. A core-only control may be run when layout prevents attribution. Report measured sample counts and median/min/max for completed rows, not unsupported tail percentiles.

## Scope and delivery

Add a reproducible example/runner and a Windows results report tied to the exact profiled commit/harness. Validate fixture generation and actual executable output, run appropriate full checks, obtain one independent branch review, publish a draft PR and verify its current-head CI. Update Phase 7 integration status and the old CommonMark wording minor while touching those records. Keep local raw evidence under ignored target; include portable raw measurement artifacts needed to reproduce the report, without personal paths or machine identifiers.

The profile is a baseline and prioritization input. Background parsing, incremental projection, virtualization and cache changes require evidence first and are not implemented in this slice.
