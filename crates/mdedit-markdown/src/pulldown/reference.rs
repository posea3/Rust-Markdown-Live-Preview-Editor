// Frozen parent933c16f parser; test-only.
use std::ops::Range;

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, LinkType, MetadataBlockKind,
    Options, Parser, Tag, TagEnd,
};

use crate::{
    MarkdownDialect, MarkdownParser, RawFallbackReason, SyntaxAttribute, SyntaxBlockQuoteKind,
    SyntaxCodeBlockKind, SyntaxKind, SyntaxLinkMetadata, SyntaxLinkType, SyntaxMetadata,
    SyntaxMetadataBlockKind, SyntaxNode, SyntaxSnapshot, SyntaxTableAlignment,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceParser;

impl MarkdownParser for ReferenceParser {
    fn parse(&self, snapshot: &DocumentSnapshot, dialect: &MarkdownDialect) -> SyntaxSnapshot {
        let source = snapshot.text();
        let parser = Parser::new_ext(&source, options_for(*dialect)).into_offset_iter();
        build_snapshot(snapshot.revision(), &source, parser)
    }
}

struct OpenNode {
    node: SyntaxNode,
    expected_end: Option<TagEnd>,
}

pub(super) fn build_snapshot<'a>(
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
                let expected_end = tag.to_end();
                stack.push(OpenNode {
                    node: node_from_tag(tag, range),
                    expected_end: Some(expected_end),
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
                stack
                    .last_mut()
                    .expect("document root remains")
                    .node
                    .push_child(node_from_event(leaf, range));
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

fn node_from_tag(tag: Tag<'_>, range: TextRange) -> SyntaxNode {
    match tag {
        Tag::Paragraph => SyntaxNode::new(SyntaxKind::Paragraph, range),
        Tag::Heading {
            level,
            id,
            classes,
            attrs,
        } => SyntaxNode::new(SyntaxKind::Heading(heading_level(level)), range).with_metadata(
            SyntaxMetadata::Heading {
                id: id.map(|value| value.to_string()),
                classes: classes.into_iter().map(|value| value.to_string()).collect(),
                attributes: attrs
                    .into_iter()
                    .map(|(name, value)| {
                        SyntaxAttribute::new(name.to_string(), value.map(|value| value.to_string()))
                    })
                    .collect(),
            },
        ),
        Tag::BlockQuote(kind) => SyntaxNode::new(SyntaxKind::BlockQuote, range).with_metadata(
            SyntaxMetadata::BlockQuote {
                kind: kind.map(block_quote_kind),
            },
        ),
        Tag::CodeBlock(kind) => {
            SyntaxNode::new(SyntaxKind::CodeBlock, range).with_metadata(SyntaxMetadata::CodeBlock {
                kind: match kind {
                    CodeBlockKind::Indented => SyntaxCodeBlockKind::Indented,
                    CodeBlockKind::Fenced(info) => SyntaxCodeBlockKind::Fenced {
                        info: info.to_string(),
                    },
                },
            })
        }
        Tag::HtmlBlock => SyntaxNode::new(SyntaxKind::HtmlBlock, range),
        Tag::List(start) => SyntaxNode::new(
            SyntaxKind::List {
                ordered: start.is_some(),
            },
            range,
        )
        .with_metadata(SyntaxMetadata::List { start }),
        Tag::Item => SyntaxNode::new(SyntaxKind::ListItem, range),
        Tag::FootnoteDefinition(label) => SyntaxNode::new(SyntaxKind::FootnoteDefinition, range)
            .with_metadata(SyntaxMetadata::FootnoteDefinition {
                label: label.to_string(),
            }),
        Tag::DefinitionList => SyntaxNode::new(SyntaxKind::DefinitionList, range),
        Tag::DefinitionListTitle => SyntaxNode::new(SyntaxKind::DefinitionListTitle, range),
        Tag::DefinitionListDefinition => {
            SyntaxNode::new(SyntaxKind::DefinitionListDefinition, range)
        }
        Tag::Table(alignments) => {
            SyntaxNode::new(SyntaxKind::Table, range).with_metadata(SyntaxMetadata::Table {
                alignments: alignments.into_iter().map(table_alignment).collect(),
            })
        }
        Tag::TableHead => SyntaxNode::new(SyntaxKind::TableHead, range),
        Tag::TableRow => SyntaxNode::new(SyntaxKind::TableRow, range),
        Tag::TableCell => SyntaxNode::new(SyntaxKind::TableCell, range),
        Tag::Emphasis => SyntaxNode::new(SyntaxKind::Emphasis, range),
        Tag::Strong => SyntaxNode::new(SyntaxKind::Strong, range),
        Tag::Strikethrough => SyntaxNode::new(SyntaxKind::Strikethrough, range),
        Tag::Superscript => SyntaxNode::new(SyntaxKind::Superscript, range),
        Tag::Subscript => SyntaxNode::new(SyntaxKind::Subscript, range),
        Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        } => SyntaxNode::new(SyntaxKind::Link, range).with_metadata(SyntaxMetadata::Link(
            link_metadata(link_type, dest_url.as_ref(), title.as_ref(), id.as_ref()),
        )),
        Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        } => SyntaxNode::new(SyntaxKind::Image, range).with_metadata(SyntaxMetadata::Image(
            link_metadata(link_type, dest_url.as_ref(), title.as_ref(), id.as_ref()),
        )),
        Tag::MetadataBlock(kind) => SyntaxNode::new(SyntaxKind::MetadataBlock, range)
            .with_metadata(SyntaxMetadata::MetadataBlock {
                kind: metadata_block_kind(kind),
            }),
    }
}

fn node_from_event(event: Event<'_>, range: TextRange) -> SyntaxNode {
    match event {
        Event::Text(_) => SyntaxNode::new(SyntaxKind::Text, range),
        Event::Code(content) => SyntaxNode::new(SyntaxKind::InlineCode, range).with_metadata(
            SyntaxMetadata::InlineCode {
                content: content.to_string(),
            },
        ),
        Event::InlineMath(content) => SyntaxNode::new(SyntaxKind::InlineMath, range).with_metadata(
            SyntaxMetadata::InlineMath {
                content: content.to_string(),
            },
        ),
        Event::DisplayMath(content) => SyntaxNode::new(SyntaxKind::DisplayMath, range)
            .with_metadata(SyntaxMetadata::DisplayMath {
                content: content.to_string(),
            }),
        Event::Html(_) => SyntaxNode::new(SyntaxKind::Html, range),
        Event::InlineHtml(_) => SyntaxNode::new(SyntaxKind::InlineHtml, range),
        Event::FootnoteReference(label) => SyntaxNode::new(SyntaxKind::FootnoteReference, range)
            .with_metadata(SyntaxMetadata::FootnoteReference {
                label: label.to_string(),
            }),
        Event::SoftBreak => SyntaxNode::new(SyntaxKind::SoftBreak, range),
        Event::HardBreak => SyntaxNode::new(SyntaxKind::HardBreak, range),
        Event::Rule => SyntaxNode::new(SyntaxKind::Rule, range),
        Event::TaskListMarker(checked) => {
            SyntaxNode::new(SyntaxKind::TaskListMarker { checked }, range)
        }
        Event::Start(_) | Event::End(_) => unreachable!("container events are handled separately"),
    }
}

fn link_metadata(
    link_type: LinkType,
    destination: &str,
    title: &str,
    reference: &str,
) -> SyntaxLinkMetadata {
    SyntaxLinkMetadata::new(
        match link_type {
            LinkType::Inline => SyntaxLinkType::Inline,
            LinkType::Reference => SyntaxLinkType::Reference,
            LinkType::ReferenceUnknown => SyntaxLinkType::ReferenceUnknown,
            LinkType::Collapsed => SyntaxLinkType::Collapsed,
            LinkType::CollapsedUnknown => SyntaxLinkType::CollapsedUnknown,
            LinkType::Shortcut => SyntaxLinkType::Shortcut,
            LinkType::ShortcutUnknown => SyntaxLinkType::ShortcutUnknown,
            LinkType::Autolink => SyntaxLinkType::Autolink,
            LinkType::Email => SyntaxLinkType::Email,
            LinkType::WikiLink { has_pothole } => SyntaxLinkType::WikiLink {
                has_alias: has_pothole,
            },
        },
        destination.to_owned(),
        title.to_owned(),
        reference.to_owned(),
    )
}

const fn block_quote_kind(kind: BlockQuoteKind) -> SyntaxBlockQuoteKind {
    match kind {
        BlockQuoteKind::Note => SyntaxBlockQuoteKind::Note,
        BlockQuoteKind::Tip => SyntaxBlockQuoteKind::Tip,
        BlockQuoteKind::Important => SyntaxBlockQuoteKind::Important,
        BlockQuoteKind::Warning => SyntaxBlockQuoteKind::Warning,
        BlockQuoteKind::Caution => SyntaxBlockQuoteKind::Caution,
    }
}

const fn table_alignment(alignment: Alignment) -> SyntaxTableAlignment {
    match alignment {
        Alignment::None => SyntaxTableAlignment::None,
        Alignment::Left => SyntaxTableAlignment::Left,
        Alignment::Center => SyntaxTableAlignment::Center,
        Alignment::Right => SyntaxTableAlignment::Right,
    }
}

const fn metadata_block_kind(kind: MetadataBlockKind) -> SyntaxMetadataBlockKind {
    match kind {
        MetadataBlockKind::YamlStyle => SyntaxMetadataBlockKind::Yaml,
        MetadataBlockKind::PlusesStyle => SyntaxMetadataBlockKind::Pluses,
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
