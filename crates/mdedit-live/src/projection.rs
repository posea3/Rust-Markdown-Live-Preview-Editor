use std::{collections::BTreeMap, error::Error, fmt};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};
use mdedit_markdown::{
    BlockId, BlockSnapshot, DelimiterKind, DelimiterSnapshot, DelimiterSpan, ExtensionSnapshot,
    OBSIDIAN_CALLOUT, OBSIDIAN_EMBED, ParseStatus, SyntaxKind, SyntaxMetadata, SyntaxNode,
    SyntaxSnapshot, SyntaxTableAlignment,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevealPolicy {
    #[default]
    SourceVisible,
    ConcealInactive,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructuralPaddingKind {
    HeadingIndent,
    HeadingSeparator,
    HeadingClosingSeparator,
    SetextIndent,
    SetextLineEnding,
    BlockQuoteSeparator,
    ListSeparator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructuralPaddingSpan {
    kind: StructuralPaddingKind,
    source_range: TextRange,
    group: RevealGroupId,
}

impl StructuralPaddingSpan {
    #[must_use]
    pub const fn kind(self) -> StructuralPaddingKind {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    HorizontalRule,
    TaskCheckbox,
    Image,
    DisplayMath,
    Table,
    Callout,
    CustomEmbed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageWidgetData {
    destination: String,
    title: String,
    alt: String,
}

impl ImageWidgetData {
    #[must_use]
    pub fn destination(&self) -> &str {
        &self.destination
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn alt(&self) -> &str {
        &self.alt
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathWidgetData {
    tex: String,
}

impl MathWidgetData {
    #[must_use]
    pub fn tex(&self) -> &str {
        &self.tex
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableWidgetData {
    alignments: Vec<SyntaxTableAlignment>,
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TableWidgetData {
    #[must_use]
    pub fn alignments(&self) -> &[SyntaxTableAlignment] {
        &self.alignments
    }

    #[must_use]
    pub fn header(&self) -> &[String] {
        &self.header
    }

    #[must_use]
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalloutWidgetData {
    callout_type: String,
    title: String,
    fold: Option<char>,
    body: String,
}

impl CalloutWidgetData {
    #[must_use]
    pub fn callout_type(&self) -> &str {
        &self.callout_type
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub const fn fold(&self) -> Option<char> {
        self.fold
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomEmbedWidgetData {
    target: String,
    path: String,
    fragment: Option<String>,
    display: Option<String>,
}

impl CustomEmbedWidgetData {
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    #[must_use]
    pub fn display(&self) -> Option<&str> {
        self.display.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetPayload {
    None,
    TaskCheckbox { checked: bool },
    Image(ImageWidgetData),
    DisplayMath(MathWidgetData),
    Table(TableWidgetData),
    Callout(CalloutWidgetData),
    CustomEmbed(CustomEmbedWidgetData),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetAction {
    SetTaskChecked {
        marker_range: TextRange,
        checked: bool,
    },
}

impl WidgetAction {
    #[must_use]
    pub const fn replacement_text(self) -> &'static str {
        match self {
            Self::SetTaskChecked { checked: true, .. } => "[x]",
            Self::SetTaskChecked { checked: false, .. } => "[ ]",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedWidget {
    kind: WidgetKind,
    payload: WidgetPayload,
    source_range: TextRange,
    projected_range: ProjectedRange,
}

impl ProjectedWidget {
    #[must_use]
    pub const fn kind(&self) -> WidgetKind {
        self.kind
    }

    #[must_use]
    pub const fn payload(&self) -> &WidgetPayload {
        &self.payload
    }

    #[must_use]
    pub const fn source_range(&self) -> TextRange {
        self.source_range
    }

    #[must_use]
    pub const fn projected_range(&self) -> ProjectedRange {
        self.projected_range
    }

    #[must_use]
    pub const fn task_checked(&self) -> Option<bool> {
        match &self.payload {
            WidgetPayload::TaskCheckbox { checked } => Some(*checked),
            WidgetPayload::None
            | WidgetPayload::Image(_)
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::Callout(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }

    #[must_use]
    pub fn image(&self) -> Option<&ImageWidgetData> {
        match &self.payload {
            WidgetPayload::Image(image) => Some(image),
            WidgetPayload::None
            | WidgetPayload::TaskCheckbox { .. }
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::Callout(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }

    #[must_use]
    pub fn display_math(&self) -> Option<&MathWidgetData> {
        match &self.payload {
            WidgetPayload::DisplayMath(math) => Some(math),
            WidgetPayload::None
            | WidgetPayload::TaskCheckbox { .. }
            | WidgetPayload::Image(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::Callout(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }

    #[must_use]
    pub fn table(&self) -> Option<&TableWidgetData> {
        match &self.payload {
            WidgetPayload::Table(table) => Some(table),
            WidgetPayload::None
            | WidgetPayload::TaskCheckbox { .. }
            | WidgetPayload::Image(_)
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Callout(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }

    #[must_use]
    pub fn callout(&self) -> Option<&CalloutWidgetData> {
        match &self.payload {
            WidgetPayload::Callout(callout) => Some(callout),
            WidgetPayload::None
            | WidgetPayload::TaskCheckbox { .. }
            | WidgetPayload::Image(_)
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }

    #[must_use]
    pub fn custom_embed(&self) -> Option<&CustomEmbedWidgetData> {
        match &self.payload {
            WidgetPayload::CustomEmbed(embed) => Some(embed),
            WidgetPayload::None
            | WidgetPayload::TaskCheckbox { .. }
            | WidgetPayload::Image(_)
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::Callout(_) => None,
        }
    }

    #[must_use]
    pub const fn primary_action(&self) -> Option<WidgetAction> {
        match &self.payload {
            WidgetPayload::TaskCheckbox { checked } => Some(WidgetAction::SetTaskChecked {
                marker_range: self.source_range,
                checked: !*checked,
            }),
            WidgetPayload::None
            | WidgetPayload::Image(_)
            | WidgetPayload::DisplayMath(_)
            | WidgetPayload::Table(_)
            | WidgetPayload::Callout(_)
            | WidgetPayload::CustomEmbed(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WidgetCandidate {
    kind: WidgetKind,
    payload: WidgetPayload,
    source_range: TextRange,
    conceal_range: TextRange,
    active: bool,
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
    widgets: Vec<ProjectedWidget>,
    reveal_groups: Vec<RevealGroup>,
    concealed: Vec<ConcealSpan>,
    structural_padding: Vec<StructuralPaddingSpan>,
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
        Self::build_internal(document, syntax, blocks, delimiters, None, policy, context)
    }

    pub fn build_with_extensions(
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
        blocks: &BlockSnapshot,
        delimiters: &DelimiterSnapshot,
        extensions: &ExtensionSnapshot,
        policy: RevealPolicy,
        context: &RevealContext,
    ) -> Result<Self, ProjectionBuildError> {
        Self::build_internal(
            document,
            syntax,
            blocks,
            delimiters,
            Some(extensions),
            policy,
            context,
        )
    }

    fn build_internal(
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
        blocks: &BlockSnapshot,
        delimiters: &DelimiterSnapshot,
        extensions: Option<&ExtensionSnapshot>,
        policy: RevealPolicy,
        context: &RevealContext,
    ) -> Result<Self, ProjectionBuildError> {
        validate_inputs(document, syntax, blocks, delimiters)?;
        if let Some(extensions) = extensions {
            validate_extension_input(document, extensions)?;
        }
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

        let mut concealed = reveal_groups
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

        let mut structural_padding = build_structural_padding(&source, &reveal_groups)?;
        let mut widget_candidates =
            build_widget_candidates(&source, syntax.root(), policy, context)?;
        if let Some(extensions) = extensions {
            widget_candidates.extend(build_extension_widget_candidates(
                &source,
                syntax.root(),
                extensions,
                policy,
                context,
            )?);
            widget_candidates = normalize_widget_candidate_ownership(widget_candidates);
        }
        concealed.retain(|span| !shadowed_by_widget_owner(&widget_candidates, span.source_range));
        structural_padding
            .retain(|span| !shadowed_by_widget_owner(&widget_candidates, span.source_range));

        if has_overlapping_projection_ranges(&concealed, &structural_padding, &widget_candidates) {
            return Self::raw(
                document.revision(),
                syntax.source_len(),
                source,
                blocks,
                syntax,
                ProjectionFallbackReason::OverlappingConcealment,
            );
        }

        let omitted_ranges = concealed
            .iter()
            .map(|span| span.source_range)
            .chain(structural_padding.iter().map(|span| span.source_range))
            .chain(
                widget_candidates
                    .iter()
                    .filter(|widget| !widget.active)
                    .map(|widget| widget.conceal_range),
            )
            .collect();
        let map = ProjectionMap::from_concealed(syntax.source_len(), omitted_ranges)?;
        let text = materialize_projection(&source, map.concealed_ranges());
        let projected_blocks = project_blocks(blocks, &map)?;
        let styles = project_styles(syntax.root(), &map)?;
        let widgets = project_widgets(&widget_candidates, &map)?;

        Ok(Self {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            text,
            status: ProjectionStatus::Projected,
            map,
            blocks: projected_blocks,
            styles,
            widgets,
            reveal_groups,
            concealed,
            structural_padding,
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
            widgets: Vec::new(),
            reveal_groups: Vec::new(),
            concealed: Vec::new(),
            structural_padding: Vec::new(),
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
    pub fn widgets(&self) -> &[ProjectedWidget] {
        &self.widgets
    }

    #[must_use]
    pub fn reveal_groups(&self) -> &[RevealGroup] {
        &self.reveal_groups
    }

    #[must_use]
    pub fn concealed_spans(&self) -> &[ConcealSpan] {
        &self.concealed
    }

    #[must_use]
    pub fn structural_padding_spans(&self) -> &[StructuralPaddingSpan] {
        &self.structural_padding
    }
}

fn validate_extension_input(
    document: &DocumentSnapshot,
    extensions: &ExtensionSnapshot,
) -> Result<(), ProjectionBuildError> {
    let revision = document.revision();
    if extensions.revision() != revision {
        return Err(ProjectionBuildError::RevisionMismatch {
            input: "extensions",
            document: revision,
            other: extensions.revision(),
        });
    }
    let document_len = document
        .len()
        .map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    if extensions.source_len() != document_len {
        return Err(ProjectionBuildError::SourceLengthMismatch {
            input: "extensions",
            document: document_len,
            other: extensions.source_len(),
        });
    }
    Ok(())
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

type RevealGroupKey = (u32, u32, u32);
type PendingRevealGroup = (SyntaxKind, TextRange, Vec<DelimiterSpan>);

fn build_reveal_groups(
    root: &SyntaxNode,
    delimiters: &DelimiterSnapshot,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<RevealGroup>, ProjectionFallbackReason> {
    let mut grouped: BTreeMap<RevealGroupKey, PendingRevealGroup> = BTreeMap::new();

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

    let mut groups = grouped
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
            Ok(RevealGroup {
                id,
                owner,
                source_range,
                delimiters,
                revealed: policy == RevealPolicy::SourceVisible,
            })
        })
        .collect::<Result<Vec<_>, ProjectionFallbackReason>>()?;

    if policy == RevealPolicy::ConcealInactive {
        apply_reveal_context(&mut groups, context);
    }

    Ok(groups)
}

fn apply_reveal_context(groups: &mut [RevealGroup], context: &RevealContext) {
    for caret in context.carets() {
        reveal_point(groups, *caret);
    }
    for selection in context.selections() {
        reveal_interaction_range(groups, *selection);
    }
    for composition in context.compositions() {
        reveal_interaction_range(groups, *composition);
    }
}

fn reveal_point(groups: &mut [RevealGroup], point: TextSize) {
    let candidates = groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            range_contains_point(group.source_range, point).then_some(index)
        })
        .collect::<Vec<_>>();

    let reveal = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            !candidates.iter().copied().any(|other| {
                other != *candidate
                    && strictly_contains(
                        groups[*candidate].source_range,
                        groups[other].source_range,
                    )
                    && range_contains_point(groups[other].source_range, point)
            })
        })
        .collect::<Vec<_>>();

    for index in reveal {
        groups[index].revealed = true;
    }
}

fn reveal_interaction_range(groups: &mut [RevealGroup], interaction: TextRange) {
    if interaction.is_empty() {
        reveal_point(groups, interaction.start());
        return;
    }

    let candidates = groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            ranges_overlap_nonempty(group.source_range, interaction).then_some(index)
        })
        .collect::<Vec<_>>();

    let reveal = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            !candidates.iter().copied().any(|other| {
                other != *candidate
                    && strictly_contains(
                        groups[*candidate].source_range,
                        groups[other].source_range,
                    )
                    && range_contains(groups[other].source_range, interaction)
            })
        })
        .collect::<Vec<_>>();

    for index in reveal {
        groups[index].revealed = true;
    }
}

const fn range_contains_point(range: TextRange, point: TextSize) -> bool {
    range.start().get() <= point.get() && point.get() <= range.end().get()
}

const fn strictly_contains(outer: TextRange, inner: TextRange) -> bool {
    range_contains(outer, inner)
        && (outer.start().get() < inner.start().get() || inner.end().get() < outer.end().get())
}

const fn ranges_overlap_nonempty(left: TextRange, right: TextRange) -> bool {
    left.start().get() < right.end().get() && right.start().get() < left.end().get()
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

fn build_structural_padding(
    source: &str,
    groups: &[RevealGroup],
) -> Result<Vec<StructuralPaddingSpan>, ProjectionBuildError> {
    let mut padding = Vec::new();

    for group in groups.iter().filter(|group| !group.revealed) {
        for delimiter in &group.delimiters {
            let range = delimiter.range().as_usize_range();
            match delimiter.kind() {
                DelimiterKind::HeadingPrefix => {
                    let line_start = source[..range.start]
                        .rfind('\n')
                        .map_or(0, |index| index + 1);
                    if source.as_bytes()[line_start..range.start]
                        .iter()
                        .all(|byte| *byte == b' ')
                    {
                        push_structural_padding(
                            &mut padding,
                            source,
                            group.id,
                            StructuralPaddingKind::HeadingIndent,
                            line_start,
                            range.start,
                        )?;
                    }

                    let end = horizontal_whitespace_end(source, range.end, source.len());
                    push_structural_padding(
                        &mut padding,
                        source,
                        group.id,
                        StructuralPaddingKind::HeadingSeparator,
                        range.end,
                        end,
                    )?;
                }
                DelimiterKind::HeadingSuffix => {
                    let start = horizontal_whitespace_start(source, range.start, 0);
                    push_structural_padding(
                        &mut padding,
                        source,
                        group.id,
                        StructuralPaddingKind::HeadingClosingSeparator,
                        start,
                        range.start,
                    )?;
                }
                DelimiterKind::SetextUnderline => {
                    let line_start = source[..range.start]
                        .rfind('\n')
                        .map_or(0, |index| index + 1);
                    if source.as_bytes()[line_start..range.start]
                        .iter()
                        .all(|byte| matches!(byte, b' ' | b'\t'))
                    {
                        push_structural_padding(
                            &mut padding,
                            source,
                            group.id,
                            StructuralPaddingKind::SetextIndent,
                            line_start,
                            range.start,
                        )?;
                    }

                    let mut end = horizontal_whitespace_end(source, range.end, source.len());
                    if source.as_bytes().get(end) == Some(&b'\r')
                        && source.as_bytes().get(end + 1) == Some(&b'\n')
                    {
                        end += 2;
                    } else if source.as_bytes().get(end) == Some(&b'\n') {
                        end += 1;
                    }
                    push_structural_padding(
                        &mut padding,
                        source,
                        group.id,
                        StructuralPaddingKind::SetextLineEnding,
                        range.end,
                        end,
                    )?;
                }
                DelimiterKind::BlockQuoteMarker => {
                    let end = match source.as_bytes().get(range.end) {
                        Some(b' ' | b'\t') => range.end + 1,
                        _ => range.end,
                    };
                    push_structural_padding(
                        &mut padding,
                        source,
                        group.id,
                        StructuralPaddingKind::BlockQuoteSeparator,
                        range.end,
                        end,
                    )?;
                }
                DelimiterKind::ListMarker => {
                    let end = list_separator_end(source, range.end);
                    push_structural_padding(
                        &mut padding,
                        source,
                        group.id,
                        StructuralPaddingKind::ListSeparator,
                        range.end,
                        end,
                    )?;
                }
                _ => {}
            }
        }
    }

    padding.sort_by_key(|span| {
        (
            span.source_range.start().get(),
            span.source_range.end().get(),
            structural_padding_kind_code(span.kind),
        )
    });
    padding.dedup_by(|left, right| {
        left.kind == right.kind
            && left.source_range == right.source_range
            && left.group == right.group
    });
    Ok(padding)
}

fn push_structural_padding(
    output: &mut Vec<StructuralPaddingSpan>,
    source: &str,
    group: RevealGroupId,
    kind: StructuralPaddingKind,
    start: usize,
    end: usize,
) -> Result<(), ProjectionBuildError> {
    if start == end {
        return Ok(());
    }
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(ProjectionBuildError::InvalidStructuralPaddingBounds { start, end });
    }

    let start =
        TextSize::try_from_usize(start).map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    let end = TextSize::try_from_usize(end).map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    let source_range = TextRange::new(start, end).map_err(|_| {
        ProjectionBuildError::InvalidStructuralPaddingBounds {
            start: start.to_usize(),
            end: end.to_usize(),
        }
    })?;
    output.push(StructuralPaddingSpan {
        kind,
        source_range,
        group,
    });
    Ok(())
}

fn horizontal_whitespace_end(source: &str, start: usize, limit: usize) -> usize {
    let mut cursor = start;
    let bytes = source.as_bytes();
    while cursor < limit && matches!(bytes[cursor], b' ' | b'\t') {
        cursor += 1;
    }
    cursor
}

fn horizontal_whitespace_start(source: &str, end: usize, limit: usize) -> usize {
    let mut cursor = end;
    let bytes = source.as_bytes();
    while cursor > limit && matches!(bytes[cursor - 1], b' ' | b'\t') {
        cursor -= 1;
    }
    cursor
}

fn list_separator_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    match bytes.get(start) {
        Some(b'\t') => start + 1,
        Some(b' ') => {
            let mut cursor = start;
            while cursor < bytes.len() && bytes[cursor] == b' ' {
                cursor += 1;
            }
            let count = cursor - start;
            if count <= 4 { cursor } else { start + 1 }
        }
        _ => start,
    }
}

const fn structural_padding_kind_code(kind: StructuralPaddingKind) -> u8 {
    match kind {
        StructuralPaddingKind::HeadingIndent => 0,
        StructuralPaddingKind::HeadingSeparator => 1,
        StructuralPaddingKind::HeadingClosingSeparator => 2,
        StructuralPaddingKind::SetextIndent => 3,
        StructuralPaddingKind::SetextLineEnding => 4,
        StructuralPaddingKind::BlockQuoteSeparator => 5,
        StructuralPaddingKind::ListSeparator => 6,
    }
}

fn shadowed_by_widget_owner(widgets: &[WidgetCandidate], range: TextRange) -> bool {
    widgets.iter().any(|widget| {
        (!widget.active && range_contains(widget.conceal_range, range))
            || (matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed)
                && range_contains(widget.source_range, range))
    })
}

fn has_overlapping_projection_ranges(
    concealed: &[ConcealSpan],
    structural_padding: &[StructuralPaddingSpan],
    widgets: &[WidgetCandidate],
) -> bool {
    let mut ranges = concealed
        .iter()
        .map(|span| span.source_range)
        .chain(structural_padding.iter().map(|span| span.source_range))
        .chain(
            widgets
                .iter()
                .filter(|widget| !widget.active)
                .map(|widget| widget.conceal_range),
        )
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

fn build_widget_candidates(
    source: &str,
    root: &SyntaxNode,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<WidgetCandidate>, ProjectionBuildError> {
    let mut widgets = Vec::new();
    collect_widget_candidates(source, root, policy, context, &mut widgets)?;
    widgets.sort_by_key(|widget| {
        (
            widget.source_range.start().get(),
            widget.source_range.end().get(),
        )
    });
    Ok(widgets)
}

fn collect_widget_candidates(
    source: &str,
    node: &SyntaxNode,
    policy: RevealPolicy,
    context: &RevealContext,
    output: &mut Vec<WidgetCandidate>,
) -> Result<(), ProjectionBuildError> {
    match node.kind() {
        SyntaxKind::Rule => {
            let source_range = node.range();
            let conceal_range = horizontal_rule_conceal_range(source, source_range)?;
            let active = policy == RevealPolicy::SourceVisible
                || context_touches_range(context, conceal_range);
            output.push(WidgetCandidate {
                kind: WidgetKind::HorizontalRule,
                payload: WidgetPayload::None,
                source_range,
                conceal_range,
                active,
            });
        }
        SyntaxKind::TaskListMarker { checked } => {
            let source_range = node.range();
            let active = policy == RevealPolicy::SourceVisible
                || context_touches_range(context, source_range);
            output.push(WidgetCandidate {
                kind: WidgetKind::TaskCheckbox,
                payload: WidgetPayload::TaskCheckbox { checked },
                source_range,
                conceal_range: source_range,
                active,
            });
        }
        SyntaxKind::Table => {
            let source_range = node.range();
            let active = policy == RevealPolicy::SourceVisible
                || context_touches_range(context, source_range);
            if let Some(table) = table_widget_data(source, node) {
                output.push(WidgetCandidate {
                    kind: WidgetKind::Table,
                    payload: WidgetPayload::Table(table),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
                if !active {
                    return Ok(());
                }
            }
        }
        SyntaxKind::Image if image_is_standalone(source, node.range()) => {
            let source_range = node.range();
            let active = policy == RevealPolicy::SourceVisible
                || context_touches_range(context, source_range);
            if let Some(image) = image_widget_data(source, node) {
                output.push(WidgetCandidate {
                    kind: WidgetKind::Image,
                    payload: WidgetPayload::Image(image),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
        }
        SyntaxKind::DisplayMath => {
            let source_range = node.range();
            let active = policy == RevealPolicy::SourceVisible
                || context_touches_range(context, source_range);
            if let SyntaxMetadata::DisplayMath { content } = node.metadata() {
                output.push(WidgetCandidate {
                    kind: WidgetKind::DisplayMath,
                    payload: WidgetPayload::DisplayMath(MathWidgetData {
                        tex: content.clone(),
                    }),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
        }
        _ => {}
    }

    for child in node.children() {
        collect_widget_candidates(source, child, policy, context, output)?;
    }
    Ok(())
}

fn build_extension_widget_candidates(
    source: &str,
    root: &SyntaxNode,
    extensions: &ExtensionSnapshot,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<WidgetCandidate>, ProjectionBuildError> {
    let mut widgets = Vec::new();

    for extension in extensions.matches() {
        match extension.kind().name() {
            OBSIDIAN_CALLOUT => {
                let marker_range = extension.range();
                let Some(blockquote) =
                    smallest_containing_node(root, marker_range, SyntaxKind::BlockQuote)
                else {
                    continue;
                };
                let source_range = extend_range_to_line_ending(source, blockquote.range())?;
                let active = policy == RevealPolicy::SourceVisible
                    || context_touches_range(context, source_range);
                let callout_type = extension.attribute("type").unwrap_or("note").to_owned();
                let title = extension.attribute("title").unwrap_or("").to_owned();
                let fold = extension
                    .attribute("fold")
                    .and_then(|value| value.chars().next());
                let body = callout_body_visible_text(source, blockquote, source_range);

                widgets.push(WidgetCandidate {
                    kind: WidgetKind::Callout,
                    payload: WidgetPayload::Callout(CalloutWidgetData {
                        callout_type,
                        title,
                        fold,
                        body,
                    }),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
            OBSIDIAN_EMBED if image_is_standalone(source, extension.range()) => {
                let source_range = extension.range();
                let active = policy == RevealPolicy::SourceVisible
                    || context_touches_range(context, source_range);
                widgets.push(WidgetCandidate {
                    kind: WidgetKind::CustomEmbed,
                    payload: WidgetPayload::CustomEmbed(CustomEmbedWidgetData {
                        target: extension.attribute("target").unwrap_or("").to_owned(),
                        path: extension.attribute("path").unwrap_or("").to_owned(),
                        fragment: extension.attribute("fragment").map(str::to_owned),
                        display: extension.attribute("display").map(str::to_owned),
                    }),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
            _ => {}
        }
    }

    widgets.sort_by_key(|widget| {
        (
            widget.source_range.start().get(),
            widget.source_range.end().get(),
        )
    });
    Ok(widgets)
}

fn normalize_widget_candidate_ownership(mut widgets: Vec<WidgetCandidate>) -> Vec<WidgetCandidate> {
    let extension_owner_ranges = widgets
        .iter()
        .filter(|widget| matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed))
        .map(|widget| widget.source_range)
        .collect::<Vec<_>>();

    widgets.retain(|widget| {
        if matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed) {
            return true;
        }

        !extension_owner_ranges
            .iter()
            .any(|range| range_contains(*range, widget.source_range))
    });
    widgets.sort_by_key(|widget| {
        (
            widget.source_range.start().get(),
            widget.source_range.end().get(),
        )
    });
    widgets
}

fn smallest_containing_node(
    node: &SyntaxNode,
    range: TextRange,
    kind: SyntaxKind,
) -> Option<&SyntaxNode> {
    let mut best = None;
    if node.kind() == kind && range_contains(node.range(), range) {
        best = Some(node);
    }

    for child in node.children() {
        if !range_contains(child.range(), range) {
            continue;
        }
        if let Some(candidate) = smallest_containing_node(child, range, kind) {
            best = match best {
                Some(existing) if existing.range().len() <= candidate.range().len() => {
                    Some(existing)
                }
                _ => Some(candidate),
            };
        }
    }
    best
}

fn extend_range_to_line_ending(
    source: &str,
    range: TextRange,
) -> Result<TextRange, ProjectionBuildError> {
    let bytes = range.as_usize_range();
    if bytes.end > source.len()
        || !source.is_char_boundary(bytes.start)
        || !source.is_char_boundary(bytes.end)
    {
        return Err(ProjectionBuildError::InvalidWidgetBounds {
            start: bytes.start,
            end: bytes.end,
        });
    }

    let mut end = bytes.end;
    let already_has_line_ending =
        end > bytes.start && matches!(source.as_bytes()[end - 1], b'\r' | b'\n');
    if !already_has_line_ending && end < source.len() {
        if source.as_bytes()[end] == b'\r' {
            end += 1;
        }
        if end < source.len() && source.as_bytes()[end] == b'\n' {
            end += 1;
        }
    }

    TextRange::new(
        TextSize::try_from_usize(bytes.start)
            .map_err(|_| ProjectionBuildError::DocumentTooLarge)?,
        TextSize::try_from_usize(end).map_err(|_| ProjectionBuildError::DocumentTooLarge)?,
    )
    .map_err(|_| ProjectionBuildError::InvalidWidgetBounds {
        start: bytes.start,
        end,
    })
}

fn callout_body_visible_text(source: &str, blockquote: &SyntaxNode, range: TextRange) -> String {
    let source_range = range.as_usize_range();
    let Some(block_source) = source.get(source_range.clone()) else {
        return String::new();
    };
    let Some(first_line_break) = block_source.find('\n') else {
        return String::new();
    };
    let body_start = source_range.start + first_line_break + 1;
    let Ok(body_start) = TextSize::try_from_usize(body_start) else {
        return String::new();
    };

    let mut body = String::new();
    collect_visible_callout_text(source, blockquote, body_start, &mut body);
    body.trim().to_owned()
}

fn collect_visible_callout_text(
    source: &str,
    node: &SyntaxNode,
    body_start: TextSize,
    output: &mut String,
) {
    if node.range().end() <= body_start {
        return;
    }

    match node.kind() {
        SyntaxKind::Text | SyntaxKind::Html | SyntaxKind::InlineHtml => {
            if let Some(text) = source_text(source, node.range()) {
                output.push_str(text);
            }
            return;
        }
        SyntaxKind::InlineCode => {
            if let SyntaxMetadata::InlineCode { content } = node.metadata() {
                output.push_str(content);
            }
            return;
        }
        SyntaxKind::InlineMath => {
            if let SyntaxMetadata::InlineMath { content } = node.metadata() {
                output.push_str(content);
            }
            return;
        }
        SyntaxKind::FootnoteReference => {
            if let SyntaxMetadata::FootnoteReference { label } = node.metadata() {
                output.push_str(label);
            }
            return;
        }
        SyntaxKind::SoftBreak | SyntaxKind::HardBreak => {
            output.push('\n');
            return;
        }
        _ => {}
    }

    for child in node.children() {
        collect_visible_callout_text(source, child, body_start, output);
    }
}

fn table_widget_data(source: &str, node: &SyntaxNode) -> Option<TableWidgetData> {
    let SyntaxMetadata::Table { alignments } = node.metadata() else {
        return None;
    };

    let header = node
        .children()
        .iter()
        .find(|child| child.kind() == SyntaxKind::TableHead)
        .map_or_else(Vec::new, |head| table_cells(source, head));
    let rows = node
        .children()
        .iter()
        .filter(|child| child.kind() == SyntaxKind::TableRow)
        .map(|row| table_cells(source, row))
        .collect::<Vec<_>>();

    if header.is_empty() && rows.is_empty() {
        return None;
    }

    Some(TableWidgetData {
        alignments: alignments.clone(),
        header,
        rows,
    })
}

fn table_cells(source: &str, row: &SyntaxNode) -> Vec<String> {
    row.children()
        .iter()
        .filter(|child| child.kind() == SyntaxKind::TableCell)
        .map(|cell| {
            let mut text = String::new();
            collect_table_cell_visible_text(source, cell, &mut text);
            text.trim().to_owned()
        })
        .collect()
}

fn collect_table_cell_visible_text(source: &str, node: &SyntaxNode, output: &mut String) {
    match node.kind() {
        SyntaxKind::Text | SyntaxKind::Html | SyntaxKind::InlineHtml => {
            if let Some(text) = source_text(source, node.range()) {
                output.push_str(text);
            }
            return;
        }
        SyntaxKind::InlineCode => {
            if let SyntaxMetadata::InlineCode { content } = node.metadata() {
                output.push_str(content);
            }
            return;
        }
        SyntaxKind::InlineMath => {
            if let SyntaxMetadata::InlineMath { content } = node.metadata() {
                output.push_str(content);
            }
            return;
        }
        SyntaxKind::FootnoteReference => {
            if let SyntaxMetadata::FootnoteReference { label } = node.metadata() {
                output.push_str(label);
            }
            return;
        }
        SyntaxKind::SoftBreak | SyntaxKind::HardBreak => {
            output.push(' ');
            return;
        }
        _ => {}
    }

    for child in node.children() {
        collect_table_cell_visible_text(source, child, output);
    }
}

fn source_text(source: &str, range: TextRange) -> Option<&str> {
    let range = range.as_usize_range();
    (range.end <= source.len()
        && source.is_char_boundary(range.start)
        && source.is_char_boundary(range.end))
    .then(|| &source[range])
}

fn image_widget_data(source: &str, node: &SyntaxNode) -> Option<ImageWidgetData> {
    let SyntaxMetadata::Image(metadata) = node.metadata() else {
        return None;
    };

    let mut alt = String::new();
    collect_image_alt_text(source, node, &mut alt);

    Some(ImageWidgetData {
        destination: metadata.destination().to_owned(),
        title: metadata.title().to_owned(),
        alt,
    })
}

fn collect_image_alt_text(source: &str, node: &SyntaxNode, output: &mut String) {
    for child in node.children() {
        if child.kind() == SyntaxKind::Text {
            let range = child.range().as_usize_range();
            if range.end <= source.len()
                && source.is_char_boundary(range.start)
                && source.is_char_boundary(range.end)
            {
                output.push_str(&source[range]);
            }
        } else {
            collect_image_alt_text(source, child, output);
        }
    }
}

fn image_is_standalone(source: &str, source_range: TextRange) -> bool {
    let range = source_range.as_usize_range();
    if range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return false;
    }

    let line_start = source[..range.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line_end = source[range.end..]
        .find('\n')
        .map_or(source.len(), |relative| range.end + relative);

    source[line_start..line_end].trim() == source[range].trim()
}

fn horizontal_rule_conceal_range(
    source: &str,
    source_range: TextRange,
) -> Result<TextRange, ProjectionBuildError> {
    let range = source_range.as_usize_range();
    if range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return Err(ProjectionBuildError::InvalidWidgetBounds {
            start: range.start,
            end: range.end,
        });
    }

    let bytes = source.as_bytes();
    let mut end = range.end;
    while end > range.start && matches!(bytes[end - 1], b'\r' | b'\n') {
        end -= 1;
    }
    if end == range.start {
        return Err(ProjectionBuildError::InvalidWidgetBounds {
            start: range.start,
            end: range.end,
        });
    }

    let start = TextSize::try_from_usize(range.start)
        .map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    let end = TextSize::try_from_usize(end).map_err(|_| ProjectionBuildError::DocumentTooLarge)?;
    TextRange::new(start, end).map_err(|_| ProjectionBuildError::InvalidWidgetBounds {
        start: range.start,
        end: range.end,
    })
}

fn context_touches_range(context: &RevealContext, range: TextRange) -> bool {
    context
        .carets()
        .iter()
        .any(|caret| range_contains_point(range, *caret))
        || context.selections().iter().any(|selection| {
            if selection.is_empty() {
                range_contains_point(range, selection.start())
            } else {
                ranges_overlap_nonempty(range, *selection)
            }
        })
        || context.compositions().iter().any(|composition| {
            if composition.is_empty() {
                range_contains_point(range, composition.start())
            } else {
                ranges_overlap_nonempty(range, *composition)
            }
        })
}

fn project_widgets(
    candidates: &[WidgetCandidate],
    map: &ProjectionMap,
) -> Result<Vec<ProjectedWidget>, ProjectionBuildError> {
    candidates
        .iter()
        .filter(|widget| !widget.active)
        .map(|widget| {
            Ok(ProjectedWidget {
                kind: widget.kind,
                payload: widget.payload.clone(),
                source_range: widget.source_range,
                projected_range: map.source_range_to_projected(widget.conceal_range)?,
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
    InvalidStructuralPaddingBounds {
        start: usize,
        end: usize,
    },
    InvalidWidgetBounds {
        start: usize,
        end: usize,
    },
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
            Self::InvalidStructuralPaddingBounds { start, end } => {
                write!(
                    formatter,
                    "structural padding bounds {start}..{end} are invalid"
                )
            }
            Self::InvalidWidgetBounds { start, end } => {
                write!(formatter, "widget bounds {start}..{end} are invalid")
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
        obsidian_extension_set,
    };

    use super::*;

    fn build(source: &str, policy: RevealPolicy, context: &RevealContext) -> Projection {
        build_with_dialect(source, MarkdownDialect::gfm(), policy, context)
    }

    fn build_with_dialect(
        source: &str,
        dialect: MarkdownDialect,
        policy: RevealPolicy,
        context: &RevealContext,
    ) -> Projection {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &dialect);
        let mut block_cache = BlockCache::new();
        let blocks = block_cache.reconcile(&snapshot, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&snapshot, &syntax).unwrap();

        Projection::build(&snapshot, &syntax, &blocks, &delimiters, policy, context).unwrap()
    }

    fn build_with_obsidian_extensions(
        source: &str,
        policy: RevealPolicy,
        context: &RevealContext,
    ) -> Projection {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let dialect = MarkdownDialect::obsidian();
        let syntax = PulldownCmarkParser.parse(&snapshot, &dialect);
        let extensions = obsidian_extension_set().scan(&snapshot, &syntax).unwrap();
        let mut block_cache = BlockCache::new();
        let blocks = block_cache.reconcile(&snapshot, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&snapshot, &syntax).unwrap();

        Projection::build_with_extensions(
            &snapshot,
            &syntax,
            &blocks,
            &delimiters,
            &extensions,
            policy,
            context,
        )
        .unwrap()
    }

    #[test]
    fn inactive_obsidian_callout_becomes_native_widget_with_full_block_payload() {
        let source = concat!(
            "before\n\n",
            "> [!warning]- Custom title\n",
            "> Be careful.\n",
            "> Second line.\n",
            "\nafter\n",
        );
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        let widget = projection
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Callout)
            .expect("callout widget");
        let callout = widget.callout().expect("callout payload");

        assert_eq!(callout.callout_type(), "warning");
        assert_eq!(callout.title(), "Custom title");
        assert_eq!(callout.fold(), Some('-'));
        assert_eq!(callout.body(), "Be careful.\nSecond line.");
        assert_eq!(
            &source[widget.source_range().as_usize_range()],
            "> [!warning]- Custom title\n> Be careful.\n> Second line.\n"
        );
        assert!(widget.projected_range().is_empty());
        assert!(!projection.text().contains("[!warning]"));
        assert!(!projection.text().contains("Be careful."));
    }

    #[test]
    fn callout_payload_uses_visible_body_text_without_inline_markdown_markers() {
        let source = concat!(
            "> [!note] Rendered body\n",
            "> **bold** [link](https://example.com) `code`\n",
        );
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        let callout = projection
            .widgets()
            .iter()
            .find_map(ProjectedWidget::callout)
            .expect("callout payload");

        assert_eq!(callout.body(), "bold link code");
    }

    #[test]
    fn callout_source_reveals_when_caret_enters_callout_body() {
        let source = "> [!note] Title\n> Body text.\n";
        let marker = source.find("Body").unwrap();
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::Callout)
        );
    }

    #[test]
    fn callout_source_reveal_restores_nested_markers_and_widgets() {
        let source = concat!("> [!note] Title\n", "> - [ ] **Task**\n",);
        let marker = source.find("Title").unwrap();
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(
            projection.text(),
            source,
            "activating the callout must reveal nested delimiter, quote, list, and task markers across the owned source range",
        );
        assert!(projection.widgets().iter().all(|widget| {
            !matches!(
                widget.kind(),
                WidgetKind::Callout | WidgetKind::TaskCheckbox
            )
        }));
    }

    #[test]
    fn inactive_obsidian_embed_becomes_custom_embed_widget() {
        let source = concat!(
            "before\n\n",
            "![[diagram.svg#Overview|Architecture]]\n",
            "\nafter\n",
        );
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        let widget = projection
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::CustomEmbed)
            .expect("custom embed widget");
        let embed = widget.custom_embed().expect("embed payload");

        assert_eq!(embed.target(), "diagram.svg#Overview");
        assert_eq!(embed.path(), "diagram.svg");
        assert_eq!(embed.fragment(), Some("Overview"));
        assert_eq!(embed.display(), Some("Architecture"));
        assert_eq!(
            &source[widget.source_range().as_usize_range()],
            "![[diagram.svg#Overview|Architecture]]"
        );
        assert!(widget.projected_range().is_empty());
        assert!(!projection.text().contains("diagram.svg"));
    }

    #[test]
    fn inline_obsidian_embed_remains_source_visible_without_inline_width_reservation() {
        let source = "before ![[diagram.svg|Architecture]] after";
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::CustomEmbed)
        );
    }

    #[test]
    fn custom_embed_source_reveals_when_caret_enters_embed_range() {
        let source = "prefix ![[Note#Heading]] suffix";
        let marker = source.find("Note").unwrap();
        let projection = build_with_obsidian_extensions(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::CustomEmbed)
        );
    }

    #[test]
    fn inactive_horizontal_rule_becomes_projected_widget_without_removing_line_break() {
        let source = "above\n\n---\n\nbelow\n";
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        assert!(!projection.text().contains("---"));
        assert_eq!(
            projection.text().matches('\n').count(),
            source.matches('\n').count()
        );
        assert_eq!(projection.widgets().len(), 1);
        let widget = &projection.widgets()[0];
        assert_eq!(widget.kind(), WidgetKind::HorizontalRule);
        assert!(widget.projected_range().is_empty());
    }

    #[test]
    fn horizontal_rule_reveals_source_when_caret_enters_widget_range() {
        let source = "above\n\n---\n\nbelow\n";
        let marker = source.find("---").unwrap();
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker + 1).unwrap()),
        );

        assert!(projection.text().contains("---"));
        assert!(projection.widgets().is_empty());
    }

    #[test]
    fn inactive_task_markers_become_checkbox_widgets_with_toggle_actions() {
        let source = "- [ ] todo\n- [x] done\n";
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(source.len()).unwrap()),
        );

        assert_eq!(projection.widgets().len(), 2);
        assert_eq!(projection.widgets()[0].kind(), WidgetKind::TaskCheckbox);
        assert_eq!(projection.widgets()[1].kind(), WidgetKind::TaskCheckbox);
        assert!(projection.widgets().iter().all(|widget| {
            widget.projected_range().is_empty()
                && matches!(
                    &source[widget.source_range().as_usize_range()],
                    "[ ]" | "[x]"
                )
        }));

        assert_eq!(projection.widgets()[0].task_checked(), Some(false));
        assert_eq!(projection.widgets()[1].task_checked(), Some(true));
        assert_eq!(
            projection.widgets()[0].primary_action(),
            Some(WidgetAction::SetTaskChecked {
                marker_range: projection.widgets()[0].source_range(),
                checked: true,
            })
        );
        assert_eq!(
            projection.widgets()[0]
                .primary_action()
                .unwrap()
                .replacement_text(),
            "[x]"
        );
        assert_eq!(
            projection.widgets()[1]
                .primary_action()
                .unwrap()
                .replacement_text(),
            "[ ]"
        );
    }

    #[test]
    fn task_marker_source_is_revealed_when_caret_enters_it() {
        let source = "- [ ] todo\n- [x] done\n";
        let marker = source.find("[ ]").unwrap();
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker + 1).unwrap()),
        );

        assert!(projection.text().contains("[ ]"));
        assert_eq!(
            projection
                .widgets()
                .iter()
                .filter(|widget| widget.kind() == WidgetKind::TaskCheckbox)
                .count(),
            1
        );
        assert_eq!(
            projection
                .widgets()
                .iter()
                .find(|widget| widget.kind() == WidgetKind::TaskCheckbox)
                .unwrap()
                .kind(),
            WidgetKind::TaskCheckbox
        );
    }

    #[test]
    fn standalone_image_becomes_projected_widget_with_link_metadata() {
        let source = "before\n\n![blue square](fixtures/phase-06-image.png \"fixture\")\n\nafter\n";
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        let image = projection
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Image)
            .expect("standalone image widget");
        assert!(image.projected_range().is_empty());
        assert!(!projection.text().contains("![blue square]"));

        let data = image.image().expect("image payload");
        assert_eq!(data.destination(), "fixtures/phase-06-image.png");
        assert_eq!(data.title(), "fixture");
        assert_eq!(data.alt(), "blue square");
    }

    #[test]
    fn image_source_reveals_when_caret_enters_image_range() {
        let source = "![alt](image.png)\n";
        let marker = source.find("image.png").unwrap();
        let projection = build(
            source,
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::Image)
        );
    }

    #[test]
    fn inline_image_remains_source_visible_until_inline_reservation_exists() {
        let source = "prefix ![alt](image.png) suffix";
        let projection = build(source, RevealPolicy::ConcealInactive, &RevealContext::new());

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::Image)
        );
    }

    #[test]
    fn inactive_table_becomes_block_widget_with_alignment_and_cell_data() {
        let source = concat!(
            "before\n\n",
            "| Name | Score |\n",
            "| :--- | ---: |\n",
            "| Ada | 42 |\n",
            "| Bob | 7 |\n",
            "\nafter\n",
        );
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        let widget = projection
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Table)
            .expect("table widget");
        assert!(widget.projected_range().is_empty());
        assert!(!projection.text().contains("| Name | Score |"));

        let table = widget.table().expect("table payload");
        assert_eq!(
            table.alignments(),
            &[
                mdedit_markdown::SyntaxTableAlignment::Left,
                mdedit_markdown::SyntaxTableAlignment::Right,
            ]
        );
        assert_eq!(table.header(), &["Name", "Score"]);
        assert_eq!(table.rows(), &[vec!["Ada", "42"], vec!["Bob", "7"]]);
    }

    #[test]
    fn table_payload_uses_visible_cell_text_without_inline_markdown_markers() {
        let source = concat!(
            "| Name | Detail |\n",
            "| --- | --- |\n",
            "| **Ada** | [site](https://example.com) and `code` |\n",
        );
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        let table = projection
            .widgets()
            .iter()
            .find_map(ProjectedWidget::table)
            .expect("table payload");

        assert_eq!(table.header(), &["Name", "Detail"]);
        assert_eq!(table.rows(), &[vec!["Ada", "site and code"]]);
    }

    #[test]
    fn table_source_reveals_when_caret_enters_table_range() {
        let source = concat!(
            "before\n\n",
            "| Name | Score |\n",
            "| --- | ---: |\n",
            "| Ada | 42 |\n",
            "\nafter\n",
        );
        let marker = source.find("Ada").unwrap();
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::Table)
        );
    }

    #[test]
    fn inactive_display_math_becomes_block_widget_with_tex_payload() {
        let source = concat!(
            "before\n\n",
            "$",
            "$",
            "\\frac{a}{b} + \\sqrt{x}",
            "$",
            "$",
            "\n\nafter\n"
        );
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::ZERO),
        );

        let widget = projection
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::DisplayMath)
            .expect("display math widget");

        assert!(widget.projected_range().is_empty());
        assert_eq!(
            widget.display_math().expect("math payload").tex(),
            "\\frac{a}{b} + \\sqrt{x}"
        );
        assert!(!projection.text().contains("$"));
    }

    #[test]
    fn display_math_source_reveals_when_caret_enters_range() {
        let source = concat!("$", "$", "x^2 + y^2", "$", "$", "\n");
        let marker = source.find("x^2").unwrap();
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::DisplayMath)
        );
    }

    #[test]
    fn inline_math_stays_source_visible_until_inline_width_is_reserved() {
        let source = "prefix $x^2 + y^2$ suffix";
        let projection = build_with_dialect(
            source,
            MarkdownDialect::obsidian(),
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), source);
        assert!(
            projection
                .widgets()
                .iter()
                .all(|widget| widget.kind() != WidgetKind::DisplayMath)
        );
    }

    #[test]
    fn quoted_fenced_code_preserves_literal_quote_characters() {
        let source = "> ```\n> > literal\n> >> more\n> ```\n";
        let projection = build(source, RevealPolicy::ConcealInactive, &RevealContext::new());
        assert_eq!(projection.text(), "```\n> literal\n>> more\n```\n");

        let nested = "> > ```\r\n> > > literal\r\n> > ```\r\n";
        let projection = build(nested, RevealPolicy::ConcealInactive, &RevealContext::new());
        assert_eq!(projection.text(), "```\r\n> literal\r\n```\r\n");
    }

    #[test]
    fn unsupported_wikilinks_stay_source_visible_next_to_supported_markup() {
        let source = "[[Note]] and **bold** and [[Note|Alias]]";
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::obsidian());
        let blocks = BlockCache::new().reconcile(&snapshot, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&snapshot, &syntax).unwrap();
        let projection = Projection::build(
            &snapshot,
            &syntax,
            &blocks,
            &delimiters,
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        )
        .unwrap();
        assert_eq!(projection.text(), "[[Note]] and bold and [[Note|Alias]]");
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
                && style.source_range().as_usize_range() == (0..8)
                && style.projected_range().as_usize_range() == (0..8)
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
            style.kind() == StyleKind::Strong && style.projected_range().as_usize_range() == (0..4)
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
    fn nested_caret_reveals_only_the_most_specific_construct() {
        let context = RevealContext::new().with_caret(TextSize::new(11));
        let projection = build(
            "**outer *inner* tail**",
            RevealPolicy::ConcealInactive,
            &context,
        );

        assert_eq!(projection.text(), "outer *inner* tail");
        assert!(
            projection
                .reveal_groups()
                .iter()
                .any(|group| group.owner() == SyntaxKind::Emphasis && group.revealed())
        );
        assert!(
            projection
                .reveal_groups()
                .iter()
                .any(|group| group.owner() == SyntaxKind::Strong && !group.revealed())
        );
    }

    #[test]
    fn outer_only_caret_does_not_reveal_nested_child() {
        let context = RevealContext::new().with_caret(TextSize::new(4));
        let projection = build(
            "**outer *inner* tail**",
            RevealPolicy::ConcealInactive,
            &context,
        );

        assert_eq!(projection.text(), "**outer inner tail**");
        assert!(
            projection
                .reveal_groups()
                .iter()
                .any(|group| group.owner() == SyntaxKind::Strong && group.revealed())
        );
        assert!(
            projection
                .reveal_groups()
                .iter()
                .any(|group| group.owner() == SyntaxKind::Emphasis && !group.revealed())
        );
    }

    #[test]
    fn compact_combined_delimiters_reveal_only_the_innermost_construct() {
        let context = RevealContext::new().with_caret(TextSize::new(3));
        let projection = build("***x***", RevealPolicy::ConcealInactive, &context);

        assert_eq!(projection.text(), "**x**");
        let strong = projection
            .reveal_groups()
            .iter()
            .find(|group| group.owner() == SyntaxKind::Strong)
            .unwrap();
        let emphasis = projection
            .reveal_groups()
            .iter()
            .find(|group| group.owner() == SyntaxKind::Emphasis)
            .unwrap();
        assert!(strong.revealed());
        assert!(!emphasis.revealed());
    }

    #[test]
    fn caret_on_outer_compact_delimiter_reveals_only_the_outer_construct() {
        let context = RevealContext::new().with_caret(TextSize::ZERO);
        let projection = build("***x***", RevealPolicy::ConcealInactive, &context);

        assert_eq!(projection.text(), "*x*");
        let strong = projection
            .reveal_groups()
            .iter()
            .find(|group| group.owner() == SyntaxKind::Strong)
            .unwrap();
        let emphasis = projection
            .reveal_groups()
            .iter()
            .find(|group| group.owner() == SyntaxKind::Emphasis)
            .unwrap();
        assert!(!strong.revealed());
        assert!(emphasis.revealed());
    }

    #[test]
    fn selection_ending_at_adjacent_construct_does_not_reveal_it() {
        let selection = TextRange::new(TextSize::new(1), TextSize::new(6)).unwrap();
        let projection = build(
            "*one* **two**",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_selection(selection),
        );

        assert_eq!(projection.text(), "*one* two");
        let strong = projection
            .reveal_groups()
            .iter()
            .find(|group| group.owner() == SyntaxKind::Strong)
            .unwrap();
        assert!(!strong.revealed());
    }

    #[test]
    fn selection_fully_inside_nested_child_shadows_parent_reveal() {
        let selection = TextRange::new(TextSize::new(10), TextSize::new(13)).unwrap();
        let projection = build(
            "**outer *inner* tail**",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_selection(selection),
        );

        assert_eq!(projection.text(), "outer *inner* tail");
    }

    #[test]
    fn selection_crossing_child_boundary_reveals_parent_and_child() {
        let selection = TextRange::new(TextSize::new(6), TextSize::new(18)).unwrap();
        let projection = build(
            "**outer *inner* tail**",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_selection(selection),
        );

        assert_eq!(projection.text(), "**outer *inner* tail**");
    }

    #[test]
    fn zero_length_composition_uses_point_specificity() {
        let composition = TextRange::new(TextSize::new(11), TextSize::new(11)).unwrap();
        let projection = build(
            "**outer *inner* tail**",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_composition(composition),
        );

        assert_eq!(projection.text(), "outer *inner* tail");
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
    fn structural_projection_collapses_heading_marker_and_separator_separately() {
        let projection = build(
            "# Heading\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "Heading\n");
        assert_eq!(projection.concealed_spans().len(), 1);
        assert_eq!(
            projection.concealed_spans()[0].kind(),
            DelimiterKind::HeadingPrefix
        );
        assert_eq!(projection.structural_padding_spans().len(), 1);
        assert_eq!(
            projection.structural_padding_spans()[0].kind(),
            StructuralPaddingKind::HeadingSeparator
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::ZERO, ProjectionBias::After),
            Some(TextSize::new(2))
        );
    }

    #[test]
    fn heading_indent_and_closing_marker_padding_collapse_without_joining_syntax_ownership() {
        let projection = build(
            "  ###   Heading   ###\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "Heading\n");
        assert!(
            projection
                .structural_padding_spans()
                .iter()
                .any(|span| span.kind() == StructuralPaddingKind::HeadingIndent)
        );
        assert!(
            projection
                .structural_padding_spans()
                .iter()
                .any(|span| span.kind() == StructuralPaddingKind::HeadingSeparator)
        );
        assert!(
            projection
                .structural_padding_spans()
                .iter()
                .any(|span| span.kind() == StructuralPaddingKind::HeadingClosingSeparator)
        );
    }

    #[test]
    fn block_quote_and_list_separators_collapse_but_reveal_with_their_owner() {
        let quote = build(
            "> one\n> > two\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );
        assert_eq!(quote.text(), "one\ntwo\n");
        assert!(
            quote
                .structural_padding_spans()
                .iter()
                .all(|span| span.kind() == StructuralPaddingKind::BlockQuoteSeparator)
        );

        let list = build(
            "-   one\n10. two\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );
        assert_eq!(list.text(), "one\ntwo\n");

        let revealed = build(
            "-   one\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new().with_caret(TextSize::new(5)),
        );
        assert_eq!(revealed.text(), "-   one\n");
        assert!(revealed.structural_padding_spans().is_empty());
    }

    #[test]
    fn list_padding_preserves_code_indentation_when_more_than_four_spaces_follow_marker() {
        let projection = build(
            "-     code\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "    code\n");
        assert_eq!(
            projection.structural_padding_spans()[0]
                .source_range()
                .len(),
            TextSize::new(1)
        );
    }

    #[test]
    fn setext_marker_line_collapses_without_leaving_a_blank_projection_line() {
        let projection = build(
            "Heading\n  ===  \nNext\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "Heading\nNext\n");
        assert!(
            projection
                .structural_padding_spans()
                .iter()
                .any(|span| span.kind() == StructuralPaddingKind::SetextIndent)
        );
        assert!(
            projection
                .structural_padding_spans()
                .iter()
                .any(|span| span.kind() == StructuralPaddingKind::SetextLineEnding)
        );
    }

    #[test]
    fn structural_padding_mapping_stays_on_utf8_boundaries() {
        let projection = build(
            "# 한글\n",
            RevealPolicy::ConcealInactive,
            &RevealContext::new(),
        );

        assert_eq!(projection.text(), "한글\n");
        assert_eq!(
            projection.map().source_to_projected(TextSize::new(2)),
            Some(ProjectedSize::ZERO)
        );
        assert_eq!(
            projection
                .map()
                .projected_to_source(ProjectedSize::ZERO, ProjectionBias::After),
            Some(TextSize::new(2))
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
