use std::ops::Range;

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    MarkdownDialect, MarkdownParser, ParseStatus, RawFallbackReason, SyntaxKind, SyntaxNode,
    SyntaxSnapshot,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct PulldownCmarkParser;

impl MarkdownParser for PulldownCmarkParser {
    fn parse(
        &self,
        snapshot: &DocumentSnapshot,
        dialect: &MarkdownDialect,
    ) -> SyntaxSnapshot {
        let source = snapshot.text();
        let parser = Parser::new_ext(&source, options_for(*dialect)).into_offset_iter();
        build_snapshot(snapshot.revision(), &source, parser)
    }
}

struct OpenNode {
    node: SyntaxNode,
    expected_end: Option<TagEnd>,
}

fn build_snapshot<'a>(
    revision: Revision,
    source: &'a str,
    events: impl IntoIterator<Item = (Event<'a>, Range<usize>)>,
) -> SyntaxSnapshot {
    let Ok(source_len) = TextSize::try_from_usize(source.len()) else {
        unreachable!("DocumentSnapshot already enforces the mdedit source-size limit");
    };
    let whole_range =
        TextRange::new(TextSize::ZERO, source_len).expect("zero-to-source-length range is ordered");
    let mut stack = vec![OpenNode {
        node: SyntaxNode::new(SyntaxKind::Document, whole_range),
        expected_end: None,
    }];

    for (event, byte_range) in events {
        let Some(range) = checked_range(source, byte_range) else {
            return SyntaxSnapshot::raw_fallback(
                revision,
                source_len,
                RawFallbackReason::InvalidSourceRange,
            );
        };

        match event {
            Event::Start(tag) => {
                let kind = tag_kind(&tag);
                stack.push(OpenNode {
                    node: SyntaxNode::new(kind, range),
                    expected_end: Some(tag.to_end()),
                });
            }
            Event::End(end) => {
                if stack.len() == 1 {
                    return SyntaxSnapshot::raw_fallback(
                        revision,
                        source_len,
                        RawFallbackReason::UnbalancedEvents,
                    );
                }

                let open = stack.pop().expect("non-root open node exists");
                if open.expected_end != Some(end) {
                    return SyntaxSnapshot::raw_fallback(
                        revision,
                        source_len,
                        RawFallbackReason::MismatchedEndTag,
                    );
                }
                stack
                    .last_mut()
                    .expect("document root remains")
                    .node
                    .push_child(open.node);
            }
            leaf => {
                let kind = leaf_kind(leaf);
                stack
                    .last_mut()
                    .expect("document root remains")
                    .node
                    .push_child(SyntaxNode::new(kind, range));
            }
        }
    }

    if stack.len() != 1 {
        return SyntaxSnapshot::raw_fallback(
            revision,
            source_len,
            RawFallbackReason::UnbalancedEvents,
        );
    }

    let root = stack.pop().expect("document root exists").node;
    SyntaxSnapshot::parsed(revision, source_len, root)
}

fn checked_range(source: &str, range: Range<usize>) -> Option<TextRange> {
    if range.start > range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return None;
    }

    let start = TextSize::try_from_usize(range.start).ok()?;
    let end = TextSize::try_from_usize(range.end).ok()?;
    TextRange::new(start, end).ok()
}

fn tag_kind(tag: &Tag<'_>) -> SyntaxKind {
    match tag {
        Tag::Paragraph => SyntaxKind::Paragraph,
        Tag::Heading { level, .. } => SyntaxKind::Heading(heading_level(*level)),
        Tag::BlockQuote(_) => SyntaxKind::BlockQuote,
        Tag::CodeBlock(_) => SyntaxKind::CodeBlock,
        Tag::HtmlBlock => SyntaxKind::HtmlBlock,
        Tag::List(start) => SyntaxKind::List {
            ordered: start.is_some(),
        },
        Tag::Item => SyntaxKind::ListItem,
        Tag::FootnoteDefinition(_) => SyntaxKind::FootnoteDefinition,
        Tag::DefinitionList => SyntaxKind::DefinitionList,
        Tag::DefinitionListTitle => SyntaxKind::DefinitionListTitle,
        Tag::DefinitionListDefinition => SyntaxKind::DefinitionListDefinition,
        Tag::Table(_) => SyntaxKind::Table,
        Tag::TableHead => SyntaxKind::TableHead,
        Tag::TableRow => SyntaxKind::TableRow,
        Tag::TableCell => SyntaxKind::TableCell,
        Tag::Emphasis => SyntaxKind::Emphasis,
        Tag::Strong => SyntaxKind::Strong,
        Tag::Strikethrough => SyntaxKind::Strikethrough,
        Tag::Superscript => SyntaxKind::Superscript,
        Tag::Subscript => SyntaxKind::Subscript,
        Tag::Link { .. } => SyntaxKind::Link,
        Tag::Image { .. } => SyntaxKind::Image,
        Tag::MetadataBlock(_) => SyntaxKind::MetadataBlock,
    }
}

fn leaf_kind(event: Event<'_>) -> SyntaxKind {
    match event {
        Event::Text(_) => SyntaxKind::Text,
        Event::Code(_) => SyntaxKind::InlineCode,
        Event::InlineMath(_) => SyntaxKind::InlineMath,
        Event::DisplayMath(_) => SyntaxKind::DisplayMath,
        Event::Html(_) => SyntaxKind::Html,
        Event::InlineHtml(_) => SyntaxKind::InlineHtml,
        Event::FootnoteReference(_) => SyntaxKind::FootnoteReference,
        Event::SoftBreak => SyntaxKind::SoftBreak,
        Event::HardBreak => SyntaxKind::HardBreak,
        Event::Rule => SyntaxKind::Rule,
        Event::TaskListMarker(checked) => SyntaxKind::TaskListMarker { checked },
        Event::Start(_) | Event::End(_) => unreachable!("container events are handled separately"),
    }
}

const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn options_for(dialect: MarkdownDialect) -> Options {
    let mut options = Options::empty();

    if dialect.tables() {
        options.insert(Options::ENABLE_TABLES);
    }
    if dialect.footnotes() {
        options.insert(Options::ENABLE_FOOTNOTES);
    }
    if dialect.strikethrough() {
        options.insert(Options::ENABLE_STRIKETHROUGH);
    }
    if dialect.task_lists() {
        options.insert(Options::ENABLE_TASKLISTS);
    }
    if dialect.gfm_extensions() {
        options.insert(Options::ENABLE_GFM);
    }
    if dialect.heading_attributes() {
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    }
    if dialect.math() {
        options.insert(Options::ENABLE_MATH);
    }
    if dialect.wikilinks() {
        options.insert(Options::ENABLE_WIKILINKS);
    }
    if dialect.definition_lists() {
        options.insert(Options::ENABLE_DEFINITION_LIST);
    }
    if dialect.metadata_blocks() {
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        options.insert(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);
    }
    if dialect.superscript() {
        options.insert(Options::ENABLE_SUPERSCRIPT);
    }
    if dialect.subscript() {
        options.insert(Options::ENABLE_SUBSCRIPT);
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdedit_core::Document;

    fn parse(source: &str, dialect: MarkdownDialect) -> SyntaxSnapshot {
        let document = Document::new(source).unwrap();
        PulldownCmarkParser.parse(&document.snapshot(), &dialect)
    }

    fn find_first(
        node: &SyntaxNode,
        predicate: &impl Fn(SyntaxKind) -> bool,
    ) -> Option<&SyntaxNode> {
        if predicate(node.kind()) {
            return Some(node);
        }
        node.children()
            .iter()
            .find_map(|child| find_first(child, predicate))
    }

    fn assert_ranges_are_valid(node: &SyntaxNode, source: &str) {
        let range = node.range().as_usize_range();
        assert!(range.start <= range.end);
        assert!(range.end <= source.len());
        assert!(source.is_char_boundary(range.start));
        assert!(source.is_char_boundary(range.end));
        for child in node.children() {
            assert_ranges_are_valid(child, source);
        }
    }

    #[test]
    fn commonmark_builds_nested_source_mapped_tree() {
        let source = "# Hello **world** and [site](https://example.com)\n";
        let snapshot = parse(source, MarkdownDialect::commonmark());

        assert_eq!(snapshot.status(), ParseStatus::Parsed);
        assert!(find_first(snapshot.root(), &|kind| kind == SyntaxKind::Heading(1)).is_some());
        assert!(find_first(snapshot.root(), &|kind| kind == SyntaxKind::Strong).is_some());

        let link =
            find_first(snapshot.root(), &|kind| kind == SyntaxKind::Link).expect("link node");
        assert_eq!(
            &source[link.range().as_usize_range()],
            "[site](https://example.com)"
        );
        assert_ranges_are_valid(snapshot.root(), source);
    }

    #[test]
    fn gfm_dialect_enables_tables_tasks_and_strikethrough() {
        let source =
            "| a | b |\n| --- | --- |\n| x | y |\n\n- [x] done\n\n~~gone~~\n";
        let snapshot = parse(source, MarkdownDialect::gfm());

        assert_eq!(snapshot.status(), ParseStatus::Parsed);
        assert!(find_first(snapshot.root(), &|kind| kind == SyntaxKind::Table).is_some());
        assert!(
            find_first(snapshot.root(), &|kind| {
                kind == SyntaxKind::TaskListMarker { checked: true }
            })
            .is_some()
        );
        assert!(find_first(snapshot.root(), &|kind| kind == SyntaxKind::Strikethrough).is_some());
    }

    #[test]
    fn malformed_markdown_remains_safe_and_source_mapped() {
        let source = "**unterminated [link](\n한글 👨‍👩‍👧‍👦";
        let snapshot = parse(source, MarkdownDialect::extended());

        assert_eq!(snapshot.status(), ParseStatus::Parsed);
        assert_ranges_are_valid(snapshot.root(), source);
    }

    #[test]
    fn invalid_adapter_range_falls_back_to_raw_source() {
        let source = "x";
        let snapshot = build_snapshot(
            Revision::initial(),
            source,
            [(Event::Text("x".into()), 0..2)],
        );

        assert_eq!(
            snapshot.status(),
            ParseStatus::RawFallback(RawFallbackReason::InvalidSourceRange)
        );
        assert_eq!(snapshot.root().children().len(), 1);
        assert_eq!(
            snapshot.root().children()[0].kind(),
            SyntaxKind::RawSource
        );
        assert_eq!(
            snapshot.root().children()[0].range().as_usize_range(),
            0..source.len()
        );
    }

    #[test]
    fn mismatched_end_event_falls_back_instead_of_panicking() {
        let source = "x";
        let snapshot = build_snapshot(
            Revision::initial(),
            source,
            [
                (Event::Start(Tag::Paragraph), 0..1),
                (Event::End(TagEnd::Strong), 0..1),
            ],
        );

        assert_eq!(
            snapshot.status(),
            ParseStatus::RawFallback(RawFallbackReason::MismatchedEndTag)
        );
    }
}
