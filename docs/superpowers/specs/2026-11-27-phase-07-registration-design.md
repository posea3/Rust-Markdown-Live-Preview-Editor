# Phase 7 Slice 02 — Unique scanner registration

The next hardening slice enforces the existing contract that each scanner owns a distinct `ExtensionId`. Duplicate registrations currently share a namespace and can make equal-priority matches depend on scanner emission order. The user requested continuing Phase 7 after publishing the Windows acceptance fixes.

## Approaches

- Change `register` to return `Result`: makes every call fallible, but changes the existing public signature and builder usage.
- Add `try_register` and retain the existing signatures: callers loading configuration can recover from a duplicate, while `register` and `with_extension` remain convenient for known-valid startup configuration and panic on duplicates.

The second approach is the implementation choice for this slice. Existing valid registration code keeps working. A duplicate ID is a configuration error, rather than an override or an additional scanner in the same namespace. This tightens the documented distinct-ID precondition; migration guidance must name the new panic behavior and the recoverable alternative.

## Contract

- Add `ExtensionSet::try_register<E>(&mut self, extension: E) -> Result<(), ExtensionRegistrationError>` with the same scanner bounds as `register`.
- Export `ExtensionRegistrationError::DuplicateId(ExtensionId)`, supporting equality, `Display`, and `std::error::Error`.
- Reject a duplicate before adding it; preserve all existing registrations and never scan the rejected scanner.
- `register` and `with_extension` keep their signatures and panic with a diagnostic containing the conflicting ID.
- Capture each scanner's ID once at registration. Use the captured namespace for all later matches and diagnostics, even if an implementation's `id()` uses mutable state.
- Exact ID equality defines a duplicate. Distinct namespaces can still use the same local kind names, and existing merge/protected-source rules remain intact.

## Scope and verification

Rust 1.90; no new dependency; no scanner trait signature, rendering, input, or canonical-source change. Configuration-epoch identity, panic isolation during scanning, and renderer registration remain later slices.

Public-consumer regressions cover recoverable duplicate rejection and unchanged accepted matches, duplicate panic behavior in both existing APIs, distinct IDs, and registration-time identity with mutable scanners. Run failing behavior tests before implementation, then focused tests, the full workspace suite, fmt/check/clippy, rustdoc, and the public scanner example. Publish this slice separately above the Windows acceptance PR and verify its own current-head CI.
