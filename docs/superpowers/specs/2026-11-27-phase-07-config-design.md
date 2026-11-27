# Phase 7 Slice 03 — Extension configuration identity

PRs #13, #14, and #15 are merged in order. This slice continues the documented Phase 7 cache-identity work on integrated main `7d46db377c44aeda755d7f8978ead128bcea6c1d`.

## Problem and approach

An extension snapshot currently carries only document revision and length. A cached snapshot can therefore conceal an old Obsidian widget after scanners or their configuration change without any source edit. Different extension sets can also produce different results for the same document.

A host-supplied epoch alone would allow unrelated sets with the same epoch to collide. Deriving identity from scanner IDs would miss different scanner implementations and mutable settings. This slice instead assigns each observed configuration a process-local unique opaque `ExtensionConfigId`. A lazy identity preserves `ExtensionSet::new()` as a const constructor and avoids assigning IDs to configurations nobody observes.

This is an implementation choice within the requested next Phase 7 step. Host-owned document identity and parser configuration remain separate concerns.

## Contract

- Export copyable, comparable, hashable `ExtensionConfigId`; values are opaque process-local cache keys, not persistent IDs or content hashes.
- `ExtensionSet::configuration_id()` returns a stable ID until its configuration is invalidated.
- Successful registration invalidates the cached identity. Duplicate rejection leaves it unchanged.
- `ExtensionSet::invalidate_configuration(&mut self)` invalidates identity after host-owned scanner settings change. Mutable scanner internals cannot be discovered automatically; the host must call this method and rescan.
- Every successful `scan` stamps its `ExtensionSnapshot` with the current ID. Cloning a snapshot preserves it; different sets never share IDs, even with identical scanner namespaces or empty registrations.
- IDs are never reused within a process. Use a checked atomic allocator and fail explicitly on exhaustion rather than wrap. This limit is independent of the source document.
- Add `Projection::build_with_current_extensions`, accepting the existing inputs plus the current `ExtensionSet`. Reject configuration mismatch with `ProjectionBuildError::ExtensionConfigurationMismatch` before building a projection, then retain existing document-revision/length validation.
- Preserve `scan`, `register`, `with_extension`, `Projection::build`, and `Projection::build_with_extensions` signatures. The last remains suitable for a freshly scanned snapshot; it has no current set and therefore cannot validate cached configuration by itself.
- Keep `ExtensionSnapshot` Clone/Debug/PartialEq/Eq. Equality now includes configuration provenance, so equal results from unrelated sets are not interchangeable cache entries.

## Scope and evidence

Rust 1.90, no new dependency, canonical source unchanged, no input or native rendering change. This adds an in-memory cache contract, not a cache implementation, background parser, scanner panic sandbox, or renderer registry.

Public-consumer tests cover stable IDs and clone provenance, different sets, successful/failed registration, explicit mutable-setting invalidation, rejection of stale/cross-set snapshots, fresh widget restoration after rescanning, and continued document validation. Test allocator exhaustion with an isolated counter, preserving the process-global allocator. Run the focused tests, full workspace suite, fmt/check/clippy, rustdoc, and public examples; obtain an independent review and publish a separate draft PR against main with current-head three-OS CI.
