# Phase 7 compatibility and integration review

Slice 06 reviews the public extension boundary and its composition through scanner metadata, checked projection, host dispatch, and canonical document transactions. Existing runtime signatures and behavior are unchanged in this slice. It adds consumer coverage and migration guidance, including both deferred Slice 05 review items.

## Delivery and integration status

| Slice | Delivered contract | Integration status |
| --- | --- | --- |
| 01 | Qualified scanner kinds and public consumer examples | Merged through [PR #13](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/13) |
| 02 | Unique registration and captured scanner namespaces | Merged through [PR #15](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/15) |
| 03 | Configuration identity and checked cached projection | Merged through [PR #16](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/16), `f24453a` |
| 04 | Declared scanner failures and strict scan abort | Merged through [PR #17](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/17), `ee58f72` |
| 05 | Host renderer dispatch and widget support preflight | Merged through [PR #18](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/18), `5f4563a` |
| 06 | Cumulative compatibility audit and composed consumer verification | Merged through [PR #19](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/19), `0df1961` |

The cumulative review covered `7d46db3..c57407c`; the important delayed-action issue was corrected in `66fbb0b` with regression evidence and a green suite. PRs #16 -> #17 -> #18 -> #19 merged in that order on 2026-11-28. Fresh integrated-main [Windows/macOS/Ubuntu CI](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/actions/runs/46023467296) passed at `0df1961`, whose tree matches verified `66fbb0b`. Slices 01–02 were already in the review baseline. The old CommonMark fixture-wording minor is clarified below and in its test name.

## Public API and migration audit

| Surface | Compatibility effect | Host upgrade action |
| --- | --- | --- |
| `SyntaxExtension` | Required `id` and `scan -> Vec<ExtensionCandidate>` signatures and `Send + Sync` contract retained | Existing valid scanner implementations continue to compile |
| `ExtensionSet` construction/registration | Const `new`, `Default`, `register`, and builder signatures retained; duplicate IDs deliberately reject/panic, and IDs are captured once | Choose stable distinct IDs; use `try_register` / `try_register_fallible` for dynamic configuration; do not rely on replacement or later ID mutation |
| `FallibleSyntaxExtension` | Optional separate trait; first declared error aborts the scan, publishing no partial snapshot | Handle `ExtensionError::ScannerFailed`; choose explicit retry/fallback, and do not treat old successful metadata as a new successful scan |
| `ExtensionError` | Adds `ScannerFailed { extension, error }` | Update exhaustive matches; the owned `ExtensionScanError` is available through `source()`; panic/side-effect rollback is not provided |
| `ExtensionSnapshot` | Equality includes process-local configuration provenance | Reuse only for the correct document, current configuration and parser epoch; IDs are not persistent fingerprints |
| Existing `Projection::build_with_extensions` | Signature retained; validates document revision/length but cannot validate the current extension set | Use freshly scanned metadata or perform the current-configuration check in the host |
| Checked projection / `ProjectionBuildError` | Adds `build_with_current_extensions` and `ExtensionConfigurationMismatch { expected, snapshot }` | Adopt the checked builder for cached scans, update exhaustive matches, and rescan mismatches |
| Qualified kinds / built-in adapter | Foreign `callout` or `embed` names do not acquire Obsidian widget semantics | Use the opt-in Obsidian scanner for those widgets or explicit host decorations; compare namespace and local name |
| `HostRendererRegistry<C,O,E>` | New optional API; no graphics dependency or change to existing native views; callbacks may be non-Send `FnMut` | Register exact keys, preflight all concealed widgets, and handle missing/runtime failures; hosts own geometry, caches, current inputs and action application |
| Renderer errors | Typed cause chaining requires `E: Error + 'static`; plain `Box<dyn Error>` does not meet that generic bound | Use a concrete error enum or local boxed-error newtype for `?` and chaining; see the compiling registry rustdoc example |

All workspace packages remain at `0.1.0`, Rust 1.90, with no new dependency. This audit is pre-1.0 upgrade guidance, not a frozen API or ABI promise. Added enum variants and intentionally tightened duplicate/equality semantics matter even though existing method signatures remain available. Error display strings are diagnostics; match typed variants rather than parsing their text.

## Composed consumer evidence

[`extension_pipeline.rs`](../crates/mdedit-live/tests/extension_pipeline.rs) exercises actual exported APIs, without a new automatic host runtime:

- The widget-free CommonMark bold-text fixture builds checked projection with an empty extension set and accepts an empty renderer registry. CommonMark core widgets such as thematic breaks still require handlers under concealment.
- GFM with a legacy application scanner filters a protected inline-code claim, dispatches Unicode reference metadata, returns a checkbox action, and applies that action through `Document::apply`. CRLF/Unicode source is preserved, old scan metadata is rejected after the edit, and a refreshed render returns the reverse toggle action.
- A delayed checkbox action retains the originating projection revision. An intervening insertion causes `Document::apply` to return `StaleRevision`, preserving that insertion instead of overwriting the old byte range in changed source.
- Obsidian combines the legacy built-in scanner with a fallible application scanner. A declared failure stays an error despite an older matching-configuration snapshot. An explicit source-visible frame retains the complete canonical text; subsequent invalidation rejects old metadata and rescanning restores dispatch.
- Successful support preflight does not hide renderer resource failure. A concrete boxed-error newtype preserves the dispatch key/range and I/O cause through `?` into a boxed host result, and source-visible projection remains available.

[`host_renderers.rs`](../crates/mdedit-live/tests/host_renderers.rs) now also verifies that a supported first widget does not prevent detecting an unsupported later widget, and no callbacks run during that preflight. This closes the first deferred Slice 05 item. The registry rustdoc example plus the real boxed-cause pipeline test close its second item.

The preceding scanner, configuration, failure, namespace, overlap, and widget tests remain in the full suite. These representative host scenarios do not claim exhaustive dialect conformance or native acceptance. Temporarily checking only the first widget, disabling configuration validation, or swallowing declared scan errors each caused a targeted assertion failure; restored implementations passed the focused tests and the new rustdoc example.

## Host lifecycle responsibilities

The host retains document identity, parser/dialect epochs, and current source/configuration. Equal revision and byte length alone cannot identify a document. After scanner settings or cached external-state dependencies change, invalidate extension configuration and rescan; after parser configuration changes, also discard incompatible syntax/extension snapshots and advance the parser epoch. A reported scanner failure does not automatically invalidate its ID or authorize reuse of an older successful result.

Renderer/view settings have their own host cache lifecycle. Preflight checks registration, not resource loading or layout correctness. Missing concealed-widget support or runtime rendering failure requires explicit source reveal/rebuild/reflow and disposal of partial paint output as appropriate. Returning a `WidgetAction` does not apply an edit, and the action itself contains no document identity or revision. Retain those with the action, or prepare a transaction immediately using `projection.revision()` and keep it in that originating document's queue. Never substitute the latest `document.revision()` when applying an old range: an intervening edit could otherwise pass the transaction guard and overwrite unrelated text. Reject/recompute actions on `StaleRevision`; a revision alone cannot protect against applying an action to a different document. After a host transaction, refresh revision-dependent syntax, scan, projection, layout and view data before rendering again. Do not dispatch built-in extension metadata twice if a projected widget already represents it.

## Verification and remaining gates

Initial local Windows verification on 2026-11-27 passed 291 tests. Cumulative review then found that delayed actions must use the originating projection revision. The previous latest-revision recipe reproduced corruption from `abc- [ ] task` to `ab[x][ ] task`; the corrected recipe returns `StaleRevision` and preserves the insertion. The example now prepares revision-bound transactions and keeps their per-document association explicit.

Post-review Windows verification on 2026-11-28: **292 passed, 0 failed, 1 ignored**. Format, workspace/all-target check, clippy with warnings denied, rustdoc with warnings denied, and all three public examples passed. This adds five composed-consumer tests, one later-widget preflight case, and one compiling error-newtype rustdoc example to the Slice 05 baseline. The ignored test requires a native graphics adapter (`image_widget::tests::gpu_upload_rejects_oversized_image_and_accepts_next_valid_image`); no new GPU run is claimed. The cumulative review range, verified correction, deferred polish and exact-head push/PR three-OS CI outcomes are tracked in the current PR and execution ledger, without an additional status-only source commit.

The authorized stack merge and fresh integrated-main CI are complete. No new native input, native renderer, GPU, IME, Narrator, or VoiceOver acceptance is claimed by these API tests; earlier Windows/macOS results retain their recorded scope, with Windows host fixes integrated through [PR #14](https://github.com/posea3/Rust-Markdown-Live-Preview-Editor/pull/14). Adopting hosts still need actual native integration acceptance. Phase 9 owns the frozen API/accessibility/1.0 gate.

The next development phase is Phase 8 profiling: measure the documented 10 KB, 100 KB, 1 MB, 10 MB and million-character single-line fixtures before selecting optimizations. This review does not introduce a background parser, virtualized view, incremental projection, or cache based on unmeasured assumptions.
