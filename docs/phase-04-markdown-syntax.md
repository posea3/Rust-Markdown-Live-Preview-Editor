# Phase 4 - Markdown Syntax Engine

Branch: `feature/markdown-syntax-phase-04`

## Purpose

Phase 4 introduces Markdown understanding without introducing Live Preview projection.

The canonical document remains the source owned by `mdedit-core`. The syntax layer reads immutable `DocumentSnapshot` values and returns project-owned syntax data with UTF-8 source ranges.

## First slice

Implemented in this slice:

- `mdedit-markdown` crate
- framework-independent `MarkdownParser` trait
- `MarkdownDialect` presets for CommonMark, GFM, and an extended dialect
- `PulldownCmarkParser` adapter
- `SyntaxSnapshot`
- `SyntaxNode`
- project-owned `SyntaxKind`
- source ranges from pulldown-cmark offset events
- parser-specific types kept private to the adapter
- correctness-first full-document parsing
- raw-source fallback on invalid ranges, unbalanced events, or mismatched end tags
- CommonMark/GFM/malformed-source/fallback tests
- stable top-level `BlockId` assignment and reconciliation cache
- deterministic `BlockFingerprint` values for cache invalidation
- exact-content reuse across source shifts plus conservative edited-block overlap reconciliation
- document/syntax revision and source-length validation at the cache boundary
- `DelimiterResolver` with exact source marker spans for headings, emphasis, strong, strikethrough, inline code, block quotes, list items, links, and autolinks
- unresolved/ambiguous delimiter forms remain visible and are surfaced as `DelimiterIssue` values instead of being guessed

## Boundary

This phase does **not** hide Markdown markers.

For example:

```markdown
This is **important**.
```

remains exactly that source text. Phase 4 may classify the strong node and its range, but concealment and reveal behavior belong to Phase 5.

## Raw-source fallback

A parser adapter must never force downstream code to guess when its structural/source mapping is inconsistent.

If the adapter encounters:

- a source range outside the canonical snapshot,
- a source range that splits UTF-8,
- an unbalanced container event,
- a mismatched end event,

the result becomes a `RawFallback` syntax snapshot whose single child spans the full canonical source.

This is intentionally conservative.

## Dependency boundary

`pulldown-cmark` is an implementation dependency. Public APIs expose no pulldown-cmark `Event`, `Tag`, `Options`, or parser types.

This makes it possible to replace or supplement the parser later without changing the editor-facing syntax model.

## Block identity and cache boundary

`BlockCache` assigns opaque monotonic `BlockId` values to top-level syntax blocks. Identity and content freshness are intentionally separate:

- unchanged blocks keep both `BlockId` and `BlockFingerprint`
- an edited block may keep its `BlockId` while its fingerprint changes
- newly inserted blocks receive a new ID
- exact unchanged blocks are reconciled first, so source shifts do not invalidate identity
- edited-block reuse is only accepted when source overlap is unambiguous in both directions
- ambiguous cases prefer a new ID rather than incorrectly attaching cached layout/projection state

The current cache is a correctness boundary, not an incremental parser. It still consumes a complete syntax snapshot. Phase 8 may later replace the reconciliation strategy after profiling without changing the public block identity contract.

## Delimiter resolution

`DelimiterResolver` consumes the canonical `DocumentSnapshot` and its matching `SyntaxSnapshot`. It never rewrites source and never asks downstream Live Preview code to infer punctuation from rendered text.

Resolved spans currently cover:

- ATX heading prefixes and optional closing hashes
- Setext heading underline markers
- emphasis / strong / strikethrough opening and closing delimiters
- variable-length inline-code backtick fences
- block-quote `>` markers
- unordered and ordered list markers
- link label brackets plus inline/reference destination syntax
- autolink angle brackets

Resolution is conservative. A semantic node whose exact source form cannot be proven produces a `DelimiterIssue`; it produces no concealment span for the ambiguous part. That means later projection can safely leave the raw Markdown visible.

Whitespace adjacent to markers is intentionally not part of the delimiter span. Phase 5 projection policy may decide whether layout whitespace should also collapse when a marker is concealed.

## Remaining Phase 4 work

- extension scanner and merge policy
- richer syntax metadata needed by semantic styling
- explicit compatibility fixtures for CommonMark/GFM/Obsidian-like extensions
- parse snapshot reconciliation rules for later background/incremental parsing

Live Preview projection remains Phase 5.
