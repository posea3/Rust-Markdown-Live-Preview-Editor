use mdedit_core::Document;
use mdedit_markdown::{
    BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
};

use super::{ownership::*, ownership_reference, *};

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

#[test]
fn any_container_matches_all_small_range_subsets_and_extreme_endpoints() {
    let universe = (0..=4)
        .flat_map(|start| (start..=4).map(move |end| range(start, end)))
        .collect::<Vec<_>>();
    for mask in 0..1_u32 << universe.len() {
        let mut ranges = universe
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, range)| (mask & (1 << index) != 0).then_some(*range))
            .collect::<Vec<_>>();
        if let Some(first) = ranges.first().copied() {
            ranges.push(first);
        }
        let index = ContainmentSet::new(ranges.iter().copied());
        for start in 0..=5 {
            for end in start..=5 {
                let query = range(start, end);
                assert_eq!(
                    index.contains(query),
                    ranges.iter().any(|r| range_contains(*r, query)),
                    "mask={mask}, query={query:?}"
                );
            }
        }
    }
    let ranges = [range(0, 0), range(3, u32::MAX), range(u32::MAX, u32::MAX)];
    let index = ContainmentSet::new(ranges);
    for query in [
        range(0, 0),
        range(0, 1),
        range(1, 2),
        range(2, 3),
        range(3, u32::MAX),
        range(u32::MAX, u32::MAX),
    ] {
        assert_eq!(
            index.contains(query),
            ranges.iter().any(|r| range_contains(*r, query))
        );
    }
}

#[test]
fn owner_lookup_matches_brute_force_with_unsorted_nested_tied_and_clipped_nodes() {
    let specs = [
        (SyntaxKind::Strong, range(8, 12), range(8, 12)),
        (SyntaxKind::Strong, range(0, 20), range(3, 18)),
        (SyntaxKind::Strong, range(2, 6), range(2, 6)),
        (SyntaxKind::Strong, range(1, 5), range(1, 5)),
        (SyntaxKind::Emphasis, range(0, u32::MAX), range(0, u32::MAX)),
        (SyntaxKind::Strong, range(2, 6), range(2, 6)),
        (
            SyntaxKind::Strong,
            range(u32::MAX, u32::MAX),
            range(u32::MAX, u32::MAX),
        ),
        (SyntaxKind::BlockQuote, range(0, 0), range(0, 0)),
    ];
    let nodes = specs
        .iter()
        .map(|(kind, source, _)| SyntaxNode::new(*kind, *source))
        .collect::<Vec<_>>();
    let entries = nodes
        .iter()
        .zip(&specs)
        .map(|(node, (_, _, bounds))| (node, *bounds))
        .collect();
    let index = OwnerLookup::from_entries(entries);
    let mut points = (0..=21).collect::<Vec<_>>();
    points.extend([u32::MAX - 1, u32::MAX]);
    for kind in [
        SyntaxKind::Strong,
        SyntaxKind::Emphasis,
        SyntaxKind::BlockQuote,
        SyntaxKind::Link,
    ] {
        for (start_index, start) in points.iter().enumerate() {
            for end in &points[start_index..] {
                let query = range(*start, *end);
                let expected = nodes
                    .iter()
                    .zip(&specs)
                    .enumerate()
                    .filter(|(_, (node, (_, _, bounds)))| {
                        node.kind() == kind && range_contains(*bounds, query)
                    })
                    .min_by_key(|(order, (node, _))| (node.range().len(), *order))
                    .map(|(_, (node, _))| node);
                assert_eq!(
                    index
                        .smallest_containing(kind, query)
                        .map(std::ptr::from_ref),
                    expected.map(std::ptr::from_ref),
                    "kind={kind:?}, query={query:?}"
                );
            }
        }
    }
    assert!(
        OwnerLookup::from_entries(Vec::new())
            .smallest_containing(SyntaxKind::Strong, range(0, 0))
            .is_none()
    );
}

#[test]
fn broad_owner_queries_have_logarithmically_bounded_visits() {
    let nodes = (0..4096)
        .map(|index| SyntaxNode::new(SyntaxKind::Strong, range(index * 4, index * 4 + 3)))
        .collect::<Vec<_>>();
    let lookup = OwnerLookup::from_entries(nodes.iter().map(|node| (node, node.range())).collect());
    for (position, node) in nodes.iter().enumerate() {
        let point = position as u32 * 4 + 1;
        let (actual, visits) =
            lookup.query_with_visits(SyntaxKind::Strong, range(point, point + 1));
        assert!(std::ptr::eq(actual.unwrap(), node));
        assert!(visits <= 52, "broad query visited {visits} nodes");
    }
}

#[test]
fn parsed_owners_and_complete_projections_match_frozen_parent_reference() {
    let sources = [
        "",
        "plain 한🙂\r\n",
        "***a\u{301}*** and **b *c* d**\r\n\r\n# title ##\r\n",
        "> [!note] Outer\r\n> - [ ] Task **bold**\r\n> > [!warning] Inner\r\n> > body *em*\r\n\r\n![[note#part|name]]\r\n",
        "| a | b |\r\n| - | - |\r\n| **x** | 한🙂 |\r\n\r\n![a](image.png)\r\n\r\n$$\r\nx^2\r\n$$\r\n\r\n---\r\n",
        "head\r\n====\r\n\r\n1. [x] task\r\n\r\n```md\r\n**raw**\r\n```\r\n\r\n[**label**](https://example.com)\r\n",
        "> [!note] a\n> > **b**\n> [!tip] same owner\n> c\n\n> d\n\n- **x**\n- *y*\n",
    ];
    for (source_index, source) in sources.into_iter().enumerate() {
        let document = Document::new(source).unwrap().snapshot();
        let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
        let blocks = BlockCache::new().reconcile(&document, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&document, &syntax).unwrap();
        let extensions = mdedit_markdown::obsidian_extension_set();
        let metadata = extensions.scan(&document, &syntax).unwrap();
        let lookup = OwnerLookup::new(syntax.root());
        let mut points = vec![0, source.len() as u32];
        let mut kinds = Vec::new();
        fn collect(node: &SyntaxNode, points: &mut Vec<u32>, kinds: &mut Vec<SyntaxKind>) {
            points.extend([node.range().start().get(), node.range().end().get()]);
            if !kinds.contains(&node.kind()) {
                kinds.push(node.kind());
            }
            for child in node.children() {
                collect(child, points, kinds);
            }
        }
        collect(syntax.root(), &mut points, &mut kinds);
        points.sort_unstable();
        points.dedup();
        for kind in kinds {
            for (index, start) in points.iter().enumerate() {
                for end in &points[index..] {
                    let query = range(*start, *end);
                    assert_eq!(
                        lookup
                            .smallest_containing(kind, query)
                            .map(SyntaxNode::range),
                        ownership_reference::smallest_owner_range(syntax.root(), kind, query)
                    );
                }
            }
        }
        let contexts = [
            RevealContext::new(),
            RevealContext::new().with_caret(TextSize::ZERO),
            RevealContext::new().with_caret(TextSize::new(source.len() as u32 / 2)),
            RevealContext::new().with_selection(range(0, source.len() as u32)),
            RevealContext::new().with_composition(range(0, source.len() as u32)),
        ];
        for policy in [RevealPolicy::ConcealInactive, RevealPolicy::SourceVisible] {
            for (context_index, context) in contexts.iter().enumerate() {
                let actual = Projection::build_with_current_extensions(
                    &document,
                    &syntax,
                    &blocks,
                    &delimiters,
                    &metadata,
                    &extensions,
                    policy,
                    context,
                )
                .unwrap();
                let expected = ownership_reference::build_reference(
                    &document,
                    &syntax,
                    &blocks,
                    &delimiters,
                    Some(&metadata),
                    policy,
                    context,
                )
                .unwrap();
                assert_eq!(
                    actual, expected,
                    "source={source:?}, policy={policy:?}, context={context:?}"
                );
                let expected_status = if matches!(source_index, 3 | 6)
                    && context_index == 0
                    && policy == RevealPolicy::ConcealInactive
                {
                    ProjectionStatus::RawFallback(ProjectionFallbackReason::OverlappingConcealment)
                } else {
                    ProjectionStatus::Projected
                };
                assert_eq!(actual.status(), expected_status);
                assert_eq!(document.text(), source);
            }
        }
    }
}
