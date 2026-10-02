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
- generic `SyntaxExtension` scanner API with deterministic priority/overlap merge policy
- generic suppression of Markdown-style extensions inside code, HTML, metadata blocks, and raw-source fallback regions
- opt-in Obsidian compatibility scanner using actual Obsidian syntax for wikilinks, embeds, highlights, comments, callouts, and block IDs
- `MarkdownDialect::obsidian()` preset for the standard parser features used alongside the Obsidian extension scanner
- parser-owned semantic metadata preserved in project types for headings, block quotes, code blocks, lists, footnotes, tables, links/images, metadata blocks, and math/code leaf nodes
- file-backed CommonMark/GFM/Obsidian compatibility fixtures
- `ParseReconciler` request/result policy for stale document revisions, parser/config invalidation, superseded requests, and current raw-fallback acceptance

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

## Extension scanner and merge policy

The extension layer is generic. Third-party applications register implementations of `SyntaxExtension`, which emit source-ranged `ExtensionCandidate` values. The merge engine validates UTF-8/source bounds, suppresses Markdown-style extensions inside code/HTML/metadata/raw-fallback regions, then resolves overlapping claims deterministically by priority and containment policy.

No Obsidian-specific type is required by the generic engine.

For immediate compatibility, `ObsidianSyntaxExtension` is shipped as an opt-in implementation and uses current Obsidian Flavored Markdown forms rather than invented test syntax. The initial compatibility surface recognizes:

- `[[Note]]`, aliases, heading links, and block references
- `![[embed]]`, including the pipe suffix used by Obsidian embeds
- `==highlight==`
- `%%comment%%`, including multiline comments
- blockquote callout headers such as `> [!warning]- Title`
- block IDs such as `Paragraph text ^block-id`

CommonMark/GFM/LaTeX-adjacent constructs already handled by the parser remain parser-owned instead of being duplicated in the Obsidian scanner.

This layer recognizes syntax only. Vault path resolution, file rename handling, resource loading, embed rendering, callout styling, and host navigation remain outside `mdedit-markdown`.

## Semantic metadata

`SyntaxNode` now retains parser-owned information in project-owned `SyntaxMetadata` values. This prevents downstream rendering, widgets, accessibility, and host integration from depending on pulldown-cmark types.

The metadata boundary currently preserves:

- heading id/classes/custom attributes when enabled by the selected dialect
- GFM alert/block-quote kind
- indented versus fenced code blocks and fenced info strings
- ordered-list start values
- footnote definition/reference labels
- table column alignments
- link/image type, destination, title, and reference label
- YAML/pluses metadata-block kind
- normalized inline-code and math content

Compatibility tests use file-backed Markdown samples rather than synthesized event streams. CommonMark and GFM fixtures follow their published syntax, while the Obsidian fixture uses actual Obsidian forms for frontmatter, wikilinks, embeds, highlights, comments, callouts, and block IDs.

## Parse snapshot reconciliation

`ParseReconciler` defines the correctness contract that a later background parser must obey without introducing threading or scheduling into Phase 4.

Each parse request receives a `ParseTicket` containing:

- an opaque monotonic request ID
- the canonical document revision
- the canonical source length
- the parser/configuration epoch

A result is accepted only when it matches the ticket and is still the latest request for the current document/configuration. Results are discarded when the document advanced, parser configuration changed, or a newer request superseded them. A ticket/result revision or length mismatch is treated as a programming error rather than a harmless stale result.

Changing dialect/parser/extension configuration invalidates the current accepted syntax snapshot by advancing `ParseConfigEpoch`. A current-revision `RawFallback` remains a valid accepted syntax snapshot because raw source is the safe correctness fallback.

This layer intentionally does not spawn threads, debounce edits, or perform incremental parsing. Phase 8 may provide those mechanisms while reusing this acceptance contract.

## Remaining Phase 4 work

- additional compatibility cases discovered during Phase 5 projection work

Live Preview projection remains Phase 5.
