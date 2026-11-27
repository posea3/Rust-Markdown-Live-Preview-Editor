# Windows indexed-mapping comparison evidence

See [the result and correctness report](../../phase-08-indexed-mapping.md). Measured code: `14994398d4e3eb0cffbe67bdd1261905345a866f`, stacked above profiling parent `6603de6dd8a3b03742ec7bc8e782bf66470ea5fb`. Before values come from the [unchanged archived baseline](../2026-11-28-windows/README.md), engine `0df1961` / harness `c037bfd`.

`environment.json` pins code, source/runner/binary hashes, CPU/OS/toolchain and the dependency-lock hash. The matching [Cargo.lock snapshot](../2026-11-28-windows/Cargo.lock.snapshot) is shared with the baseline. `protocol.json` describes each cohort. Raw numerical values are unedited; Git may normalize line endings.

| Directory | Fixture | Layout | Warmups / samples requested | Process deadline | Paired archive |
| --- | --- | --- | --- | --- | --- |
| `small` | 10 KB, 100 KB mixed | yes | 1 / 5 | 90s each | `baseline`, same protocol |
| `single-sample` | 1 MB mixed | yes | 0 / 1 | 180s | `single-sample`, same protocol |
| `core-only` | million-scalar single line | no | 1 / 5 | 90s | `core-only`, same protocol |
| `large-bounded` | 10 MB mixed | yes | 0 / 1 | 90s | none: archive first iteration was warmup |

All use Obsidian / ConcealInactive, ASCII `x` inserted at source zero, 1.5 GB observed working-set ceiling, and the runner's 200ms untimed settle. Runs were serial after compilation/tests; before data is the earlier same-machine archive, not a randomized interleaving. A 10 MB timeout retains only completed load-call prefixes and is not a projection duration or a complete frame.

- `<fixture>.csv` uses seven columns: `fixture,kind,sample,phase,stage,ns,items`. Timed kinds are `setup`, `warmup`, `sample`. Schema2 metadata uses `meta-setup`, `meta-warmup`, `meta-sample` and a `schema_version=2` setup row, preserving iteration identity. Archived files use legacy `meta`; associate those rows with the preceding timed iteration (initial metadata is setup).
- `summary.csv` includes only completed `kind=sample` calls, grouped by fixture/phase/stage, with count/min/median/max in milliseconds. Warmups/setup/metadata are excluded; a group count alone does not prove a complete frame.
- `processes.csv` includes exit status, deadline/settings, elapsed process time, observed OS peak working set and last begun stage. Peaks include setup/warmup, not retained heap. A timeout is censored work.
- `comparison.csv` joins 75 timing groups in the three paired cohorts, retains each side's count, and reports `old_over_new = old_median_ms / new_median_ms`. Single observations are not tail estimates.
- `verification.json` records the raw-row audit: 82 new summary groups, five schema2 fixture files with unique keys and 281 matching common semantic metadata rows. For legacy keys, track the preceding timed `kind`/`sample` while reading rows in order, starting at setup/0; compare each `meta` value to the corresponding schema2 key. The newer concealed-range/span counts have no counterpart in the original harness and are not counted as paired metadata.

Reproduce at the measured commit from PowerShell 7, using fresh output directories (the runner rejects nonempty ones):

```powershell
Copy-Item docs/performance/2026-11-28-windows/Cargo.lock.snapshot Cargo.lock
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
./scripts/profile-phase8.ps1 -Fixtures mixed-10kb,mixed-100kb -OutputDirectory target/phase-08-indexed-mapping/repeat-small
./scripts/profile-phase8.ps1 -Fixtures mixed-1mb -Samples 1 -Warmups 0 -TimeoutSeconds 180 -OutputDirectory target/phase-08-indexed-mapping/repeat-single
./scripts/profile-phase8.ps1 -Fixtures single-line -NoLayout -OutputDirectory target/phase-08-indexed-mapping/repeat-core
./scripts/profile-phase8.ps1 -Fixtures mixed-10mb -Samples 1 -Warmups 0 -TimeoutSeconds 90 -OutputDirectory target/phase-08-indexed-mapping/repeat-large
```

The metadata emitter change is outside timed operations; fixture generation, engine call sequence and layout settings remain the same. Smaller projection differences are observations without isolated substage attribution. Native styled-line/widget layout, resources, GPU presentation, IME and accessibility remain outside this headless CPU benchmark.
