# Phase 8 measured CPU baseline

Phase 7 PRs #16 -> #17 -> #18 -> #19 merged into main in order. Integrated main `0df1961` passed fresh [three-OS CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/46023467296) before this work began. This slice measures existing engine behavior; it does not implement an optimization.

## Reproduce on Windows

```powershell
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
./scripts/profile-phase8.ps1 -OutputDirectory target/phase-08-profiling/my-run
```

Use a fresh output directory: the runner refuses to overwrite prior fixture files. It launches only its own child processes hidden and captures their stdout CSV, stage-progress stderr, observed process peak working set, completion/limit status, and measured-row summaries. Defaults: one warmup, five samples, 90 seconds per child and 1.5 GB observed working-set ceiling. Resource-limited data remains partial and is not counted as complete.

## Protocol

- Deterministic mixed Markdown: exactly 10,000, 100,000, 1,000,000 and 10,000,000 bytes (decimal KB/MB), with Unicode/CRLF and styles/tasks/tables/math/code/Obsidian metadata. Padding uses ASCII without cutting a UTF-8 scalar.
- Single line: exactly 1,000,000 Unicode scalar values, 2,666,665 UTF-8 bytes, repeating Latin/Korean/emoji with no line endings.
- Initial load and one-byte insertion at source offset zero are separate phases. Each sample starts a fresh document/block cache/buffer; the cache and buffer persist from its load into its edit. The process FontSystem is reused across warmup/samples, with initialization reported separately.
- Timed exported APIs: document creation/transaction, snapshot, parse, opt-in extension scan, block reconciliation, delimiter resolution, checked concealment projection and edited-frame caret stops. Buffer creation/text replacement and advanced first-viewport shaping use current cosmic-text, SansSerif, 22/31 px metrics, Word wrap, 1000 x 700 viewport.
- `Instant` and consumed results measure calls; logging, source generation and teardown are outside stage timings. Stage sums are CPU-call totals, not end-to-end frame/input latency. Report completed sample counts and median/min/max, with no tail-percentile claim from five samples.
- The headless buffer omits the native view's complete styled-line synchronization, widget reservations/resources, GPU rendering, presentation, IME and accessibility paths. Those need separate native profiling.

## Measurements

The measured results and portable raw CSV will be filled before publication. Original smoke/guard/build/test evidence remains in ignored `target/phase-08-profiling`.
