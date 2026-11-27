use mdedit_core::{Document, TextRange, TextSize};
use mdedit_live::{
    CaretDirection, ProjectedCaretStops, ProjectedSize, Projection, ProjectionBias, RevealContext,
    RevealPolicy,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    obsidian_extension_set,
};

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

fn project(source: &str, policy: RevealPolicy, context: &RevealContext) -> Projection {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
    let extensions = obsidian_extension_set();
    let metadata = extensions.scan(&document, &syntax).unwrap();
    let blocks = BlockCache::new().reconcile(&document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(&document, &syntax).unwrap();
    let projection = Projection::build_with_current_extensions(
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
    assert_eq!(document.text(), source);
    projection
}

#[test]
fn combining_cjk_emoji_crlf_family_and_flag_keep_exact_caret_edges() {
    let projection = project(
        "**a\u{301}한🙂**\r\n👨‍👩‍👧‍👦🇰🇷",
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    );
    assert_eq!(projection.text(), "a\u{301}한🙂\r\n👨‍👩‍👧‍👦🇰🇷");
    let stops = ProjectedCaretStops::from_projection(&projection);
    let actual = stops
        .stops()
        .iter()
        .map(|stop| {
            (
                stop.projected().get(),
                stop.source_before().get(),
                stop.source_after().get(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (0, 0, 2),
            (3, 5, 5),
            (6, 8, 8),
            (10, 12, 14),
            (12, 16, 16),
            (37, 41, 41),
            (45, 49, 49)
        ]
    );
    assert_eq!(
        stops.move_from_source(&projection, TextSize::new(6), CaretDirection::Backward),
        Some(TextSize::new(5))
    );
    assert_eq!(
        stops.move_from_source(&projection, TextSize::new(6), CaretDirection::Forward),
        Some(TextSize::new(8))
    );
}

#[test]
fn grapheme_joined_across_hidden_markup_skips_its_internal_map_boundary() {
    let projection = project(
        "a**\u{301}**b",
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    );
    assert_eq!(projection.text(), "a\u{301}b");
    let stops = ProjectedCaretStops::from_projection(&projection);
    assert_eq!(
        stops
            .stops()
            .iter()
            .map(|stop| (
                stop.projected().get(),
                stop.source_before().get(),
                stop.source_after().get()
            ))
            .collect::<Vec<_>>(),
        vec![(0, 0, 0), (3, 5, 7), (4, 8, 8)]
    );
    assert!(stops.stop_at(ProjectedSize::new(1)).is_none());
}

#[test]
fn real_reveal_and_widget_maps_match_retained_byte_counts_and_inverse_edges() {
    let sources = [
        "**a\u{301}한🙂**\r\n👨‍👩‍👧‍👦🇰🇷",
        "# title\r\n\r\n- [ ] 작업\r\n\r\n> [!note] a\r\n> **b**\r\n\r\n![[note]]\r\n\r\n| a | b |\r\n| - | - |\r\n| 1 | 2 |\r\n\r\n`**raw**`\r\n",
    ];
    let contexts = [
        RevealContext::new(),
        RevealContext::new().with_caret(TextSize::new(8)),
        RevealContext::new().with_selection(range(0, 8)),
        RevealContext::new().with_composition(range(2, 5)),
    ];
    for source in sources {
        for context in &contexts {
            for policy in [RevealPolicy::SourceVisible, RevealPolicy::ConcealInactive] {
                let projection = project(source, policy, context);
                let map = projection.map();
                // Independent retained-byte arithmetic, not the span index.
                let expected = (0..=source.len() as u32)
                    .map(|position| {
                        let removed: u32 = map
                            .concealed_ranges()
                            .iter()
                            .map(|range| {
                                position
                                    .min(range.end().get())
                                    .saturating_sub(range.start().get())
                            })
                            .sum();
                        position - removed
                    })
                    .collect::<Vec<_>>();
                for (source_position, projected) in expected.iter().enumerate() {
                    assert_eq!(
                        map.source_to_projected(TextSize::new(source_position as u32)),
                        Some(ProjectedSize::new(*projected))
                    );
                }
                for projected in 0..=projection.text().len() as u32 {
                    let before = expected
                        .iter()
                        .position(|value| *value == projected)
                        .unwrap() as u32;
                    let after = expected
                        .iter()
                        .rposition(|value| *value == projected)
                        .unwrap() as u32;
                    assert_eq!(
                        map.projected_to_source(
                            ProjectedSize::new(projected),
                            ProjectionBias::Before
                        ),
                        Some(TextSize::new(before))
                    );
                    assert_eq!(
                        map.projected_to_source(
                            ProjectedSize::new(projected),
                            ProjectionBias::After
                        ),
                        Some(TextSize::new(after))
                    );
                }
                for stop in ProjectedCaretStops::from_projection(&projection).stops() {
                    let projected = stop.projected().get();
                    assert_eq!(
                        stop.source_before().get(),
                        expected
                            .iter()
                            .position(|value| *value == projected)
                            .unwrap() as u32
                    );
                    assert_eq!(
                        stop.source_after().get(),
                        expected
                            .iter()
                            .rposition(|value| *value == projected)
                            .unwrap() as u32
                    );
                }
            }
        }
    }
}
