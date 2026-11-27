# Phase 7 Extension API Hardening

Branch: `feature/extension-api-phase-07`

Status: **Slice 01 implemented and locally verified — namespace isolation, public consumer examples, and compatibility contracts; current-head 3-OS CI is tracked by the PR checks**.

## Entry decision

All six Phase 6 widget slices are implemented and PR #12 is integrated at `064bc40130f9a1c61adaab1168180d791e5b1416`. Its final-head Windows/macOS/Ubuntu CI passed. The user explicitly deferred Windows native GUI/IME acceptance on 2026-11-26 and authorized Phase 7 to proceed on macOS. On 2026-11-27 the Windows native checklist passed locally with CRLF and widget-layout host fixes, recorded in `docs/phase-06-widgets.md`. Those fixes still require integration and fresh cross-platform CI; the unchanged upstream head does not inherit the local result. Narrator/VoiceOver acceptance remains deferred.

## Slice 01 design

Applications implement the existing `SyntaxExtension` trait and register scanners in an `ExtensionSet`. Candidates retain canonical source ranges and string attributes. The merge engine validates bounds and resolves protected syntax and overlap claims before producing an `ExtensionSnapshot`.

An extension kind is identified by the pair `(ExtensionId, kind name)`. Live Preview's built-in Callout/Custom Embed adapter recognizes only the `obsidian` namespace. An application may use the local names `callout` or `embed` without having its source interpreted as an Obsidian widget. Unknown extension kinds remain available as scanner metadata and do not gain a native renderer automatically.

The implementation keeps the existing scanner API and adds a qualified dispatch check, public consumer regression tests, two runnable sample scanners, and contract documentation. A generic widget registry is a later API design decision; this slice establishes the behavior of the current scanner/projection boundary.

The first compatibility tests cover application/built-in coexistence, registration order, protected source, invalid UTF-8 ranges, containment and priority conflicts. No native rendering or IME integration is changed by this slice.

## Public entry points

| API | Consumer responsibility |
| --- | --- |
| `SyntaxExtension` | Supply a stable namespace and synchronous scanner; implementations are `Send + Sync` |
| `ExtensionScanContext` | Read canonical source and project-owned syntax; construct byte ranges with `source_range` |
| `ExtensionCandidate` | Claim a non-empty range, local kind name, optional string attributes, priority, and overlap policy |
| `ExtensionSet` | Explicitly register the scanners desired by the host and scan a matching document/syntax snapshot |
| `ExtensionSnapshot` | Consume accepted source ranges and qualified kinds; inspect rejected-candidate diagnostics |
| `Projection::build_with_extensions` | Combine validated scanner metadata with the core Markdown projection |

CommonMark/GFM applications may register no scanners. Obsidian syntax is opt-in through `ObsidianSyntaxExtension` or `obsidian_extension_set`; selecting a parser dialect does not register scanners by itself.

## Runnable samples

```bash
cargo run -p mdedit-markdown --example extension_scanners
```

The complete consumer is in `crates/mdedit-markdown/examples/extension_scanners.rs`; the two reusable scanner implementations are in `examples/support/scanners.rs` within that crate.

- `example.references::reference` recognizes `{{ref:설계 노트}}` and emits a `target` attribute. A target is trimmed, non-empty, and confined to one line.
- `example.issues::issue` recognizes `#42` at the start of source or after whitespace and emits a `number` attribute. The number contains ASCII digits.

These examples demonstrate the scanner boundary; their small grammars define no nested-delimiter or Markdown escape processing. Applications own such syntax choices. The example prints matched source slices and a protected-code diagnostic without modifying source or requesting a native widget.

## Identity and rendering

Use a distinct, stable `ExtensionId` for each registered scanner and treat its local kind names as namespaced values. Compare both `matched.kind().extension()` and `matched.kind().name()` when assigning meaning. The built-in `obsidian` namespace is reserved for the built-in scanner.

The registration API currently accepts duplicate IDs. Hosts must avoid them; duplicates do not create isolated namespaces, and equal-priority claims with the same qualified identity can depend on the scanner's emission order. Unique string attribute keys should also be chosen by the scanner; `attribute(key)` returns the first matching attribute.

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

Inspect `ExtensionSnapshot::issues()` for `InvalidRange`, `SuppressedByCoreSyntax`, and `Conflict { kept }`. A rejected individual claim is a diagnostic, whereas unavailable syntax, stale revision, source-length mismatch, or unsupported document size returns `ExtensionError` for the scan.

## Snapshot and configuration lifecycle

Parse and scan the same `DocumentSnapshot`. Revision and source-length checks reject stale or mismatched inputs, but do not identify unrelated documents with the same revision and byte length; the host must retain document identity.

Scanner registration/configuration changes require a rescan even when the document revision is unchanged. Hosts using `ParseReconciler` should advance `ParseConfigEpoch` when parser/dialect/extension configuration changes, discard old accepted extension data, and rebuild projection from the newly accepted syntax and extension snapshots. The extension snapshot itself has no configuration-epoch field.

Scans are synchronous. Scanner panics are not caught by `ExtensionSet`; hosts select the implementations they register. This slice adds no asynchronous plugin runtime or dynamic library loading.

## Compatibility guidance

The workspace remains at `0.1.0`; this slice preserves existing public scanner method signatures and adds no dependency. It fixes the accidental interpretation of foreign `callout`/`embed` kinds as built-in widgets. Applications that relied on that implicit interpretation must explicitly choose the built-in Obsidian scanner or provide a host adapter.

Keep host-owned syntax in host-owned scanner crates. Depend on exported `mdedit-*` types, preserve canonical byte ranges, and exercise examples/consumer tests when upgrading. Parser-specific internals and native rendering types are not part of this scanner contract. Future breaking changes require explicit migration guidance before an API-stability milestone is declared.

## Verification and follow-up

```bash
cargo test -p mdedit-live --test extension_compatibility
cargo test -p mdedit-markdown --test extension_contract
cargo run -p mdedit-markdown --example extension_scanners
```

The namespace regression was observed before the fix: a foreign `embed` emitted an extra Custom Embed widget, and a foreign `callout` concealed the containing blockquote. The three regression cases passed after qualifying built-in dispatch by `OBSIDIAN_EXTENSION_ID`. Nine public-consumer contract tests exercise the merge/snapshot boundary.

Local macOS verification on 2026-11-26: fmt/check/clippy PASS; workspace tests **242 passed, 0 failed, 1 ignored**; the public scanner example ran successfully. The ignored native GPU regression was not rerun, and native GUI/IME acceptance was not repeated for this scanner-only slice.

Further Phase 7 decisions include explicit duplicate-registration policy, configuration identity on cached extension data, scanner failure handling, and any public host-renderer registration boundary. These require their own compatibility review; this slice does not declare a frozen 1.0 API. The Windows native checklist passed with local host fixes that still require integration and new CI. Narrator/VoiceOver checks remain separately deferred; scanner tests do not establish native accessibility acceptance.
