# Phase 8 parse and projection design

Slice 05 continues above draft PR #23 / `933c16fc3b04f8e734bf835040849a177fd82192`. After scan optimization, the measured 10 MB edit still spends about 1.05s in projection and 0.75s in parsing. Attribute these paths before selecting local changes, then compare normal release calls and complete semantic outputs.

## Constraints and choices

Preserve public parser/projection signatures, const constructors, complete SyntaxSnapshot structure/metadata/order/ranges/status, malformed-event fallback, all Projection fields and error/reveal/widget/map semantics. No dependency/version changes, persistent caches, source edits, native renderer/IME changes, or wall-clock CI thresholds. Keep root Cargo.lock untracked. Use `feature/phase-08-parse-projection` above unmerged #23; publish a stacked draft and verify final-head three-OS CI without merges.

Use existing projection profiling, split its owner-index/grouping/materialization work, and compile out new instrumentation arguments in feature-off code. Add `mdedit-markdown/parse-profiling` diagnostics: source copy, pulldown initialization, buffered event decoding, then syntax construction. Buffering is explicitly a diagnostic control: it changes allocation/lazy interleaving and peak memory, so feature-enabled totals are not the normal parser's latency. The default path stays single-pass; malformed synthetic event tests still exercise its original builder directly. A per-event clock would heavily perturb millions of events, and a system-wide Windows sampler has no established scoped setup here; the chosen diagnostic separates the two major components reproducibly without a new public timing API.

Candidate local changes include reducing indexed syntax entries/query work for known delimiter owner kinds, and avoiding unnecessary adapter metadata copies/child-vector capacity waste. Choose exact algorithms only after actual attribution. Do not replace the Markdown parser, merge syntax nodes, change metadata ownership contracts or infer that a vendor's entire parse cost can be removed by an adapter change.

## Evidence and correctness

Use frozen before/after binaries, existing source fixtures/layout/policy/edit, one warmup/five samples for 10/100 KB and 1 MB, zero/one bounded 10 MB, and a core-only single-line control. Diagnostic features may run separately if combined memory pressure hits the 1.5GB/90s bounds; retain censored prefixes. Default normal totals remain comparable. Add streaming diagnostic hashes of complete ordered syntax/projection values outside timed calls; full object equality/oracles, not hashes, establish correctness. Pin source/toolchain/lock/four binary identities and retain raw logs/rows/settings.

Compare syntax to the frozen parent parser across dialects, Unicode/CRLF, metadata-rich and malformed event corpora. Compare full projection against the existing independent reference, including nested/equal/clipped bounds, ownership ties, source reveal and current extension configuration. Use deterministic node/allocated-slot/visit counts for structural work, preserving const construction and wide/multi-child growth. Feature profiling and normal parsing must return equal snapshots on the exercised real-input corpus.

One inline implementer and one fresh final branch reviewer. Important findings get one RED/GREEN/full-suite fix pass; minor findings and decisions stay explicit. Native latency, giant-line shaping, exact heap attribution and Phase 8 completion remain separate claims.
