# Windows Phase 8 raw evidence

See [the report](../../phase-08-performance.md). Engine baseline is `0df1961` (all Phase 7 PRs merged, main CI `46023467296` PASS). Data values are unedited numeric rows; Git may normalize text line endings.

| Directory | Profiler source commit | Layout | Reveal policy | Warmups / requested samples | Nominal process deadline |
| --- | --- | --- | --- | --- | --- |
| `baseline` | `c037bfd` | enabled | ConcealInactive | 1 / 5 | 90 s |
| `single-sample` | `c037bfd` | enabled | ConcealInactive | 0 / 1 | 180 s |
| `core-only` | `c037bfd` | disabled | ConcealInactive | 1 / 5 | 90 s |
| `source-visible` | `21ffaf4` | enabled | SourceVisible | 1 / 5 | 90 s |
| `source-visible-large` | `21ffaf4` | disabled | SourceVisible | 0 / 1 | 90 s |

`protocol.json` records exact commit IDs and settings. `environment.json` identifies the original baseline/large/core-only harness; `control-environment.json` identifies the policy-control harness. Their sampling fields are defaults; the per-directory settings above and protocol JSON govern supplemental runs. File hashes are original-machine byte-level provenance; checkout line-ending conversion can alter text hashes without changing code/data semantics. The root Cargo.lock was held fixed during all runs; its resolution is provided as `Cargo.lock.snapshot` and its hash is in the protocol.

Each fixture CSV has `fixture,kind,sample,phase,stage,ns,items`:

- `kind=sample` contains timed calls from measured iterations; `warmup` is excluded from summaries; `setup` includes per-process FontSystem initialization.
- `kind=meta` records sizes/counts/fallback state in `items`, with `ns=0`; these are not timing samples.
- `phase=load` starts a fresh document/cache/buffer; `phase=edit` reuses that cache/buffer after inserting one ASCII byte at source offset zero.
- `summary.csv` contains min/median/max of completed **stage calls**, with their individual counts. A timed-out iteration can contribute a prefix; do not infer a complete frame or sum stages with different counts.
- `processes.csv` records completion/timeout, observed peak working set and the last begun stage. Older-harness files omit policy/deadline columns; `protocol.json` supplies them. A timeout is not the last stage's duration.

Default raw data intentionally retains censored large cases. The 1 MB supplemental run completes one pair without warmup; the single-line baseline completes one measured pair before its next iteration times out. SourceVisible removes concealment/widgets and therefore changes semantics. Core-only omits the entire Buffer/FontSystem path. Neither is an equivalent native-editor optimization.

Reproduce from a fresh checkout inside PowerShell 7 (`pwsh`), with the recorded dependency snapshot. The current runner rejects nonempty output directories and cleans up owned children on normal interruption/error; those guard corrections do not change the Rust measurement code or archived values:

```powershell
Copy-Item docs/performance/2026-11-28-windows/Cargo.lock.snapshot Cargo.lock
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
./scripts/profile-phase8.ps1 -OutputDirectory target/phase-08-profiling/repeat-baseline
./scripts/profile-phase8.ps1 -Fixtures mixed-1mb -Samples 1 -Warmups 0 -TimeoutSeconds 180 -OutputDirectory target/phase-08-profiling/repeat-single
./scripts/profile-phase8.ps1 -Fixtures single-line -NoLayout -OutputDirectory target/phase-08-profiling/repeat-core
./scripts/profile-phase8.ps1 -Fixtures mixed-10kb,mixed-100kb -SourceVisible -OutputDirectory target/phase-08-profiling/repeat-visible
./scripts/profile-phase8.ps1 -Fixtures mixed-1mb -SourceVisible -NoLayout -Samples 1 -Warmups 0 -OutputDirectory target/phase-08-profiling/repeat-visible-large
```

The baseline/current control harness differs only in benchmark policy selection and untimed map-count reporting. The same engine functions and dependency resolution are used. All measurements are foreground-machine elapsed API-call times, not GPU presentation/input latency or exact predictions for other CPUs/fonts/operating systems.
