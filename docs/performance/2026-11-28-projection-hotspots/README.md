# Projection hotspots Windows evidence

See [the attribution and correctness report](../../phase-08-projection-hotspots.md). Before commit: `87cf92732707300f0121de7d6c6e061565575863` (instrumentation only, algorithm unchanged from #21). After commit: `dc8415e585e526d77076ca8cf3408d63ab59b021`. `environment.json` records all four binary hashes, source/profile/runner/harness hashes, CPU/OS/toolchain and dependency identity. The matching [dependency snapshot](../2026-11-28-windows/Cargo.lock.snapshot) is shared with earlier measurements. Numerical rows and log contents are copied unchanged; Git may normalize text line endings.

## Cohorts

| Directory family | Fixture | Layout | Warmups / requested samples | Process deadline |
| --- | --- | --- | --- | --- |
| `normal-{before,after}-small` | 10 KB and 100 KB mixed | yes | 1 / 5 | 90s each |
| `profile-{before,after}-small` | 10 KB and 100 KB mixed | yes | 1 / 5 | 90s each |
| `normal-{before,after}-1mb` | 1 MB mixed | yes | 0 / 1 | 180s |
| `profile-{before,after}-1mb` | 1 MB mixed | yes | 0 / 1 | 180s |
| `normal-{before,after}-core` | million-scalar line | no | 1 / 5 | 90s |
| `normal-after-10mb` | 10 MB mixed | yes | 0 / 1 | 90s |
| `normal-after-1mb-warmed` | 1 MB mixed, after-only | yes | 1 / 5 | 90s |
| `profile-censored` | 1 MB, **before** binary | no | 0 / 1 | 3s |

`normal-*` builds have no optional features. `profile-*` builds enable only `mdedit-live/projection-profiling`. All use Obsidian / ConcealInactive, ASCII `x` inserted at source zero, 1.5 GB observed working-set ceiling, 200ms untimed settle, and the unchanged generator/viewport/font settings in `protocol.json`. Baseline and after were serial cohorts after compilation, not randomized interleavings. The one-sample rows do not establish tail latency.

## Files and identities

- `<fixture>.csv`: existing schema2 seven-column outer timings/metadata. Only `kind=sample` is aggregated; metadata kinds preserve setup/warmup/sample identity. No timings are invented for unfinished work.
- `<fixture>.stages.log`: original outer/inner begin/end records. Inner stages have a process-local atomic call ID; the serial outer record supplies fixture, kind, sample and phase. This is a diagnostic transcript, not a concurrent production telemetry API.
- `summary.csv`: completed outer sample calls, count/min/median/max in milliseconds. Setup/warmup/metadata excluded. Prefix counts do not imply a completed frame.
- `projection-stages.csv`: completed inner records with `fixture,kind,sample,phase,call,stage,ns`. `projection-summary.csv` aggregates only sample records. `projection-censored.csv` is present only for the actual interrupted run: two earlier stages completed, reveal_groups began and did not end. That stage has no timing observation.
- `processes.csv`: process status, elapsed time, observed OS peak working set, deadline/settings and last outer stage. All main cohorts completed; the separate before/censored control timed out as intended. A censored peak cannot substitute for a complete-run memory measurement.
- `comparison.csv`: 225 matched groups (75 normal outer, 60 profiled outer, 90 profiled inner), preserving both counts. `before_over_after` is a ratio of cohort medians, with n=1 retained explicitly. The after-only warmed/10 MB cohorts are not included in paired ratios.
- `verification.json`: audit totals — 316 outer / 182 inner summaries, 17 unique-key fixture files, 1,129 matching metadata-row comparisons across normal before/after, profiled before/after and matched normal/profile controls. Metadata comparisons include schema identity and all common setup/source/projection/layout/caret counts.

`extract-stages.ps1 -Directory <cohort>` re-derives inner CSVs from logs, checking association/completion IDs and retaining interrupted prefixes. It rejects a mismatched completion rather than assigning it to another stage. `audit-evidence.ps1` rechecks this archive's raw counts/min/median/max, complete inner stage order, censored prefix, feature-off marker absence, metadata equality and all paired groups. Both write only derived files in this evidence directory/cohort. Run from PowerShell 7:

```powershell
./docs/performance/2026-11-28-projection-hotspots/audit-evidence.ps1
```

## Reproduction

Use fresh checkouts at the before/after commits and PowerShell 7. The runner's `BinaryPath` option selects the separately frozen executable; it keeps its existing default path if omitted. Build variants before measurement, never while another cohort runs:

```powershell
Copy-Item docs/performance/2026-11-28-windows/Cargo.lock.snapshot Cargo.lock
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
Copy-Item target/release/examples/phase8_profile.exe target/normal.exe
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile --features mdedit-live/projection-profiling
Copy-Item target/release/examples/phase8_profile.exe target/profile.exe
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-10kb,mixed-100kb -OutputDirectory target/repeat-normal-small
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-1mb -Samples 1 -Warmups 0 -TimeoutSeconds 180 -OutputDirectory target/repeat-normal-1mb
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-10kb,mixed-100kb -OutputDirectory target/repeat-profile-small
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-1mb -Samples 1 -Warmups 0 -TimeoutSeconds 180 -OutputDirectory target/repeat-profile-1mb
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures single-line -NoLayout -OutputDirectory target/repeat-core
```

At the after commit, add the zero-warmup 10 MB / 90s and default warmed 1 MB controls. At the before commit, the censored control uses `target/profile.exe`, 1 MB, `-NoLayout -Samples 1 -Warmups 0 -TimeoutSeconds 3`. The audit helpers are shipped with the later report commit; use their saved copies when inspecting an earlier checkout. Every runner invocation needs a fresh output directory.

Marker I/O is excluded from inner intervals but included in profiled outer calls. Do not combine featured/unfeatured totals, partial stage prefixes, warmed/single-sample counts, or censored/complete memory counters into a claimed native frame speedup. All measurements are one-machine elapsed calls, not CPU samples, exact retained heap, GPU presentation, input-to-display latency or performance predictions for other systems.
