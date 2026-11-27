# Phase 7 Slice 04 — Declared scanner failures

Continue the remaining scanner-failure policy above Slice 03 (`bacd562`, draft PR #16). The next slice is separate; this task does not authorize merging #16.

## Approach

Changing `SyntaxExtension::scan` to return `Result` would break existing implementations. Silently continuing after a reported failure would publish incomplete extension metadata. The chosen approach adds a separate `FallibleSyntaxExtension` trait and uses strict, all-or-nothing scanning: the first declared error stops the scan and returns its registered namespace and owned cause to the host. Existing scanners and successful candidate merging keep their behavior.

## Contract

- `FallibleSyntaxExtension: Send + Sync` exposes `id`, default `MarkdownText` scan mode, and `try_scan(&ExtensionScanContext) -> Result<Vec<ExtensionCandidate>, ExtensionScanError>`.
- `ExtensionScanError` owns a message and implements Clone/Debug/PartialEq/Eq/Display/Error.
- Add `try_register_fallible`, `register_fallible`, and `with_fallible_extension`. Both scanner interfaces share captured IDs, duplicate rejection, and configuration invalidation. Existing APIs retain their signatures and the const set constructor.
- `ExtensionSet::scan` still returns `Result<ExtensionSnapshot, ExtensionError>`. A declared error becomes `ExtensionError::ScannerFailed { extension, error }`; its namespace is the registration-time ID and its source is the scanner error.
- Stop immediately on the first declared error. Return no partial snapshot, do not invoke later scanners, and do not alter canonical source. Earlier callbacks may already have executed; this is not rollback of scanner-owned side effects.
- Validate document/syntax metadata before calling scanners. On success, retain protected-region filtering, UTF-8/range diagnostics, overlap resolution, and configuration provenance for both interfaces.
- A reported error does not remove a scanner, poison the set, or silently change configuration identity. Hosts discard results from the failed attempt and choose source-visible fallback or a deliberate retry. Invalidate configuration when changing effective scanner settings or any external state used to cache scanner results.
- Rust panics in callbacks continue to propagate. There is no unwind catcher, panic=abort recovery, timeout, process sandbox, or dynamic plugin runtime in this slice.

## Scope and verification

Rust 1.90, no dependency addition, no native renderer/input change. Public tests cover mixed successful registration, captured failure identity/cause, short-circuiting, prevalidation, default/AllSource filtering, malformed-candidate diagnostics, cross-interface duplicates, convenience APIs, panic propagation, recovery after explicit invalidation, and successful fallible metadata in Live Preview. Mutate the failure propagation to suppress errors and observe the failure regressions turn RED, then restore and verify GREEN.

Provide a runnable success/failure example, migration/fallback guidance, and the deferred parser/dialect cache-invalidation clarification from PR #16. Run full workspace checks, rustdoc and examples; obtain an independent review; publish a stacked draft PR above #16 and verify its own final-head CI. Renderer registration remains a separate Phase 7 decision.
