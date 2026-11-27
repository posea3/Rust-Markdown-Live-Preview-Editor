# Phase 7 Extension API Hardening

Slices 01–02 are integrated through PRs #13 and #15, with Windows acceptance PR #14 merged between them. Slice 03 is proposed in PR #16 (`bacd562`), based on integrated main `7d46db3`. Current branch: `feature/extension-api-phase-07-failures` (Slice 04), stacked above Slice 03.

Status: **Slices 01–02 merged; Slices 03–04 implement configuration provenance, checked cached-snapshot projection, and declared scanner failures with strict scan abort. Current-head three-OS CI is tracked by their PR checks**.

## Entry decision

All six Phase 6 widget slices are implemented. The user initially deferred Windows native verification on 2026-11-26; the 2026-11-27 Windows session subsequently passed with host fixes recorded in `docs/phase-06-widgets.md` and [PR #14](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/14). The user then requested merging #13 → #14 → #15 in order. Their merge commits are `b3d9dfa`, `7b927f8`, and `7d46db3`; final integrated main passed fresh [Windows/macOS/Ubuntu CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/45979821603). The native GUI result remains tied to the measured acceptance build; later scanner-only changes do not imply another native run. Narrator/VoiceOver acceptance remains deferred.

## Slice 01 design

Applications implement the existing `SyntaxExtension` trait and register scanners in an `ExtensionSet`. Candidates retain canonical source ranges and string attributes. The merge engine validates bounds and resolves protected syntax and overlap claims before producing an `ExtensionSnapshot`.

An extension kind is identified by the pair `(ExtensionId, kind name)`. Live Preview's built-in Callout/Custom Embed adapter recognizes only the `obsidian` namespace. An application may use the local names `callout` or `embed` without having its source interpreted as an Obsidian widget. Unknown extension kinds remain available as scanner metadata and do not gain a native renderer automatically.

The implementation keeps the existing scanner API and adds a qualified dispatch check, public consumer regression tests, two runnable sample scanners, and contract documentation. A generic widget registry is a later API design decision; this slice establishes the behavior of the current scanner/projection boundary.

The first compatibility tests cover application/built-in coexistence, registration order, protected source, invalid UTF-8 ranges, containment and priority conflicts. No native rendering or IME integration is changed by this slice.

## Public entry points

| API | Consumer responsibility |
| --- | --- |
| `SyntaxExtension` | Supply a stable namespace and synchronous scanner; implementations are `Send + Sync` |
| `FallibleSyntaxExtension` | Supply a complete candidate list or a declared `ExtensionScanError`; callback panics still propagate |
| `ExtensionScanContext` | Read canonical source and project-owned syntax; construct byte ranges with `source_range` |
| `ExtensionCandidate` | Claim a non-empty range, local kind name, optional string attributes, priority, and overlap policy |
| `ExtensionSet` | Explicitly register the scanners desired by the host and scan a matching document/syntax snapshot |
| `ExtensionSet::try_register` | Recover from a duplicate scanner ID through `ExtensionRegistrationError::DuplicateId` without changing existing registrations |
| `ExtensionSet::try_register_fallible` | Register a fallible scanner under the same unique-namespace and configuration rules |
| `ExtensionSnapshot` | Consume accepted source ranges and qualified kinds; inspect rejected-candidate diagnostics |
| `ExtensionConfigId` | Use process-local configuration provenance when retaining scanner results in memory |
| `Projection::build_with_extensions` | Combine validated scanner metadata with the core Markdown projection |
| `Projection::build_with_current_extensions` | Reject cached metadata from an obsolete or unrelated extension configuration before building projection |

CommonMark/GFM applications may register no scanners. Obsidian syntax is opt-in through `ObsidianSyntaxExtension` or `obsidian_extension_set`; selecting a parser dialect does not register scanners by itself.

## Runnable samples

```bash
cargo run -p mdedit-markdown --example extension_scanners
```

The complete consumer is in `crates/mdedit-markdown/examples/extension_scanners.rs`; the two reusable scanner implementations are in `examples/support/scanners.rs` within that crate.

`cargo run -p mdedit-markdown --example fallible_scanners` demonstrates both complete success and a reported scanner failure. Its host policy prints the canonical source on failure, rather than treating earlier matches as a complete result.

- `example.references::reference` recognizes `{{ref:설계 노트}}` and emits a `target` attribute. A target is trimmed, non-empty, and confined to one line.
- `example.issues::issue` recognizes `#42` at the start of source or after whitespace and emits a `number` attribute. The number contains ASCII digits.

These examples demonstrate the scanner boundary; their small grammars define no nested-delimiter or Markdown escape processing. Applications own such syntax choices. The example prints matched source slices and a protected-code diagnostic without modifying source or requesting a native widget.

## Identity and rendering

Use a distinct, stable `ExtensionId` for each registered scanner and treat its local kind names as namespaced values. Compare both `matched.kind().extension()` and `matched.kind().name()` when assigning meaning. The built-in `obsidian` namespace is reserved for the built-in scanner.

Slice 02 rejects duplicate IDs using exact `ExtensionId` equality. Use `try_register` when loading host configuration: it returns `ExtensionRegistrationError::DuplicateId(id)` and leaves the existing set unchanged. The rejected scanner is not registered or scanned. A different namespace, including a name that shares a prefix, remains valid.

`register` and `with_extension` retain their existing signatures and panic on duplicate IDs, with a diagnostic naming the conflict. They are convenience APIs for known-valid configuration. Code that previously registered duplicates must remove the duplicate or migrate to `try_register`; registration never silently replaces a scanner. The public scanner example uses the fallible method with `?`.

The set captures `SyntaxExtension::id()` once when registering each scanner. That ID owns the namespace for the registration's lifetime. Changing an implementation's returned ID later does not rename existing matches, release its original namespace, or claim another registered namespace. Scanning and its diagnostics use the captured ID.

Unique string attribute keys should also be chosen by the scanner; `attribute(key)` returns the first matching attribute.

Scanner metadata is independent of rendering. Built-in Callout/Custom Embed widgets require the Obsidian namespace plus their supported kind. Foreign or unknown kinds do not conceal additional source or produce native widgets. Core Markdown projection still applies normally to that source.

## Canonical source and protected regions

All offsets are UTF-8 **byte** offsets into the supplied document snapshot, not character, UTF-16, projected, or display offsets. `source_range(start, end)` checks bounds and character boundaries. The merge engine rejects empty, out-of-bounds, and non-boundary candidates through `ExtensionIssueReason::InvalidRange`; unrelated valid candidates can still be accepted.

Every scanner receives the full canonical source. With the default `MarkdownText` scan mode, the merge engine suppresses candidates that overlap protected code, HTML, metadata, or raw-source regions after the scanner returns. Source is not pre-filtered into separate text fragments. `AllSource` explicitly opts out of that filtering, for applications that need metadata about protected text.

Neither accepted nor rejected candidates edit the document. Resource loading, navigation, widget painting, and editor transactions belong to host adapters.

## Overlap and diagnostics

Accepted claims are chosen deterministically by descending priority, earlier start, longer range, then qualified kind identity. Registration order is not a tie-breaker for distinct IDs. Results are returned in source order; diagnostic ordering is not a separate stability guarantee.

- `Exclusive` claims reject overlapping claims that lose the ordering above.
- Containment is permitted only when **both** claims use `AllowContained`.
- Identical ranges always conflict, including two `AllowContained` claims.
- Crossing/partial overlaps always conflict.

Inspect `ExtensionSnapshot::issues()` for `InvalidRange`, `SuppressedByCoreSyntax`, and `Conflict { kept }`. A rejected individual claim is a diagnostic, whereas unavailable syntax, stale revision, source-length mismatch, unsupported document size, or a declared scanner failure returns `ExtensionError` for the scan.

## Snapshot and configuration lifecycle

Parse and scan the same `DocumentSnapshot`. Revision and source-length checks reject stale or mismatched inputs, but do not identify unrelated documents with the same revision and byte length; the host must retain document identity.

Slice 03 gives each observed `ExtensionSet` configuration an opaque `ExtensionConfigId`, available through `configuration_id()` on the set and its scan snapshots. Repeated scans and snapshot clones retain that ID until configuration changes. Different sets receive different IDs even when both are empty or register scanners with identical names. Snapshot equality includes this provenance; equal match lists from unrelated configurations are no longer interchangeable cache entries. IDs are process-local and must not be persisted as configuration fingerprints.

Successful registration invalidates the current identity. Duplicate rejection leaves the identity and existing snapshots current. After changing host-owned scanner settings, call `invalidate_configuration()` before reusing cached data or rescanning. The set cannot inspect scanner interior state. Synchronize setting changes with active scans; the invalidation method takes `&mut self`. A new identity is allocated lazily on the next observation or scan. IDs never wrap or get reused; exhausting the process-wide `u64` identity space produces an explicit panic.

For an in-memory cache, reuse requires both current document metadata and a matching `snapshot.configuration_id() == extensions.configuration_id()`. Use `Projection::build_with_current_extensions` for cached metadata; it accepts the snapshot followed by the current `ExtensionSet`, rejects mismatches through `ProjectionBuildError::ExtensionConfigurationMismatch { expected, snapshot }`, and retains document revision/length validation. Rescan after a mismatch. Existing `build_with_extensions` keeps its signature and checks document metadata, but has no current set with which to validate configuration; use it for freshly scanned data or enforce the identity check in the host. The public scanner example demonstrates registration invalidating a cached scan without editing source.

Hosts using `ParseReconciler` should still advance `ParseConfigEpoch` when parser/dialect configuration changes. Discard both incompatible syntax and extension snapshots derived from that old parser configuration, invalidate the extension configuration, and rescan against the new syntax. The configuration ID cannot detect parser changes automatically. Extension identity does not replace parser configuration epochs or host-owned document identity. Unrelated documents with equal revision and byte length remain the host's responsibility.

## Declared scanner failures and host fallback

Slice 04 preserves the required `SyntaxExtension::scan -> Vec<ExtensionCandidate>` method. New scanners that need to report recoverable failures implement `FallibleSyntaxExtension::try_scan -> Result<Vec<ExtensionCandidate>, ExtensionScanError>`. The separate trait avoids changing existing scanner implementations or silently substituting an empty list when a failure occurs.

Register through `try_register_fallible`, `register_fallible`, or `with_fallible_extension`. The latter two panic on duplicate configuration, matching the existing convenience APIs. Both interfaces share the same namespace registry, registration-time IDs, protected-source policies, and configuration invalidation. A fallible scanner defaults to `MarkdownText`; `AllSource` remains explicit opt-in.

The first declared failure stops the scan immediately and returns `ExtensionError::ScannerFailed { extension, error }`. The ID is captured at registration, and the owned `ExtensionScanError` is available through the error's `source()` chain. No partial snapshot is published and later scanners are not invoked. Earlier callback side effects are not rolled back. Invalid candidates from an otherwise successful callback remain per-candidate issues; they do not become a declared scanner failure.

A reported error does not remove a scanner, poison the set, or change configuration identity. It is not a successful empty snapshot. The host must handle that attempt as failed and choose source-visible fallback, a deliberate retry, or revised scanner configuration. An older successful snapshot does not become a new success merely because its configuration ID still matches. Invalidate configuration when changing effective settings or external state used in cached scanner results, and rescan. The example demonstrates complete success with a reference scanner followed by a fallible issue scanner, and source-visible fallback when the latter fails after the former has run.

Scans are synchronous. Rust panics in `id`, `scan_mode`, `scan`, or `try_scan` are not caught or converted into declared errors. There is no recovery from `panic=abort`, timeout enforcement, process isolation, asynchronous plugin runtime, or dynamic library loading. Hosts choose the scanner implementations they register.

## Compatibility guidance

The workspace remains at `0.1.0`; this slice preserves existing public scanner method signatures and adds no dependency. It fixes the accidental interpretation of foreign `callout`/`embed` kinds as built-in widgets. Applications that relied on that implicit interpretation must explicitly choose the built-in Obsidian scanner or provide a host adapter.

Slice 02 adds a registration error type and `try_register` without changing existing valid call sites. It deliberately tightens the distinct-ID precondition: duplicate convenience registrations now panic, and scanner namespaces are captured at registration rather than reread on each scan. Hosts with dynamic configuration should handle registration errors before replacing their active extension set. Registration failure is separate from the scan-time `ExtensionError` contract.

Slice 03 preserves the const `ExtensionSet::new()` constructor and existing method signatures. Consumers that cache extension snapshots should adopt the configuration ID and checked projection builder. `ProjectionBuildError` has an additional mismatch variant; exhaustive matches must handle it. Snapshot equality now includes configuration provenance. This adds no native renderer or scanner panic isolation.

Slice 04 adds the fallible trait, error type, and registration methods without changing existing trait requirements. `ExtensionError` gains `ScannerFailed`; exhaustive matches must handle it. Successful legacy scans keep their previous behavior. Error reporting and fallback are explicit host decisions; this is not a panic-catching layer or rollback mechanism.

Keep host-owned syntax in host-owned scanner crates. Depend on exported `mdedit-*` types, preserve canonical byte ranges, and exercise examples/consumer tests when upgrading. Parser-specific internals and native rendering types are not part of this scanner contract. Future breaking changes require explicit migration guidance before an API-stability milestone is declared.

## Verification and follow-up

```bash
cargo test -p mdedit-live --test extension_compatibility
cargo test -p mdedit-markdown --test extension_contract
cargo test -p mdedit-markdown --test extension_registration
cargo test -p mdedit-markdown --test extension_configuration
cargo test -p mdedit-live --test extension_configuration
cargo test -p mdedit-markdown --test extension_failures
cargo test -p mdedit-live --test extension_failures
cargo run -p mdedit-markdown --example extension_scanners
cargo run -p mdedit-markdown --example fallible_scanners
```

The namespace regression was observed before the fix: a foreign `embed` emitted an extra Custom Embed widget, and a foreign `callout` concealed the containing blockquote. The three regression cases passed after qualifying built-in dispatch by `OBSIDIAN_EXTENSION_ID`. Nine public-consumer contract tests exercise the merge/snapshot boundary.

Local macOS verification on 2026-11-26: fmt/check/clippy PASS; workspace tests **242 passed, 0 failed, 1 ignored**; the public scanner example ran successfully. The ignored native GPU regression was not rerun, and native GUI/IME acceptance was not repeated for this scanner-only slice.

Slice 02 first reproduced three failures: duplicate `register`/builder calls did not reject configuration, and changing a scanner's reported ID renamed accepted matches. The fixes passed those regressions plus two public-consumer tests covering recoverable duplicate rejection, continued unique registration, unchanged accepted matches, and reservation of a mutable scanner's original namespace. The fallible API's absence was also recorded before implementation. Full verification and final-head CI are tracked by the Slice 02 plan and PR.

Local Windows verification on 2026-11-27 for Slice 02: fmt/check/clippy `-D warnings` PASS; workspace tests **252 passed, 0 failed, 1 ignored**; rustdoc with warnings denied PASS; the public example passed using `try_register` and retained its Korean/reference/issue/protected-code output. The five registration tests and nine existing scanner-contract tests all passed. The native GPU module and native input/rendering code are unchanged by this slice, so their earlier Windows acceptance results remain scoped to PR #14 rather than treated as new GUI runs.

Slice 03 local Windows verification on 2026-11-27: **263 passed, 0 failed, 1 ignored**; fmt/check/clippy with warnings denied, rustdoc with warnings denied, and the public cache-aware scanner example passed. Ten public-consumer tests cover configuration identity and checked projection, and one isolated-counter unit test covers exhaustion without ID reuse. Removing the projection guard reproduced three stale/cross-set failures; replacing checked allocation with wrapping allocation reproduced the exhaustion failure. The correct implementations passed after restoration. Fresh CI for integrated main `7d46db3` passed on all three operating systems; the Slice 03 PR tracks its own current-head CI.

Slice 04 local Windows verification on 2026-11-27: **276 passed, 0 failed, 1 ignored**; fmt/check/clippy with warnings denied, rustdoc with warnings denied, and both scanner examples passed. Twelve markdown consumer regressions cover successful mixed scanning, captured failure identity/cause, short-circuiting, prevalidation, protected/AllSource handling, malformed-candidate diagnostics, shared duplicate rejection, convenience APIs, panic propagation, and retry after invalidation. One Live Preview consumer regression verifies native widget mapping from successful fallible metadata. Suppressing reported errors reproduced three failures; correct propagation was restored before full verification. The fallible example reported two matches on success and the failing namespace plus unchanged canonical source on failure.

The remaining Phase 7 design decision is any public host-renderer registration boundary, followed by overall compatibility review. This work does not declare a frozen 1.0 API. Native GUI/IME was not repeated for the scanner-only slices. Narrator/VoiceOver checks remain separately deferred; scanner tests do not establish native accessibility acceptance.
