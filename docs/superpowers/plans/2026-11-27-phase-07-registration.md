# Phase 7 Scanner Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reject duplicate scanner namespaces while preserving existing valid registration call sites and offering recoverable registration errors.

**Architecture:** `ExtensionSet` stores each scanner with its registration-time ID. A fallible registration method validates uniqueness; existing methods delegate to it and report invalid configuration immediately.

**Tech Stack:** Rust 1.90, existing workspace crates and test tooling.

**Spec:** `docs/superpowers/specs/2026-11-27-phase-07-registration-design.md`.

## Global Constraints

- Rust 1.90; no new dependency.
- Preserve scanner trait and existing registration method signatures.
- No rendering, input, or canonical-source change.
- Keep registration errors separate from scan-time `ExtensionError`.
- Native implementation in the existing task checkout; separate Git branch and PR above Windows acceptance.

## Review Focus

- A duplicate with different scanner behavior must not silently replace or add to the original scanner.
- Duplicate rejection must leave the set usable for another unique registration and later scans.
- Both `register` and `with_extension` must reject duplicate IDs with the same useful diagnostic.
- Mutable `id()` implementations must not bypass uniqueness or move already-registered matches into another namespace.
- Existing unique namespaces, protected ranges, Unicode source, and native consumers must retain their prior behavior.

## Task 1 — Registration contract and consumer evidence

**Files:** modify `crates/mdedit-markdown/src/extension.rs`, `crates/mdedit-markdown/src/lib.rs`; create `crates/mdedit-markdown/tests/extension_registration.rs`.

**Interfaces:** add `try_register<E>(&mut self, extension: E) -> Result<(), ExtensionRegistrationError>` for `E: SyntaxExtension + 'static`; export `ExtensionRegistrationError::DuplicateId(ExtensionId)`; retain `register` and `with_extension` signatures.

- [x] Add public-consumer tests using existing APIs for duplicate `register`, duplicate builder registration, and captured scanner identity; run them and observe the missing behavior.
- [x] Add a private registered-scanner entry, exported registration error, and `try_register`; have existing registration methods delegate to it and use captured IDs during scanning.
- [x] Add direct recoverable-registration tests asserting the exact duplicate ID, error diagnostic, rejection without mutation, and successful distinct registrations; run the focused consumer suite.
- [x] Run `cargo fmt --all -- --check`, `cargo check --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace`.
- [x] Run rustdoc with warnings denied and `cargo run --locked -p mdedit-markdown --example extension_scanners`.
- [x] Review the final diff and update the plan with RED/GREEN and full-suite evidence.

## Task 2 — Guidance and publication

**Files:** update `docs/phase-07-extension-api.md`, `docs/roadmap.md`, `README.md`, and the public scanner example where a recoverable registration example helps.

- [x] Explain the duplicate-ID policy, the panic behavior of existing convenience APIs, `try_register` migration, registration-time IDs, and remaining Phase 7 work.
- [x] Record the Windows acceptance PR and its verified current-head CI without attributing GUI acceptance to later scanner-only changes.
- [ ] Commit and publish a separate draft Phase 7 Slice 02 PR; verify Windows/macOS/Ubuntu CI for its final head.

## Local verification evidence — 2026-11-27

- RED: all three original-API regressions failed before implementation: both convenience methods accepted duplicate IDs, and a mutable scanner renamed its matches. A separate compile run established the fallible API was absent.
- GREEN: five public registration regressions and nine pre-existing scanner contracts passed.
- fmt/check/clippy `-D warnings`: PASS.
- Full workspace suite: **252 passed, 0 failed, 1 ignored**.
- Rustdoc with warnings denied and the public `try_register` consumer example: PASS.
- Windows acceptance PR #14 at `b04dce1`: fresh CI run `45976582080` passed Windows/macOS/Ubuntu. Slice 02 receives its own current-head checks.
