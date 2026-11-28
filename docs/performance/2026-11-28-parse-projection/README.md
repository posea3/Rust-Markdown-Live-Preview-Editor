# Parse and projection Windows evidence

See [the report](../../phase-08-parse-projection.md). Instrumentation-only before is `c81f6b34e607a516964e5d0ed8d83fe987edd72c`; optimized source is `cdd83e5584c4ced4d400a33c854936599e5aef0e`, above draft PR #23 / `933c16f`. `environment.json` pins both normal/profile binaries, source/harness/runner/extractor hashes, toolchain/CPU/OS and execution order. The unchanged [Cargo.lock snapshot](../2026-11-28-windows/Cargo.lock.snapshot) supplies dependencies. Raw logs/values are copied unchanged; Git may normalize text line endings.

| Directory family | Fixtures | Layout | Warmups / samples | Deadline |
| --- | --- | --- | --- | --- |
| `normal-{before,after}-warmed` | mixed 10 KB, 100 KB, 1 MB | yes | 1 / 5 | 90s each |
| `profile-{before,after}-warmed` | mixed 10 KB, 100 KB, 1 MB | yes | 1 / 5 | 90s each |
| `normal-{before,after}-10mb` | mixed 10 MB | yes | 0 / 1 | 90s |
| `profile-{before,after}-10mb` | mixed 10 MB | yes | 0 / 1 | 90s |
| `normal-{before,after}-core` | million-scalar line | no | 1 / 5 | 90s |
| `profile-censored` | mixed 10 MB, **before profile** binary | no | 0 / 1 | 1s |

Normal builds have no profiling features. Profile builds enable **both** `mdedit-markdown/parse-profiling,mdedit-live/projection-profiling` and leave extension profiling off. The parse diagnostic buffers events, whereas normal parsing streams them; its outer/memory costs are a distinct control, not normal latency. All runs use Obsidian/ConcealInactive, ASCII `x` at source zero, 1.5GB working-set ceiling and 200ms untimed settle. All compilation/tests stopped before serialized cohorts. No randomized order or affinity/power isolation was applied. Five-sample medians and one-sample observations do not establish tail latency.

## Data and audit

- `<fixture>.csv` retains schema2 outer timing and untimed metadata, including full syntax/projection Debug digests (FNV-1a u32 halves), scan digest, counts and fallback status. Digests are diagnostic/source-specific, not persistent serialization or a substitute for full snapshot tests.
- `<fixture>.stages.log` retains outer `begin` markers and inner `parse-begin/end` and `projection-begin/end` call/stage/ns markers. `parse-stats,call,events,nodes,child_slots,single_child_nodes` is emitted after tree construction. Slots are allocated element capacity, not exact heap allocation bytes.
- `summary.csv` aggregates only completed `kind=sample` outer calls. `parse-stages.csv` / `projection-stages.csv` and their summaries retain complete inner observations, including warmup context; summaries exclude warmups. Censored `build_tree` has no duration row. No projection invocation began in the interrupted control, so it has no projection extraction files.
- `processes.csv` retains settings, completion/timeout, child elapsed time, observed OS peak working set and last outer stage. Whole-process totals include setup/digests/teardown; peaks do not isolate retained syntax/index/buffer memory.
- `comparison.csv` pairs 343 same-count timing groups. `verification.json` records 352 outer/339 inner summary groups, 19 fixture files, 2,745 matching metadata rows and 152 fully matched outer invocations including warmups. Completed projections all have zero raw fallback.

From the repository root, PowerShell 7:

```powershell
./docs/performance/2026-11-28-parse-projection/audit-evidence.ps1
```

The audit recomputes min/median/max from raw durations, checks unique identities, ordered inner/outer links, normal marker absence and matching normal/profile/before/after semantic metadata. An actual one-second interruption retains source/init/decode observations plus one censored tree call, with no fabricated duration. The reusable extractor additionally validates malformed, reordered, changed-ID, missing-warmup and torn-tail traces; all 16 regression cases pass. A completed prefix is not a full call/frame.

## Reproduction

Use separate checkouts at each measured commit and fresh output directories, then build and freeze both variants before measuring. Fixed headless layout is 1000x700, SansSerif 22/31px, Word wrap/Advanced shaping. Fixtures and process guard are unchanged from the preceding slices.

```powershell
Copy-Item docs/performance/2026-11-28-windows/Cargo.lock.snapshot Cargo.lock
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile
Copy-Item target/release/examples/phase8_profile.exe target/normal.exe
cargo build --locked --release -p mdedit-ime-lab --example phase8_profile --features mdedit-markdown/parse-profiling,mdedit-live/projection-profiling
Copy-Item target/release/examples/phase8_profile.exe target/profile.exe
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-10kb,mixed-100kb,mixed-1mb -OutputDirectory target/repeat-normal-warmed
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures mixed-10mb -Samples 1 -Warmups 0 -OutputDirectory target/repeat-normal-10mb
./scripts/profile-phase8.ps1 -BinaryPath target/normal.exe -Fixtures single-line -NoLayout -OutputDirectory target/repeat-normal-core
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-10kb,mixed-100kb,mixed-1mb -OutputDirectory target/repeat-profile-warmed
./scripts/profile-phase8.ps1 -BinaryPath target/profile.exe -Fixtures mixed-10mb -Samples 1 -Warmups 0 -OutputDirectory target/repeat-profile-10mb
$projection17 = @('validation','source_text','owner_index','reveal_grouping','reveal_materialize','conceal_spans','structural_padding','builtin_candidates','extension_candidates','normalize_ownership','shadow_filter','overlap_check','map','materialize','project_blocks','project_styles','project_widgets')
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-warmed -Subsystem parse
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-warmed -Subsystem projection -ExpectedStages $projection17
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-10mb -Subsystem parse
./scripts/extract-phase8-stages.ps1 -Directory target/repeat-profile-10mb -Subsystem projection -ExpectedStages $projection17
```

The interrupted control uses the before profile binary with `-Fixtures mixed-10mb -NoLayout -Samples 1 -Warmups 0 -TimeoutSeconds 1`, followed by parse extraction only. Its precise stopping stage can vary. Feature-on stage timers exclude marker output; outer calls include instrumentation/statistics overhead, and buffering changes allocation/interleaving. Do not subtract variants to infer overhead or normal substage latency. Default debug extension libraries still retain their preceding minor diagnostic literals/enumeration; normal release execution emits no internal markers. Replacing censored raw input within an existing directory can leave an old derived censor CSV; use fresh directories. The immutable current archive has no stale output.
