use mdedit_core::{Revision, TextRange, TextSize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawFallbackReason {
    InvalidSourceRange,
    UnbalancedEvents,
    MismatchedEndTag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseStatus {
    Parsed,
    RawFallback(RawFallbackReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Document,
    RawSource,
    Paragraph,
    Heading(u8),
    BlockQuote,
    CodeBlock,
    HtmlBlock,
    List { ordered: bool },
    ListItem,
    FootnoteDefinition,
    DefinitionList,
    DefinitionListTitle,
    DefinitionListDefinition,
    Table,
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Superscript,
    Subscript,
    Link,
    Image,
    MetadataBlock,
    Text,
    InlineCode,
    InlineMath,
    DisplayMath,
    Html,
    InlineHtml,
    FootnoteReference,
    SoftBreak,
    HardBreak,
    Rule,
    TaskListMarker { checked: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxNode {
    kind: SyntaxKind,
    range: TextRange,
    children: Vec<SyntaxNode>,
}

impl SyntaxNode {
    #[must_use]
    pub const fn new(kind: SyntaxKind, range: TextRange) -> Self {
        Self {
            kind,
            range,
            children: Vec::new(),
        }
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
    pub fn children(&self) -> &[SyntaxNode] {
        &self.children
    }

    pub(crate) fn push_child(&mut self, child: SyntaxNode) {
        self.children.push(child);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxSnapshot {
    revision: Revision,
    source_len: TextSize,
    status: ParseStatus,
    root: SyntaxNode,
}

impl SyntaxSnapshot {
    #[must_use]
    pub(crate) const fn parsed(revision: Revision, source_len: TextSize, root: SyntaxNode) -> Self {
        Self {
            revision,
            source_len,
            status: ParseStatus::Parsed,
            root,
        }
    }

    #[must_use]
    pub(crate) fn raw_fallback(
        revision: Revision,
        source_len: TextSize,
        reason: RawFallbackReason,
    ) -> Self {
        let range = TextRange::new(TextSize::ZERO, source_len)
            .expect("zero-to-source-length range is ordered");
        let mut root = SyntaxNode::new(SyntaxKind::Document, range);
        root.push_child(SyntaxNode::new(SyntaxKind::RawSource, range));
        Self {
            revision,
            source_len,
            status: ParseStatus::RawFallback(reason),
            root,
        }
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
    pub const fn status(&self) -> ParseStatus {
        self.status
    }

    #[must_use]
    pub const fn root(&self) -> &SyntaxNode {
        &self.root
    }
}
