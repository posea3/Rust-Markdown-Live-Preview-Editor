# Phase 7 Slice 06 — Compatibility and integration review

The user requested the next step after Slice 05: finish the Phase 7 compatibility/integration review. Base is `4ee95b9c8071092f67050702f9a3734606dcad92`, proposed by PR #18 above PRs #16/#17. The user has delegated routine implementation choices through repeated continuation requests; this document records the executor's chosen scope, not a separately approved new product design.

## Approach and scope

Keep existing public signatures and behavior. A documentation-only audit would leave cross-crate lifecycle assumptions unverified; introducing a new orchestration API would expand the contract during its review. Instead, add exported-API integration tests that join existing scanner, projection, renderer, and document-action APIs, plus an explicit migration/readiness report. No dependency, native input/renderer, projection algorithm, or public signature changes are planned.

Review all unmerged Phase 7 changes from integrated main `7d46db3` through the final Slice 06 head, not only this slice. Prior merged namespace/registration contracts remain part of the surrounding context. Mark proposed implementation/compatibility verification separately from merge-to-main, native acceptance, and a future 1.0 gate. Do not merge PRs #16–#18 as part of this task. Phase 8 begins with measured profiling; this review does not authorize speculative optimization.

## Acceptance

- Exercise CommonMark with no scanners, GFM with a legacy application scanner, and Obsidian with both legacy built-in and fallible application scanners, using current checked projection and host support checks.
- Return a checkbox action from the renderer, apply it through a real canonical-source transaction, reject old extension metadata after the revision change, and verify refreshed state while preserving Unicode/CRLF source.
- A declared scan failure must remain an error even with an older matching-configuration snapshot available. Show an explicit source-visible frame, then configuration invalidation, cached-data rejection, and successful recovery.
- A renderer failure after successful preflight must preserve its cause through a concrete host error newtype and `?` into `Box<dyn Error>`; rebuilding source-visible projection must retain canonical source.
- Resolve the two deferred Slice 05 minors: preflight must reach an unsupported later widget, and heterogeneous boxed errors need clear enum/newtype guidance with a compiling Rust example.
- Mutation checks must expose first-widget-only preflight, missing configuration validation, and swallowed declared scan failures; restore code before full verification.
- Run Windows fmt/check/clippy/full workspace tests, rustdoc with warnings denied, and all public examples. One independent cumulative review and exact-head push/PR CI on Windows/macOS/Ubuntu precede completion.

## Limits

Host orchestration in tests demonstrates composition, not a new automatic fallback/runtime abstraction. Hosts still own document identity, parser epochs, current metadata, renderer/cache state, geometry, transactions, runtime error policy and native accessibility. No new native GUI/IME/accessibility acceptance is claimed. Remaining deferred review polish must be recorded without inventing blockers or declaring a frozen 1.0 API.
