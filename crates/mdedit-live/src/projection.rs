use std::{collections::BTreeMap, error::Error, fmt};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};
use mdedit_markdown::{
    BlockId, BlockSnapshot, DelimiterKind, DelimiterSnapshot, DelimiterSpan, ParseStatus,
    SyntaxKind, SyntaxNode, SyntaxSnapshot,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ProjectedSize(u32);

impl ProjectedSize {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProjectedRange {
    start: ProjectedSize,
    end: ProjectedSize,
}

impl ProjectedRange {
    fn new(start: ProjectedSize, end: ProjectedSize) -> Result<Self, ProjectionBuildError> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(ProjectionBuildError::ReversedProjectedRange { start, end })
        }
    }

    #[must_use]
    pub const fn start(self) -> ProjectedSize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> ProjectedSize {
        self.end
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.0 == self.end.0
    }

    #[must_use]
    pub const fn len(self) -> ProjectedSize {
        ProjectedSize(self.end.0 - self.start.0)
    }

    #[must_use]
    pub fn as_usize_range(self) -> std::ops::Range<usize> {
        self.start.to_usize()..self.end.to_usize()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionBias {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealPolicy {
    SourceVisible,
    ConcealInactive,
}

impl Default for RevealPolicy {
    fn default() -> Self {
        Self::SourceVisible
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevealContext {
    carets: Vec<TextSize>,
    selections: Vec<TextRange>,
    compositions: Vec<TextRange>,
}

impl RevealContext {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            carets: Vec::new(),
            selections: Vec::new(),
            compositions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_caret(mut self, caret: TextSize) -> Self {
        self.carets.push(caret);
        self
    }

    #[must_use]
    pub fn with_selection(mut self, selection: TextRange) -> Self {
        self.selections.push(selection);
        self
    }

    #[must_use]
    pub fn with_composition(mut self, composition: TextRange) -> Self {
        self.compositions.push(composition);
        self
    }

    #[must_use]
    pub fn carets(&self) -> &[TextSize] {
        &self.carets
    }

    #[must_use]
    pub fn selections(&self) -> &[TextRange] {
        &self.selections
    }

    #[must_use]
    pub fn compositions(&self) -> &[TextRange] {
        &self.compositions
    }

    fn reveals(&self, range: TextRange) -> bool {
        self.carets
            .iter()
            .any(|caret| range.start() <= *caret && *caret <= range.end())
            || self
                .selections
                .iter()
                .any(|selection| ranges_touch(range, *selection))
            || self
                .compositions
                .iter()
                .any(|composition| ranges_touch(range, *composition))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RevealGroupId(u32);

impl RevealGroupId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevealGroup {
    id: RevealGroupId,
    owner: SyntaxKind,
    source_range: TextRange,
    delimiters: Vec<DelimiterSpan>,
    revealed: bool,
}

impl RevealGroup {
    #[must_use]
    pub const fn id(&self) -> RevealGroupId {
        self.id
    }

    #[must_use]
    pub const fn owner(&self) -> SyntaxKind {
        self.owner
    }

    #[must_use]
    pub const fn source_range(&self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub fn delimiters(&self) -> &[DelimiterSpan] {
        &self.delimiters
    }

    #[must_use]
    pub const fn revealed(&self) -> bool {
        self.revealed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConcealSpan {
    kind: DelimiterKind,
    source_range: TextRange,
    group: RevealGroupId,
}

impl ConcealSpan {
    #[must_use]
    pub const fn kind(self) -> DelimiterKind {
        self.kind
    }

    #[must_use]
    pub const fn source_range(self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub const fn group(self) -> RevealGroupId {
        self.group
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedSpan {
    source_range: TextRange,
    projected_range: ProjectedRange,
}

impl ProjectedSpan {
    #[must_use]
    pub const fn source_range(self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub const fn projected_range(self) -> ProjectedRange {
        self.projected_range
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedBlock {
    id: BlockId,
    source_range: TextRange,
    projected_range: ProjectedRange,
}

impl ProjectedBlock {
    #[must_use]
    pub const fn id(self) -> BlockId {
        self.id
    }

    #[must_use]
    pub const fn source_range(self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub const fn projected_range(self) -> ProjectedRange {
        self.projected_range
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleKind {
    Heading(u8),
    BlockQuote,
    CodeBlock,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    InlineCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleSpan {
    kind: StyleKind,
    source_range: TextRange,
    projected_range: ProjectedRange,
}

impl StyleSpan {
    #[must_use]
    pub const fn kind(self) -> StyleKind {
        self.kind
    }

    #[must_use]
    pub const fn source_range(self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub const fn projected_range(self) -> ProjectedRange {
        self.projected_range
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionFallbackReason {
    SyntaxRawFallback,
    UnownedDelimiter,
    OverlappingConcealment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionStatus {
    Projected,
    RawFallback(ProjectionFallbackReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionMap {
    source_len: TextSize,
    projected_len: ProjectedSize,
    concealed: Vec<TextRange>,
    visible_spans: Vec<ProjectedSpan>,
}

impl ProjectionMap {
    fn identity(source_len: TextSize) -> Self {
        let projected_len = ProjectedSize(source_len.get());
        let visible_spans = if source_len == TextSize::ZERO {
            Vec::new()
        } else {
            vec![ProjectedSpan {
                source_range: TextRange::new(TextSize::ZERO, source_len)
                    .expect("zero-to-length source range is ordered"),
                projected_range: ProjectedRange {
                    start: ProjectedSize::ZERO,
                    end: projected_len,
                },
            }]
        };
        Self {
            source_len,
            projected_len,
            concealed: Vec::new(),
            visible_spans,
        }
    }

    fn from_concealed(
        source_len: TextSize,
        concealed: Vec<TextRange>,
    ) -> Result<Self, ProjectionBuildError> {
        let concealed = merge_concealed(concealed)?;
        let removed = concealed
            .iter()
            .try_fold(0_u32, |sum, range| sum.checked_add(range.len().get()))
            .ok_or(ProjectionBuildError::ProjectedLengthOverflow)?;
        let projected_len = ProjectedSize(
            source_len
                .get()
                .checked_sub(removed)
                .ok_or(ProjectionBuildError::ConcealmentExceedsSource)?,
        );

        let mut visible_spans = Vec::new();
        let mut source_cursor = TextSize::ZERO;
        let mut projected_cursor = ProjectedSize::ZERO;

        for range in &concealed {
            if source_cursor < range.start() {
                let source_range = TextRange::new(source_cursor, range.start())
                    .expect("visible source range is ordered");
                let len = source_range.len().get();
                let end = ProjectedSize(
                    projected_cursor
                        .get()
                        .checked_add(len)
                        .ok_or(ProjectionBuildError::ProjectedLengthOverflow)?,
                );
                visible_spans.push(ProjectedSpan {
                    source_range,
                    projected_range: ProjectedRange::new(projected_cursor, end)?,
                });
                projected_cursor = end;
            }
            source_cursor = range.end();
        }

        if source_cursor < source_len {
            let source_range = TextRange::new(source_cursor, source_len)
                .expect("trailing visible source range is ordered");
            visible_spans.push(ProjectedSpan {
                source_range,
                projected_range: ProjectedRange::new(projected_cursor, projected_len)?,
            });
        }

        Ok(Self {
            source_len,
            projected_len,
            concealed,
            visible_spans,
        })
    }

    #[must_use]
    pub const fn source_len(&self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub const fn projected_len(&self) -> ProjectedSize {
        self.projected_len
    }

    #[must_use]
    pub fn concealed_ranges(&self) -> &[TextRange] {
        &self.concealed
    }

    #[must_use]
    pub fn visible_spans(&self) -> &[ProjectedSpan] {
        &self.visible_spans
    }

    #[must_use]
    pub fn source_to_projected(&self, source: TextSize) -> Option<ProjectedSize> {
        if source > self.source_len {
            return None;
        }

        let mut removed = 0_u32;
        for range in &self.concealed {
            if source < range.start() {
                break;
            }
            if source <= range.end() {
                return Some(ProjectedSize(range.start().get().checked_sub(removed)?));
            }
            removed = removed.checked_add(range.len().get())?;
        }

        Some(ProjectedSize(source.get().checked_sub(removed)?))
    }

    #[must_use]
    pub fn projected_to_source(
        &self,
        projected: ProjectedSize,
        bias: ProjectionBias,
    ) -> Option<TextSize> {
        if projected > self.projected_len {
            return None;
        }

        let mut removed = 0_u32;
        for range in &self.concealed {
            let collapse = range.start().get().checked_sub(removed)?;
            if projected.get() < collapse {
                return Some(TextSize::new(projected.get().checked_add(removed)?));
            }
            if projected.get() == collapse {
                return Some(match bias {
                    ProjectionBias::Before => range.start(),
                    ProjectionBias::After => range.end(),
                });
            }
            removed = removed.checked_add(range.len().get())?;
        }

        Some(TextSize::new(projected.get().checked_add(removed)?))
    }

    fn source_range_to_projected(
        &self,
        source: TextRange,
    ) -> Result<ProjectedRange, ProjectionBuildError> {
        let start = self
            .source_to_projected(source.start())
            .ok_or(ProjectionBuildError::SourceRangeOutsideMap(source))?;
        let end = self
            .source_to_projected(source.end())
            .ok_or(ProjectionBuildError::SourceRangeOutsideMap(source))?;
        ProjectedRange::new(start, end)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    revision: Revision,
    source_len: TextSize,
    text: String,
    status: ProjectionStatus,
    map: ProjectionMap,
    blocks: Vec<ProjectedBlock>,
    styles: Vec<StyleSpan>,
    reveal_groups: Vec<RevealGroup>,
    concealed: Vec<ConcealSpan>,
}

impl Projection {
    pub fn build(
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
        blocks: &BlockSnapshot,
        delimiters: &DelimiterSnapshot,
        policy: RevealPolicy,
        context: &RevealContext,
    ) -> Result<Self, ProjectionBuildError> {
        validate_inputs(document, syntax, blocks, delimiters)?;
        let source = document.text();

        if matches!(syntax.status(), ParseStatus::RawFallback(_)) {
            return Self::raw(
                document.revision(),
                syntax.source_len(),
                source,
                blocks,
                syntax,
                ProjectionFallbackReason::SyntaxRawFallback,
            );
        }

        let reveal_groups = match build_reveal_groups(syntax.root(), delimiters, policy, context) {
            Ok(groups) => groups,
            Err(ProjectionFallbackReason::UnownedDelimiter) => {
                return Self::raw(
                    document.revision(),
                    syntax.source_len(),
                    source,
                    blocks,
                    syntax,
                    ProjectionFallbackReason::UnownedDelimiter,
                );
            }
            Err(reason) => {
                return Self::raw(
                    document.revision(),
                    syntax.source_len(),
                    source,
                    blocks,
                    syntax,
                    reason,
                );
            }
        };

        let concealed = reveal_groups
            .iter()
            .filter(|group| !group.revealed)
            .flat_map(|group| {
                group.delimiters.iter().map(|delimiter| ConcealSpan {
                    kind: delimiter.kind(),
                    source_range: delimiter.range(),
                    group: group.id,
                })
            })
            .collect::<Vec<_>>();

        if has_overlapping_concealment(&concealed) {
            return Self::raw(
                document.revision(),
                syntax.source_len(),
                source,
                blocks,
                syntax,
                ProjectionFallbackReason::OverlappingConcealment,
            );
        }

        let map = ProjectionMap::from_concealed(
            syntax.source_len(),
            concealed.iter().map(|span| span.source_range).collect(),
        )?;
        let text = materialize_projection(&source, map.concealed_ranges());
        let projected_blocks = project_blocks(blocks, &map)?;
        let styles = project_styles(syntax.root(), &map)?;

        Ok(Self {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            text,
            status: ProjectionStatus::Projected,
            map,
            blocks: projected_blocks,
            styles,
            reveal_groups,
            concealed,
        })
    }

    fn raw(
        revision: Revision,
        source_len: TextSize,
        source: String,
        blocks: &BlockSnapshot,
        syntax: &SyntaxSnapshot,
        reason: ProjectionFallbackReason,
    ) -> Result<Self, ProjectionBuildError> {
        let map = ProjectionMap::identity(source_len);
        let projected_blocks = project_blocks(blocks, &map)?;
        let styles = project_styles(syntax.root(), &map)?;

        Ok(Self {
            revision,
            source_len,
            text: source,
            status: ProjectionStatus::RawFallback(reason),
            map,
            blocks: projected_blocks,
            styles,
            reveal_groups: Vec::new(),
            concealed: Vec::new(),
        })
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn source_len(&self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn status(&self) -> ProjectionStatus {
        self.status
    }

    #[must_use]
    pub const fn map(&self) -> &ProjectionMap {
        &self.map
    }

    #[must_use]
    pub fn blocks(&self) -> &[ProjectedBlock] {
        &self.blocks
    }

    #[must_use]
    pub fn styles(&self) -> &[StyleSpan] {
        &self.styles
    }

    #[must_use]
    pub fn reveal_groups(&self) -> &[RevealGroup] {
        &self.reveal_groups
    }

    #[must_use]
    pub fn concealed_spans(&self) -> &[ConcealSpan] {
        &self.concealed
    }
}

fn validate_inputs(
    document: &DocumentSnapshot,
    syntax: &SyntaxSnapshot,
    blocks: &BlockSnapshot,
    delimiters: &DelimiterSnapshot,
) -> Result<(), ProjectionBuildError> {
    let revision = document.revision();
    if syntax.revision() != revision {
        return Err(ProjectionBuildError::RevisionMismatch {
            input: "syntax",
            document: revision,
            other: syntax.revision(),
        });
    }
    if blocks.revision() != revision {
        return Err(ProjectionBuildError::RevisionMismatch {
            input: "blocks",
            document: revision,
            other: blocks.revision(),
        });
    }
    if delimiters.revision() != revision {
        return Err(ProjectionBuildError::RevisionMismatch {
            input: "delimiters",
            document: revision,
            other: delimiters.revision(),
        });
    }

    let document_len = document
        .len()
        .map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    for (input, len) in [
        ("syntax", syntax.source_len()),
        ("blocks", blocks.source_len()),
        ("delimiters", delimiters.source_len()),
    ] {
        if len != document_len {
            return Err(ProjectionBuildError::SourceLengthMismatch {
                input,
                document: document_len,
                other: len,
            });
        }
    }

    Ok(())
}

fn build_reveal_groups(
    root: &SyntaxNode,
    delimiters: &DelimiterSnapshot,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<RevealGroup>, ProjectionFallbackReason> {
    let mut grouped: BTreeMap<(u32, u32, u32), (SyntaxKind, TextRange, Vec<DelimiterSpan>)> =
        BTreeMap::new();

    for delimiter in delimiters.spans() {
        let Some(owner_range) = smallest_owner_range(root, delimiter.owner(), delimiter.range())
        else {
            return Err(ProjectionFallbackReason::UnownedDelimiter);
        };
        let key = (
            owner_range.start().get(),
            owner_range.end().get(),
            syntax_kind_order(delimiter.owner()),
        );
        grouped
            .entry(key)
            .or_insert_with(|| (delimiter.owner(), owner_range, Vec::new()))
            .2
            .push(*delimiter);
    }

    grouped
        .into_values()
        .enumerate()
        .map(|(index, (owner, source_range, mut delimiters))| {
            delimiters.sort_by_key(|delimiter| {
                (
                    delimiter.range().start().get(),
                    delimiter.range().end().get(),
                )
            });
            let id = RevealGroupId(
                u32::try_from(index + 1)
                    .map_err(|_| ProjectionFallbackReason::OverlappingConcealment)?,
            );
            let revealed = policy == RevealPolicy::SourceVisible || context.reveals(source_range);
            Ok(RevealGroup {
                id,
                owner,
                source_range,
                delimiters,
                revealed,
            })
        })
        .collect()
}

fn smallest_owner_range(
    node: &SyntaxNode,
    owner: SyntaxKind,
    delimiter: TextRange,
) -> Option<TextRange> {
    let mut best = None;
    smallest_owner_range_into(node, owner, delimiter, &mut best);
    best
}

fn smallest_owner_range_into(
    node: &SyntaxNode,
    owner: SyntaxKind,
    delimiter: TextRange,
    best: &mut Option<TextRange>,
) {
    if !range_contains(node.range(), delimiter) {
        return;
    }

    if node.kind() == owner && best.is_none_or(|current| node.range().len() < current.len()) {
        *best = Some(node.range());
    }

    for child in node.children() {
        smallest_owner_range_into(child, owner, delimiter, best);
    }
}

fn has_overlapping_concealment(concealed: &[ConcealSpan]) -> bool {
    let mut ranges = concealed
        .iter()
        .map(|span| span.source_range)
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| (range.start().get(), range.end().get()));

    ranges
        .windows(2)
        .any(|pair| pair[0].end() > pair[1].start())
}

fn merge_concealed(mut ranges: Vec<TextRange>) -> Result<Vec<TextRange>, ProjectionBuildError> {
    ranges.sort_by_key(|range| (range.start().get(), range.end().get()));
    let mut merged: Vec<TextRange> = Vec::new();

    for range in ranges {
        if range.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some(last) if range.start() <= last.end() => {
                if range.end() > last.end() {
                    *last = TextRange::new(last.start(), range.end())
                        .expect("merged conceal range is ordered");
                }
            }
            _ => merged.push(range),
        }
    }

    Ok(merged)
}

fn materialize_projection(source: &str, concealed: &[TextRange]) -> String {
    if concealed.is_empty() {
        return source.to_owned();
    }

    let removed = concealed
        .iter()
        .map(|range| range.len().to_usize())
        .sum::<usize>();
    let mut output = String::with_capacity(source.len().saturating_sub(removed));
    let mut cursor = 0;

    for range in concealed {
        let source_range = range.as_usize_range();
        output.push_str(&source[cursor..source_range.start]);
        cursor = source_range.end;
    }
    output.push_str(&source[cursor..]);
    output
}

fn project_blocks(
    blocks: &BlockSnapshot,
    map: &ProjectionMap,
) -> Result<Vec<ProjectedBlock>, ProjectionBuildError> {
    blocks
        .blocks()
        .iter()
        .map(|block| {
            Ok(ProjectedBlock {
                id: block.id(),
                source_range: block.range(),
                projected_range: map.source_range_to_projected(block.range())?,
            })
        })
        .collect()
}

fn project_styles(
    root: &SyntaxNode,
    map: &ProjectionMap,
) -> Result<Vec<StyleSpan>, ProjectionBuildError> {
    let mut styles = Vec::new();
    project_styles_into(root, map, &mut styles)?;
    styles.sort_by_key(|style| {
        (
            style.source_range.start().get(),
            style.source_range.end().get(),
        )
    });
    Ok(styles)
}

fn project_styles_into(
    node: &SyntaxNode,
    map: &ProjectionMap,
    styles: &mut Vec<StyleSpan>,
) -> Result<(), ProjectionBuildError> {
    if let Some(kind) = style_kind(node.kind()) {
        styles.push(StyleSpan {
            kind,
            source_range: node.range(),
            projected_range: map.source_range_to_projected(node.range())?,
        });
    }

    for child in node.children() {
        project_styles_into(child, map, styles)?;
    }
    Ok(())
}

const fn style_kind(kind: SyntaxKind) -> Option<StyleKind> {
    match kind {
        SyntaxKind::Heading(level) => Some(StyleKind::Heading(level)),
        SyntaxKind::BlockQuote => Some(StyleKind::BlockQuote),
        SyntaxKind::CodeBlock => Some(StyleKind::CodeBlock),
        SyntaxKind::Emphasis => Some(StyleKind::Emphasis),
        SyntaxKind::Strong => Some(StyleKind::Strong),
        SyntaxKind::Strikethrough => Some(StyleKind::Strikethrough),
        SyntaxKind::Link => Some(StyleKind::Link),
        SyntaxKind::InlineCode => Some(StyleKind::InlineCode),
        _ => None,
    }
}

const fn syntax_kind_order(kind: SyntaxKind) -> u32 {
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

const fn range_contains(outer: TextRange, inner: TextRange) -> bool {
    outer.start().get() <= inner.start().get() && inner.end().get() <= outer.end().get()
}

const fn ranges_touch(left: TextRange, right: TextRange) -> bool {
    left.start().get() <= right.end().get() && right.start().get() <= left.end().get()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionBuildError {
    RevisionMismatch {
        input: &'static str,
        document: Revision,
        other: Revision,
    },
    SourceLengthMismatch {
        input: &'static str,
        document: TextSize,
        other: TextSize,
    },
    SourceRangeOutsideMap(TextRange),
    ReversedProjectedRange {
        start: ProjectedSize,
        end: ProjectedSize,
    },
    ProjectedLengthOverflow,
    ConcealmentExceedsSource,
    DocumentTooLarge,
}

impl fmt::Display for ProjectionBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RevisionMismatch {
                input,
                document,
                other,
            } => write!(
                formatter,
                "{input} revision {} does not match document revision {}",
                other.get(),
                document.get()
            ),
            Self::SourceLengthMismatch {
                input,
                document,
                other,
            } => write!(
                formatter,
                "{input} source length {} does not match document length {}",
                other.get(),
                document.get()
            ),
            Self::SourceRangeOutsideMap(range) => {
                write!(
                    formatter,
                    "source range {range:?} is outside the projection map"
                )
            }
            Self::ReversedProjectedRange { start, end } => write!(
                formatter,
                "projected range is reversed: {}..{}",
                start.get(),
                end.get()
            ),
            Self::ProjectedLengthOverflow => write!(formatter, "projected length overflow"),
            Self::ConcealmentExceedsSource => {
                write!(formatter, "concealment exceeds the canonical source length")
            }
            Self::DocumentTooLarge => {
                write!(formatter, "document exceeds the supported source size")
            }
        }
    }
}

impl Error for ProjectionBuildError {}

#[cfg(test)]
mod tests {
    use mdedit_core::{Document, TextRange, TextSize};
    use mdedit_markdown::{
        BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    };

    use super::*;

    fn build(source: &str, policy: RevealPolicy, context: &RevealContext) -> Projection {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::gfm());
        let mut block_cache = BlockCache::new();
        let blocks = block_cache.reconcile(&snapshot, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&snapshot, &syntax).unwrap();

        Projection::build(&snapshot, &syntax, &blocks, &delimiters, policy, context).unwrap()
    }

    #[test]
    fn source_visible_projection_is_identity_and_keeps_semantic_styles() {
        let source = "**bold**";
        let projection = build(source, RevealPolicy::SourceVisible, &RevealContext::new());

        assert_eq!(projection.text(), source);
        assert_eq!(projection.status(), ProjectionStatus::Projected);
        assert!(projection.concealed_spans().is_empty());
        assert_eq!(
            projection.map().source_to_projected(TextSize::new(4)),
            Some(ProjectedSize::new(4))
        );
        assert!(projection.styles().iter().any(|style| {
            style.kind() == StyleKind::Strong
                && style.source_range().as_usize_range() == 0..8
                && style.projected_range().as_usize_range() == 0..8
        }));
    }

    #[test]
    fn inactive_strong_markers_are_concealed_with_bidirectional_mapping() {
        let projection = build(
            "**bold**",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "bold");
        assert_eq!(projection.map().projected_len(), ProjectedSize::new(4));
        assert_eq!(
            projection.map().source_to_projected(TextSize::ZERO),
            Some(ProjectedSize::ZERO)
        );
        assert_eq!(
            projection.map().source_to_projected(TextSize::new(2)),
            Some(ProjectedSize::ZERO)
        );
        assert_eq!(
            projection.map().source_to_projected(TextSize::new(6)),
            Some(ProjectedSize::new(4))
        );
        assert_eq!(
            projection.map().source_to_projected(TextSize::new(8)),
            Some(ProjectedSize::new(4))
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::ZERO, ProjectionBias::Before),
            Some(TextSize::ZERO)
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::ZERO, ProjectionBias::After),
            Some(TextSize::new(2))
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::new(4), ProjectionBias::Before),
            Some(TextSize::new(6))
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::new(4), ProjectionBias::After),
            Some(TextSize::new(8))
        );
        assert!(projection.styles().iter().any(|style| {
            style.kind() == StyleKind::Strong && style.projected_range().as_usize_range() == 0..4
        }));
    }

    #[test]
    fn caret_inside_construct_reveals_the_whole_group() {
        let context = RevealContext::new().with_caret(TextSize::new(4));
        let projection = build("**bold**", RevealPolicy::ConcealInactive, &context);

        assert_eq!(projection.text(), "**bold**");
        assert!(projection.concealed_spans().is_empty());
        assert_eq!(projection.reveal_groups().len(), 1);
        assert!(projection.reveal_groups()[0].revealed());
    }

    #[test]
    fn selection_and_composition_force_reveal() {
        let selection = TextRange::new(TextSize::new(3), TextSize::new(5)).unwrap();
        let selected = build(
            "a *word* z",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_selection(selection),
        );
        assert_eq!(selected.text(), "a *word* z");

        let composition = TextRange::new(TextSize::new(3), TextSize::new(3)).unwrap();
        let composing = build(
            "a *word* z",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_composition(composition),
        );
        assert_eq!(composing.text(), "a *word* z");
    }

    #[test]
    fn nested_inactive_delimiters_collapse_to_one_mapping_boundary() {
        let projection = build(
            "***x***",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "x");
        assert_eq!(
            projection.map().concealed_ranges(),
            &[
                TextRange::new(TextSize::ZERO, TextSize::new(3)).unwrap(),
                TextRange::new(TextSize::new(4), TextSize::new(7)).unwrap(),
            ]
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::ZERO, ProjectionBias::After),
            Some(TextSize::new(3))
        );
    }

    #[test]
    fn structural_projection_hides_only_proven_marker_bytes() {
        let projection = build(
            "# Heading\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), " Heading\n");
        assert_eq!(projection.concealed_spans().len(), 1);
        assert_eq!(
            projection.concealed_spans()[0].kind(),
            DelimiterKind::HeadingPrefix
        );
    }

    #[test]
    fn projected_blocks_preserve_stable_block_ids() {
        let projection = build(
            "# A\n\n**B**\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.blocks().len(), 2);
        assert_ne!(projection.blocks()[0].id(), projection.blocks()[1].id());
        assert_eq!(
            projection.blocks()[0].projected_range().start(),
            ProjectedSize::ZERO
        );
        assert!(projection.blocks()[1].projected_range().start() > ProjectedSize::ZERO);
    }
}
