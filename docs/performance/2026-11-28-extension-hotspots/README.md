# Extension scan Windows evidence

See [the report](../../phase-08-extension-hotspots.md). Before `35b384174c973a3d3b0d2927f29de45edb7d1236` adds only instrumentation/diagnostics; optimized code is `245b41cf30917e2e0980cbf920b8808b6abd3165`, stacked above PR #22 / `36a6dff`. `environment.json` pins four binary hashes, source/profile/harness/runner/extractor/lock hashes and CPU/OS/toolchain. The matching [Cargo.lock snapshot](../2026-11-28-windows/Cargo.lock.snapshot) is shared with the prior measurements. Values/logs are copied unchanged; Git may normalize text line endings.

| Directory family | Fixtures | Layout | Warmups / samples | Process deadline |
| --- | --- | --- | --- | --- |
| `normal-{before,after}-warmed` | mixed 10 KB, 100 KB, 1 MB | yes | 1 / 5 | 90s each |
| `profile-{before,after}-warmed` | mixed 10 KB, 100 KB, 1 MB | yes | 1 / 5 | 90s each |
| `normal-{before,after}-10mb` | mixed 10 MB | yes | 0 / 1 | 90s |
| `profile-{before,after}-10mb` | mixed 10 MB | yes | 0 / 1 | 90s |
| `normal-{before,after}-core` | million-scalar line | no | 1 / 5 | 90s |
| `profile-censored` | mixed 10 MB, **before** binary | no | 0 / 1 | 3s |

`profile-*` enables only `mdedit-markdown/extension-profiling`; `normal-*` enables neither profiling feature. All use Obsidian / ConcealInactive, ASCII `x` at source zero, 1.5GB observed working-set ceiling, 200ms untimed settle, and unchanged generator/layout. Runs were serial after compilation with no randomized order or affinity/power isolation. Small/control calls did not improve; one-sample results are not tail estimates.

## Files and checks

- `<fixture>.csv` uses the existing schema2 outer timing/metadata columns. New `issues_count` and `scan_digest_hi/lo` rows are untimed. The streaming diagnostic digest covers ordered Debug matches/issues, excluding configuration/revision, and is pinned to source/toolchain. It is not cryptographic, a stable API/cache key or a substitute for full snapshot equality tests.
- `<fixture>.stages.log` contains outer `begin,...,scan` and inner `extension-begin,<call>,<stage>` / `extension-end,<call>,<stage>,<ns>` records. Callback/filter suffixes are registration ordinals. This is serial headless diagnostic data, not concurrent production telemetry.
- `summary.csv` aggregates completed outer `kind=sample` calls in milliseconds; setup, warmup and metadata are excluded. A prefix is not a complete frame.
- `extension-stages.csv` retains completed inner context/call/stage/ns records; `extension-summary.csv` aggregates samples. `extension-censored.csv` exists only for the interrupted control: validation/source/protection/callback completed, filter began without ending and has zero duration observations.
- `processes.csv` records completion/timeout, process elapsed time, observed OS peak working set, settings and last outer stage. All main cohorts completed. Peaks include setup/other phases; they are not exact heap/index costs.
- `comparison.csv` contains 239 count-matched groups: 95 normal outer, 80 profiled outer and 64 profiled inner. Ratios use cohort medians, preserving n=1; a zero denominator is left null.
- `verification.json` records 353 outer / 132 internal summaries recalculated from raw ns, 19 unique-key CSVs, 2,089 matching metadata comparisons and 76 complete outer/inner links including warmups.

`audit-evidence.ps1` checks raw counts/min/median/max, metadata/digests, paired groups, absence of default markers, interrupted prefixes and complete outer/inner invocation links. In particular, removing a whole warmup inner block cannot escape the per-invocation check. It writes derived comparison/verification files alongside this archive:

```powershell
./docs/performance/2026-11-28-extension-hotspots/audit-evidence.ps1
./scripts/tests/phase8-stage-extraction.ps1
```

The [reusable extractor](../../../scripts/extract-phase8-stages.ps1) regenerates inner files from original logs and cross-checks completed outer rows, including warmups. Default `-Subsystem extension` expects the eight stages for the harness's one scanner. Custom scanner counts/stages require explicit `-ExpectedStages`. `-Subsystem projection` supports the preceding 15-stage producer. Unsigned newline-terminated completions are required; an interrupted torn tail stays censored. A first-stage-only interruption produces header-only timing CSVs and a censor record, with no observations. Eight regression cases include these interruption conditions and wholly missing warmup data; scratch evidence stays under ignored `target`. Historical numerical archives are unchanged.

## Reproduction

Use fresh checkouts at each measured commit, PowerShell 7, and the dependency snapshot. Build/freeze variants before starting serialized runs:

```powershell
Copy-Item docs/performance/2026-11-28-windows/Cargo.lock.snapshot Cargo.lock
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
Copy-Item target/release/examples/phase8_profile.exe target/normal.exe
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile --features mdedit-markdown/extension-profiling
Copy-Item target/release/examples/phase8_profile.exe target/profile.exe
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-10kb,mixed-100kb,mixed-1mb -OutputDirectory target/repeat-normal-warmed
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-10mb -Samples 1 -Warmups 0 -OutputDirectory target/repeat-normal-10mb
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-10kb,mixed-100kb,mixed-1mb -OutputDirectory target/repeat-profile-warmed
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-10mb -Samples 1 -Warmups 0 -OutputDirectory target/repeat-profile-10mb
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-warmed
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-10mb
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures single-line -NoLayout -OutputDirectory target/repeat-core
```

The before/censored control uses the before profile binary, mixed-10mb, `-NoLayout -Samples 1 -Warmups 0 -TimeoutSeconds 3`. Every output directory must be fresh. Instrumented outer timing includes marker I/O and labels; internal timing does not. Configuration/scan_mode and other bookkeeping remain in outer calls. Whole-process timing includes untimed result diagnostics, font setup and teardown. These Windows elapsed calls do not establish native input-to-display latency, exact heap savings or predictions for other platforms.
