use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    fmt,
};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};

use crate::{SyntaxKind, SyntaxSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BlockId(u64);

impl BlockId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BlockFingerprint(u64);

impl BlockFingerprint {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockEntry {
    id: BlockId,
    kind: SyntaxKind,
    range: TextRange,
    fingerprint: BlockFingerprint,
}

impl BlockEntry {
    #[must_use]
    pub const fn id(&self) -> BlockId {
        self.id
    }

    #[must_use]
    pub const fn kind(&self) -> SyntaxKind {
        self.kind
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub const fn fingerprint(&self) -> BlockFingerprint {
        self.fingerprint
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSnapshot {
    revision: Revision,
    source_len: TextSize,
    blocks: Vec<BlockEntry>,
}

impl BlockSnapshot {
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn source_len(&self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub fn blocks(&self) -> &[BlockEntry] {
        &self.blocks
    }
}

#[derive(Clone, Debug)]
pub struct BlockCache {
    next_id: u64,
    previous: Option<BlockSnapshot>,
}

impl Default for BlockCache {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockCache {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_id: 1,
            previous: None,
        }
    }

    pub fn clear(&mut self) {
        self.previous = None;
    }

    pub fn reconcile(
        &mut self,
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
    ) -> Result<BlockSnapshot, BlockCacheError> {
        if document.revision() != syntax.revision() {
            return Err(BlockCacheError::RevisionMismatch {
                document: document.revision(),
                syntax: syntax.revision(),
            });
        }

        let document_len = document
            .len()
            .map_err(|_| BlockCacheError::DocumentTooLarge)?;
        if document_len != syntax.source_len() {
            return Err(BlockCacheError::SourceLengthMismatch {
                document: document_len,
                syntax: syntax.source_len(),
            });
        }

        let source = document.text();
        let candidates = collect_candidates(&source, syntax)?;
        let mut assigned = vec![None; candidates.len()];

        if let Some(previous) = &self.previous {
            assign_exact_matches(previous, &candidates, &mut assigned);
            assign_unambiguous_overlaps(previous, &candidates, &mut assigned);
        }

        let mut blocks = Vec::with_capacity(candidates.len());
        for (candidate, id) in candidates.into_iter().zip(assigned) {
            let id = match id {
                Some(id) => id,
                None => self.allocate_id()?,
            };
            blocks.push(BlockEntry {
                id,
                kind: candidate.kind,
                range: candidate.range,
                fingerprint: candidate.fingerprint,
            });
        }

        let snapshot = BlockSnapshot {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            blocks,
        };
        self.previous = Some(snapshot.clone());
        Ok(snapshot)
    }

    fn allocate_id(&mut self) -> Result<BlockId, BlockCacheError> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(BlockCacheError::IdExhausted)?;
        Ok(BlockId(id))
    }
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    kind: SyntaxKind,
    range: TextRange,
    fingerprint: BlockFingerprint,
}

fn collect_candidates(
    source: &str,
    syntax: &SyntaxSnapshot,
) -> Result<Vec<Candidate>, BlockCacheError> {
    let mut candidates = Vec::with_capacity(syntax.root().children().len());

    for node in syntax.root().children() {
        let range = node.range().as_usize_range();
        if range.start > range.end
            || range.end > source.len()
            || !source.is_char_boundary(range.start)
            || !source.is_char_boundary(range.end)
        {
            return Err(BlockCacheError::InvalidBlockRange(node.range()));
        }

        candidates.push(Candidate {
            kind: node.kind(),
            range: node.range(),
            fingerprint: fingerprint(node.kind(), &source[range]),
        });
    }

    Ok(candidates)
}

fn assign_exact_matches(
    previous: &BlockSnapshot,
    candidates: &[Candidate],
    assigned: &mut [Option<BlockId>],
) {
    let mut by_key: HashMap<(SyntaxKind, BlockFingerprint), VecDeque<BlockId>> = HashMap::new();

    for block in &previous.blocks {
        by_key
            .entry((block.kind, block.fingerprint))
            .or_default()
            .push_back(block.id);
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if let Some(ids) = by_key.get_mut(&(candidate.kind, candidate.fingerprint))
            && let Some(id) = ids.pop_front()
        {
            assigned[index] = Some(id);
        }
    }
}

fn assign_unambiguous_overlaps(
    previous: &BlockSnapshot,
    candidates: &[Candidate],
    assigned: &mut [Option<BlockId>],
) {
    let used_ids = assigned
        .iter()
        .flatten()
        .copied()
        .collect::<std::collections::HashSet<_>>();

    let unmatched_old = previous
        .blocks
        .iter()
        .filter(|block| !used_ids.contains(&block.id))
        .collect::<Vec<_>>();

    for (current_index, candidate) in candidates.iter().enumerate() {
        if assigned[current_index].is_some() {
            continue;
        }

        let mut best: Option<(&BlockEntry, u32)> = None;
        let mut tied = false;

        for old in &unmatched_old {
            if old.kind != candidate.kind || assigned.iter().flatten().any(|id| *id == old.id) {
                continue;
            }

            let overlap = overlap_len(old.range, candidate.range);
            if overlap == 0 {
                continue;
            }

            match best {
                None => {
                    best = Some((old, overlap));
                    tied = false;
                }
                Some((_, best_overlap)) if overlap > best_overlap => {
                    best = Some((old, overlap));
                    tied = false;
                }
                Some((_, best_overlap)) if overlap == best_overlap => {
                    tied = true;
                }
                Some(_) => {}
            }
        }

        let Some((old, _)) = best else {
            continue;
        };
        if tied || !old_prefers_current(old, candidates, assigned, current_index) {
            continue;
        }

        assigned[current_index] = Some(old.id);
    }
}

fn old_prefers_current(
    old: &BlockEntry,
    candidates: &[Candidate],
    assigned: &[Option<BlockId>],
    proposed_index: usize,
) -> bool {
    let proposed_overlap = overlap_len(old.range, candidates[proposed_index].range);
    let mut best_index = proposed_index;
    let mut best_overlap = proposed_overlap;
    let mut tied = false;

    for (index, candidate) in candidates.iter().enumerate() {
        if index == proposed_index || assigned[index].is_some() || candidate.kind != old.kind {
            continue;
        }

        let overlap = overlap_len(old.range, candidate.range);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_index = index;
            tied = false;
        } else if overlap == best_overlap && overlap != 0 {
            tied = true;
        }
    }

    best_index == proposed_index && !tied
}

fn overlap_len(left: TextRange, right: TextRange) -> u32 {
    let start = left.start().get().max(right.start().get());
    let end = left.end().get().min(right.end().get());
    end.saturating_sub(start)
}

fn fingerprint(kind: SyntaxKind, source: &str) -> BlockFingerprint {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001b3;

    let mut hash = OFFSET;
    for byte in kind_code(kind)
        .to_le_bytes()
        .into_iter()
        .chain(source.as_bytes().iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    BlockFingerprint(hash)
}

const fn kind_code(kind: SyntaxKind) -> u32 {
    match kind {
        SyntaxKind::Document => 0,
        SyntaxKind::RawSource => 1,
        SyntaxKind::Paragraph => 2,
        SyntaxKind::Heading(level) => 10 + level as u32,
        SyntaxKind::BlockQuote => 20,
        SyntaxKind::CodeBlock => 21,
        SyntaxKind::HtmlBlock => 22,
        SyntaxKind::List { ordered: false } => 23,
        SyntaxKind::List { ordered: true } => 24,
        SyntaxKind::ListItem => 25,
        SyntaxKind::FootnoteDefinition => 26,
        SyntaxKind::DefinitionList => 27,
        SyntaxKind::DefinitionListTitle => 28,
        SyntaxKind::DefinitionListDefinition => 29,
        SyntaxKind::Table => 30,
        SyntaxKind::TableHead => 31,
        SyntaxKind::TableRow => 32,
        SyntaxKind::TableCell => 33,
        SyntaxKind::Emphasis => 40,
        SyntaxKind::Strong => 41,
        SyntaxKind::Strikethrough => 42,
        SyntaxKind::Superscript => 43,
        SyntaxKind::Subscript => 44,
        SyntaxKind::Link => 45,
        SyntaxKind::Image => 46,
        SyntaxKind::MetadataBlock => 47,
        SyntaxKind::Text => 50,
        SyntaxKind::InlineCode => 51,
        SyntaxKind::InlineMath => 52,
        SyntaxKind::DisplayMath => 53,
        SyntaxKind::Html => 54,
        SyntaxKind::InlineHtml => 55,
        SyntaxKind::FootnoteReference => 56,
        SyntaxKind::SoftBreak => 57,
        SyntaxKind::HardBreak => 58,
        SyntaxKind::Rule => 59,
        SyntaxKind::TaskListMarker { checked: false } => 60,
        SyntaxKind::TaskListMarker { checked: true } => 61,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockCacheError {
    RevisionMismatch {
        document: Revision,
        syntax: Revision,
    },
    SourceLengthMismatch {
        document: TextSize,
        syntax: TextSize,
    },
    InvalidBlockRange(TextRange),
    DocumentTooLarge,
    IdExhausted,
}

impl fmt::Display for BlockCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RevisionMismatch { document, syntax } => write!(
                formatter,
                "document revision {} does not match syntax revision {}",
                document.get(),
                syntax.get()
            ),
            Self::SourceLengthMismatch { document, syntax } => write!(
                formatter,
                "document length {} does not match syntax length {}",
                document.get(),
                syntax.get()
            ),
            Self::InvalidBlockRange(range) => {
                write!(
                    formatter,
                    "syntax block range {range:?} is invalid for the source"
                )
            }
            Self::DocumentTooLarge => {
                write!(formatter, "document exceeds the supported source size")
            }
            Self::IdExhausted => write!(formatter, "block identifier space is exhausted"),
        }
    }
}

impl Error for BlockCacheError {}

#[cfg(test)]
mod tests {
    use super::*;
    use mdedit_core::{Change, ChangeSet, Document, Transaction, TransactionKind};

    use crate::{MarkdownDialect, MarkdownParser, PulldownCmarkParser};

    fn parse(document: &Document) -> SyntaxSnapshot {
        PulldownCmarkParser.parse(&document.snapshot(), &MarkdownDialect::commonmark())
    }

    fn replace(document: &mut Document, range: TextRange, text: &str) {
        let transaction = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range, text)),
            TransactionKind::Programmatic,
        );
        document.apply(transaction).unwrap();
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
    }

    #[test]
    fn assigns_distinct_ids_to_top_level_blocks() {
        let document = Document::new("# A\n\nB\n").unwrap();
        let syntax = parse(&document);
        let mut cache = BlockCache::new();
        let blocks = cache.reconcile(&document.snapshot(), &syntax).unwrap();

        assert_eq!(blocks.blocks().len(), 2);
        assert_ne!(blocks.blocks()[0].id(), blocks.blocks()[1].id());
        assert_eq!(blocks.blocks()[0].kind(), SyntaxKind::Heading(1));
        assert_eq!(blocks.blocks()[1].kind(), SyntaxKind::Paragraph);
    }

    #[test]
    fn unchanged_blocks_keep_ids_when_a_block_is_inserted_before_them() {
        let mut document = Document::new("# A\n\nkeep\n").unwrap();
        let mut cache = BlockCache::new();

        let before_syntax = parse(&document);
        let before = cache
            .reconcile(&document.snapshot(), &before_syntax)
            .unwrap();
        let heading_id = before.blocks()[0].id();
        let paragraph_id = before.blocks()[1].id();

        replace(&mut document, range(0, 0), "intro\n\n");
        let after_syntax = parse(&document);
        let after = cache
            .reconcile(&document.snapshot(), &after_syntax)
            .unwrap();

        assert_eq!(after.blocks().len(), 3);
        assert_eq!(after.blocks()[1].id(), heading_id);
        assert_eq!(after.blocks()[2].id(), paragraph_id);
        assert_ne!(after.blocks()[0].id(), heading_id);
    }

    #[test]
    fn edited_block_keeps_id_but_changes_fingerprint() {
        let mut document = Document::new("# A\n\nhello world\n\n# Z\n").unwrap();
        let mut cache = BlockCache::new();

        let before_syntax = parse(&document);
        let before = cache
            .reconcile(&document.snapshot(), &before_syntax)
            .unwrap();
        let paragraph = before.blocks()[1].clone();

        replace(&mut document, range(8, 13), "there");
        let after_syntax = parse(&document);
        let after = cache
            .reconcile(&document.snapshot(), &after_syntax)
            .unwrap();
        let edited = &after.blocks()[1];

        assert_eq!(edited.id(), paragraph.id());
        assert_ne!(edited.fingerprint(), paragraph.fingerprint());
    }

    #[test]
    fn duplicate_insert_does_not_steal_existing_exact_id() {
        let mut document = Document::new("same\n\nkeep\n").unwrap();
        let mut cache = BlockCache::new();

        let before_syntax = parse(&document);
        let before = cache
            .reconcile(&document.snapshot(), &before_syntax)
            .unwrap();
        let same_id = before.blocks()[0].id();
        let keep_id = before.blocks()[1].id();

        replace(&mut document, range(6, 6), "same\n\n");
        let after_syntax = parse(&document);
        let after = cache
            .reconcile(&document.snapshot(), &after_syntax)
            .unwrap();

        assert_eq!(after.blocks()[0].id(), same_id);
        assert_ne!(after.blocks()[1].id(), same_id);
        assert_eq!(after.blocks()[2].id(), keep_id);
    }

    #[test]
    fn rejects_revision_mismatch() {
        let mut document = Document::new("a\n").unwrap();
        let syntax = parse(&document);
        replace(&mut document, range(0, 1), "b");

        let mut cache = BlockCache::new();
        let error = cache.reconcile(&document.snapshot(), &syntax).unwrap_err();

        assert!(matches!(error, BlockCacheError::RevisionMismatch { .. }));
    }

    #[test]
    fn rejects_source_length_mismatch_even_when_revision_matches() {
        let document = Document::new("a\n").unwrap();
        let other = Document::new("longer\n").unwrap();
        let syntax = parse(&other);

        let mut cache = BlockCache::new();
        let error = cache.reconcile(&document.snapshot(), &syntax).unwrap_err();

        assert!(matches!(
            error,
            BlockCacheError::SourceLengthMismatch { .. }
        ));
    }
}
