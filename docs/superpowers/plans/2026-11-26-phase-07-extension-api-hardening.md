# Phase 7 Extension API Hardening — Slice 01

**Goal:** Make application-defined scanners safe to combine with the built-in Obsidian scanner, and demonstrate the public consumer API.

**Spec:** `docs/phase-07-extension-api.md`.

**Architecture:** Keep `SyntaxExtension`, `ExtensionSet`, and source-mapped snapshots. Live Preview dispatches built-in widget behavior using both `ExtensionId` and kind name. Examples and integration tests use exported APIs rather than private implementation details.

**Constraints:** Rust 1.90; no new dependency; canonical source preserved; no native view or IME behavior change. The user explicitly deferred Windows native acceptance on 2026-11-26 so Phase 7 can proceed while that checklist remains open.

## Task 1 — Namespace isolation

- [x] Add public-consumer tests in `crates/mdedit-live/tests/extension_compatibility.rs` for foreign `callout` and `embed` kinds; require ordinary projection with no Obsidian widgets.
- [x] Run `cargo test -p mdedit-live --test extension_compatibility`; confirm the two foreign-kind cases fail by concealing source/emitting an unintended widget.
- [x] Restrict built-in widget dispatch in `crates/mdedit-live/src/projection.rs` to `OBSIDIAN_EXTENSION_ID`.
- [x] Require genuine Obsidian widgets to remain available beside application matches in both registration orders, and verify the focused tests pass.

## Task 2 — Public scanner consumers

- [x] Add `crates/mdedit-markdown/examples/extension_scanners.rs` with generic `{{ref:...}}` and `#123` scanners implemented through the public trait.
- [x] Add consumer tests in `crates/mdedit-markdown/tests/extension_contract.rs`: protected code, UTF-8 invalid ranges, opt-in AllSource behavior, priority/conflict resolution, contained overlaps, and registration-order independence.
- [x] Run the example and the consumer test target; inspect real matched source slices and diagnostics.

## Task 3 — Contract and verification

- [x] Document opt-in registration, qualified identities, source bounds, protected regions, diagnostics, snapshot/config invalidation, and pre-1.0 compatibility guidance in `docs/phase-07-extension-api.md`.
- [x] Update README/roadmap/Phase 6 status to retain Windows acceptance pending while Phase 7 is in progress.
- [x] Run fmt/check/clippy/workspace tests and the public example; review the final diff.

Publication uses a focused Phase 7 draft PR. Its current-head checks are the source of truth for the final Windows/macOS/Ubuntu CI result; Windows native acceptance stays separately deferred.

## Evidence

- Namespace regression: three failing consumer tests before the qualified dispatch fix, three passing afterward.
- Scanner contracts: nine passing consumer integration tests.
- Sample output: `example.references::reference` matched `{{ref:설계 노트}}`, `example.issues::issue` matched `#42`, and inline-code source produced a protection diagnostic.
- Local fmt/check/clippy PASS; workspace tests: 242 passed, 0 failed, 1 ignored.

## Review focus

- Foreign kinds named `embed` or `callout` must not acquire Obsidian rendering semantics.
- Real Obsidian source reveal must retain all canonical markers.
- A malformed scanner range must produce diagnostics rather than corrupt projection.
- Scanner order must not affect overlap winners for distinct IDs with fixed priority.
- Examples must preserve Unicode source and avoid assuming parser-internal node types.
