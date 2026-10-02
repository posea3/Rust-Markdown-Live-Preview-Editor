use mdedit_core::Document;
use mdedit_markdown::{
    obsidian_extension_set, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    SyntaxBlockQuoteKind, SyntaxCodeBlockKind, SyntaxKind, SyntaxLinkType, SyntaxMetadata,
    SyntaxMetadataBlockKind, SyntaxNode, SyntaxTableAlignment, OBSIDIAN_BLOCK_ID,
    OBSIDIAN_CALLOUT, OBSIDIAN_COMMENT, OBSIDIAN_EMBED, OBSIDIAN_HIGHLIGHT,
    OBSIDIAN_WIKILINK,
};

const COMMONMARK: &str = include_str!("fixtures/commonmark.md");
const GFM: &str = include_str!("fixtures/gfm.md");
const OBSIDIAN: &str = include_str!("fixtures/obsidian.md");

fn parse(source: &str, dialect: MarkdownDialect) -> mdedit_markdown::SyntaxSnapshot {
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

fn collect<'a>(
    node: &'a SyntaxNode,
    predicate: &impl Fn(SyntaxKind) -> bool,
    output: &mut Vec<&'a SyntaxNode>,
) {
    if predicate(node.kind()) {
        output.push(node);
    }
    for child in node.children() {
        collect(child, predicate, output);
    }
}

#[test]
fn commonmark_fixture_preserves_semantic_metadata() {
    let snapshot = parse(COMMONMARK, MarkdownDialect::commonmark());

    let list = find_first(snapshot.root(), &|kind| {
        kind == SyntaxKind::List { ordered: true }
    })
    .expect("ordered list");
    assert!(matches!(
        list.metadata(),
        SyntaxMetadata::List { start: Some(7) }
    ));

    let code =
        find_first(snapshot.root(), &|kind| kind == SyntaxKind::CodeBlock).expect("code block");
    assert!(matches!(
        code.metadata(),
        SyntaxMetadata::CodeBlock {
            kind: SyntaxCodeBlockKind::Fenced { info }
        } if info == "rust"
    ));

    let mut links = Vec::new();
    collect(
        snapshot.root(),
        &|kind| kind == SyntaxKind::Link,
        &mut links,
    );
    assert!(links.iter().any(|node| {
        matches!(
            node.metadata(),
            SyntaxMetadata::Link(metadata)
                if metadata.kind() == SyntaxLinkType::Inline
                    && metadata.destination() == "https://example.com"
                    && metadata.title() == "Example"
        )
    }));
    assert!(links.iter().any(|node| {
        matches!(
            node.metadata(),
            SyntaxMetadata::Link(metadata)
                if metadata.kind() == SyntaxLinkType::Reference
                    && metadata.destination() == "/reference"
                    && metadata.title() == "Reference title"
                    && metadata.reference().eq_ignore_ascii_case("target")
        )
    }));
}

#[test]
fn gfm_fixture_preserves_table_tasks_strike_and_alert_metadata() {
    let snapshot = parse(GFM, MarkdownDialect::gfm());

    let table = find_first(snapshot.root(), &|kind| kind == SyntaxKind::Table).expect("table");
    assert!(matches!(
        table.metadata(),
        SyntaxMetadata::Table { alignments }
            if alignments == &[
                SyntaxTableAlignment::Left,
                SyntaxTableAlignment::Center,
                SyntaxTableAlignment::Right,
            ]
    ));

    let mut task_markers = Vec::new();
    collect(
        snapshot.root(),
        &|kind| matches!(kind, SyntaxKind::TaskListMarker { .. }),
        &mut task_markers,
    );
    assert_eq!(task_markers.len(), 2);
    assert!(task_markers
        .iter()
        .any(|node| node.kind() == SyntaxKind::TaskListMarker { checked: true }));
    assert!(task_markers
        .iter()
        .any(|node| node.kind() == SyntaxKind::TaskListMarker { checked: false }));

    assert!(find_first(snapshot.root(), &|kind| kind == SyntaxKind::Strikethrough).is_some());

    let quote =
        find_first(snapshot.root(), &|kind| kind == SyntaxKind::BlockQuote).expect("alert");
    assert!(matches!(
        quote.metadata(),
        SyntaxMetadata::BlockQuote {
            kind: Some(SyntaxBlockQuoteKind::Note)
        }
    ));
}

#[test]
fn obsidian_fixture_combines_parser_metadata_with_opt_in_extensions() {
    let document = Document::new(OBSIDIAN).unwrap();
    let source = document.snapshot();
    let syntax = PulldownCmarkParser.parse(&source, &MarkdownDialect::obsidian());

    let metadata =
        find_first(syntax.root(), &|kind| kind == SyntaxKind::MetadataBlock).expect("frontmatter");
    assert!(matches!(
        metadata.metadata(),
        SyntaxMetadata::MetadataBlock {
            kind: SyntaxMetadataBlockKind::Yaml
        }
    ));

    let wiki_link = find_first(syntax.root(), &|kind| {
        kind == SyntaxKind::Link
            && matches!(
                kind,
                SyntaxKind::Link
            )
    })
    .expect("wikilink parser node");
    assert!(matches!(
        wiki_link.metadata(),
        SyntaxMetadata::Link(metadata)
            if matches!(
                metadata.kind(),
                SyntaxLinkType::WikiLink { has_alias: true }
            )
    ));

    let extensions = obsidian_extension_set().scan(&source, &syntax).unwrap();
    for expected in [
        OBSIDIAN_WIKILINK,
        OBSIDIAN_EMBED,
        OBSIDIAN_HIGHLIGHT,
        OBSIDIAN_COMMENT,
        OBSIDIAN_CALLOUT,
        OBSIDIAN_BLOCK_ID,
    ] {
        assert!(
            extensions
                .matches()
                .iter()
                .any(|item| item.kind().name() == expected),
            "missing Obsidian extension match: {expected}"
        );
    }

    let callout = extensions
        .matches()
        .iter()
        .find(|item| item.kind().name() == OBSIDIAN_CALLOUT)
        .expect("callout");
    assert_eq!(callout.attribute("type"), Some("warning"));
    assert_eq!(callout.attribute("fold"), Some("-"));
    assert_eq!(callout.attribute("title"), Some("Custom title"));
}

#[test]
fn extended_heading_attributes_are_project_owned_metadata() {
    let source = "# Heading {#custom .wide key=value}\n";
    let snapshot = parse(source, MarkdownDialect::extended());
    let heading =
        find_first(snapshot.root(), &|kind| kind == SyntaxKind::Heading(1)).expect("heading");

    assert!(matches!(
        heading.metadata(),
        SyntaxMetadata::Heading {
            id: Some(id),
            classes,
            attributes,
        } if id == "custom"
            && classes == &["wide".to_string()]
            && attributes.iter().any(|attribute| {
                attribute.name() == "key" && attribute.value() == Some("value")
            })
    ));
}
