use mdedit_core::{Revision, TextRange, TextSize};

use crate::SyntaxMetadata;

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
    metadata: SyntaxMetadata,
    children: Vec<SyntaxNode>,
}

impl SyntaxNode {
    #[must_use]
    pub const fn new(kind: SyntaxKind, range: TextRange) -> Self {
        Self {
            kind,
            range,
            metadata: SyntaxMetadata::None,
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
    pub const fn metadata(&self) -> &SyntaxMetadata {
        &self.metadata
    }

    #[must_use]
    pub fn children(&self) -> &[SyntaxNode] {
        &self.children
    }

    #[cfg(feature = "parse-profiling")]
    pub(crate) fn child_capacity(&self) -> usize {
        self.children.capacity()
    }

    pub(crate) fn with_metadata(mut self, metadata: SyntaxMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub(crate) fn push_child(&mut self, child: SyntaxNode) {
        if self.children.capacity() == 0 {
            // Most inline and leaf-block containers have one child. Keep wide
            // containers on Vec's geometric growth after the initial hint.
            let first_capacity = match self.kind {
                SyntaxKind::Heading(_)
                | SyntaxKind::BlockQuote
                | SyntaxKind::CodeBlock
                | SyntaxKind::TableCell
                | SyntaxKind::Emphasis
                | SyntaxKind::Strong
                | SyntaxKind::Strikethrough
                | SyntaxKind::Superscript
                | SyntaxKind::Subscript
                | SyntaxKind::Link
                | SyntaxKind::Image => 1,
                SyntaxKind::List { .. }
                | SyntaxKind::Table
                | SyntaxKind::TableHead
                | SyntaxKind::TableRow => 2,
                _ => 0,
            };
            self.children.reserve_exact(first_capacity);
        }
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

#[cfg(test)]
mod storage_tests {
    use super::*;
    fn range() -> TextRange {
        TextRange::new(TextSize::ZERO, TextSize::new(10)).unwrap()
    }
    #[test]
    fn narrow_container_storage_is_bounded_and_const_construction_stays_available() {
        const EMPTY: SyntaxNode =
            SyntaxNode::new(SyntaxKind::Document, TextRange::empty(TextSize::ZERO));
        assert!(EMPTY.children().is_empty());
        let kinds = [
            SyntaxKind::Heading(2),
            SyntaxKind::Strong,
            SyntaxKind::Emphasis,
            SyntaxKind::BlockQuote,
            SyntaxKind::CodeBlock,
            SyntaxKind::TableCell,
        ];
        let mut allocated = 0;
        for index in 0..512 {
            let mut parent = SyntaxNode::new(kinds[index % kinds.len()], range());
            parent.push_child(SyntaxNode::new(SyntaxKind::Text, range()));
            allocated += parent.children.capacity();
            assert_eq!(parent.children.len(), 1);
        }
        assert!(
            allocated <= 1024,
            "512 single-child containers allocated {allocated} slots"
        );
    }
    #[test]
    fn child_hints_preserve_wide_and_mixed_growth_order() {
        for kind in [
            SyntaxKind::TableRow,
            SyntaxKind::Paragraph,
            SyntaxKind::Heading(1),
            SyntaxKind::Strong,
        ] {
            let mut node = SyntaxNode::new(kind, range());
            for index in 0..257 {
                node.push_child(SyntaxNode::new(
                    SyntaxKind::Heading((index % 6 + 1) as u8),
                    range(),
                ));
            }
            assert_eq!(node.children.len(), 257);
            for (index, child) in node.children().iter().enumerate() {
                assert_eq!(child.kind(), SyntaxKind::Heading((index % 6 + 1) as u8));
            }
            assert!(node.children.capacity() < 1024);
        }
    }
}
