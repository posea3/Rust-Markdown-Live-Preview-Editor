use mdedit_core::Anchor;

use crate::{ProjectedCaretStops, ProjectedSelectionEndpoint, Projection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReflowAnchor {
    before_source: Anchor,
    after_source: Anchor,
    before_projected: ProjectedSelectionEndpoint,
    after_projected: ProjectedSelectionEndpoint,
}

impl ReflowAnchor {
    #[must_use]
    pub fn same_source(
        before_projection: &Projection,
        before_stops: &ProjectedCaretStops,
        after_projection: &Projection,
        after_stops: &ProjectedCaretStops,
        source: Anchor,
    ) -> Option<Self> {
        Self::mapped_source(
            before_projection,
            before_stops,
            after_projection,
            after_stops,
            source,
            source,
        )
    }

    #[must_use]
    pub fn mapped_source(
        before_projection: &Projection,
        before_stops: &ProjectedCaretStops,
        after_projection: &Projection,
        after_stops: &ProjectedCaretStops,
        before_source: Anchor,
        after_source: Anchor,
    ) -> Option<Self> {
        Some(Self {
            before_source,
            after_source,
            before_projected: ProjectedSelectionEndpoint::from_source(
                before_projection,
                before_stops,
                before_source,
            )?,
            after_projected: ProjectedSelectionEndpoint::from_source(
                after_projection,
                after_stops,
                after_source,
            )?,
        })
    }

    #[must_use]
    pub const fn before_source(self) -> Anchor {
        self.before_source
    }

    #[must_use]
    pub const fn after_source(self) -> Anchor {
        self.after_source
    }

    #[must_use]
    pub const fn before_projected(self) -> ProjectedSelectionEndpoint {
        self.before_projected
    }

    #[must_use]
    pub const fn after_projected(self) -> ProjectedSelectionEndpoint {
        self.after_projected
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPosition {
    inline: f32,
    block: f32,
}

impl LayoutPosition {
    #[must_use]
    pub fn new(inline: f32, block: f32) -> Option<Self> {
        (inline.is_finite() && block.is_finite()).then_some(Self { inline, block })
    }

    #[must_use]
    pub const fn inline(self) -> f32 {
        self.inline
    }

    #[must_use]
    pub const fn block(self) -> f32 {
        self.block
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollAdjustment {
    inline: f32,
    block: f32,
}

impl ScrollAdjustment {
    pub const ZERO: Self = Self {
        inline: 0.0,
        block: 0.0,
    };

    #[must_use]
    pub fn between(before: LayoutPosition, after: LayoutPosition) -> Option<Self> {
        let inline = after.inline - before.inline;
        let block = after.block - before.block;
        (inline.is_finite() && block.is_finite()).then_some(Self { inline, block })
    }

    #[must_use]
    pub const fn inline(self) -> f32 {
        self.inline
    }

    #[must_use]
    pub const fn block(self) -> f32 {
        self.block
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflowMeasurement {
    anchor: ReflowAnchor,
    before: LayoutPosition,
    after: LayoutPosition,
}

impl ReflowMeasurement {
    #[must_use]
    pub const fn new(anchor: ReflowAnchor, before: LayoutPosition, after: LayoutPosition) -> Self {
        Self {
            anchor,
            before,
            after,
        }
    }

    #[must_use]
    pub const fn anchor(self) -> ReflowAnchor {
        self.anchor
    }

    #[must_use]
    pub const fn before(self) -> LayoutPosition {
        self.before
    }

    #[must_use]
    pub const fn after(self) -> LayoutPosition {
        self.after
    }

    #[must_use]
    pub fn scroll_adjustment(self) -> Option<ScrollAdjustment> {
        ScrollAdjustment::between(self.before, self.after)
    }
}

#[cfg(test)]
mod tests {
    use mdedit_core::{Affinity, Anchor, Document, TextSize};
    use mdedit_markdown::{
        BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    };

    use super::*;
    use crate::{ProjectedSize, RevealContext, RevealPolicy};

    fn projection(source: &str, context: &RevealContext) -> Projection {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::gfm());
        let mut block_cache = BlockCache::new();
        let blocks = block_cache.reconcile(&snapshot, &syntax).unwrap();
        let delimiters = DelimiterResolver.resolve(&snapshot, &syntax).unwrap();

        Projection::build(
            &snapshot,
            &syntax,
            &blocks,
            &delimiters,
            RevealPolicy::ConcealInactive,
            context,
        )
        .unwrap()
    }

    #[test]
    fn same_source_anchor_tracks_reveal_reflow() {
        let before = projection("**bold**", &RevealContext::new());
        let after = projection(
            "**bold**",
            &RevealContext::new().with_caret(TextSize::new(3)),
        );
        let before_stops = ProjectedCaretStops::from_projection(&before);
        let after_stops = ProjectedCaretStops::from_projection(&after);
        let source = Anchor::new(TextSize::new(3), Affinity::After);

        let anchor =
            ReflowAnchor::same_source(&before, &before_stops, &after, &after_stops, source)
                .unwrap();

        assert_eq!(anchor.before_projected().projected(), ProjectedSize::new(1));
        assert_eq!(anchor.after_projected().projected(), ProjectedSize::new(3));
        assert_eq!(anchor.before_source(), source);
        assert_eq!(anchor.after_source(), source);
    }

    #[test]
    fn collapsed_source_edge_survives_reveal_transition() {
        let before = projection("**bold**", &RevealContext::new());
        let after = projection(
            "**bold**",
            &RevealContext::new().with_caret(TextSize::new(2)),
        );
        let before_stops = ProjectedCaretStops::from_projection(&before);
        let after_stops = ProjectedCaretStops::from_projection(&after);
        let source = Anchor::new(TextSize::new(2), Affinity::Before);

        let anchor =
            ReflowAnchor::same_source(&before, &before_stops, &after, &after_stops, source)
                .unwrap();

        assert_eq!(anchor.before_projected().projected(), ProjectedSize::ZERO);
        assert_eq!(
            anchor.before_projected().to_source(&before_stops),
            Some(source)
        );
        assert_eq!(anchor.after_projected().projected(), ProjectedSize::new(2));
        assert_eq!(
            anchor.after_projected().to_source(&after_stops),
            Some(source)
        );
    }

    #[test]
    fn mapped_source_allows_change_map_to_remain_external() {
        let before = projection("abc", &RevealContext::new());
        let after = projection("xabc", &RevealContext::new());
        let before_stops = ProjectedCaretStops::from_projection(&before);
        let after_stops = ProjectedCaretStops::from_projection(&after);
        let before_source = Anchor::new(TextSize::new(1), Affinity::After);
        let after_source = Anchor::new(TextSize::new(2), Affinity::After);

        let anchor = ReflowAnchor::mapped_source(
            &before,
            &before_stops,
            &after,
            &after_stops,
            before_source,
            after_source,
        )
        .unwrap();

        assert_eq!(anchor.before_projected().projected(), ProjectedSize::new(1));
        assert_eq!(anchor.after_projected().projected(), ProjectedSize::new(2));
        assert_eq!(anchor.before_source(), before_source);
        assert_eq!(anchor.after_source(), after_source);
    }

    #[test]
    fn source_outside_projection_cannot_become_reflow_anchor() {
        let before = projection("abc", &RevealContext::new());
        let after = projection("abc", &RevealContext::new());
        let before_stops = ProjectedCaretStops::from_projection(&before);
        let after_stops = ProjectedCaretStops::from_projection(&after);

        assert_eq!(
            ReflowAnchor::same_source(
                &before,
                &before_stops,
                &after,
                &after_stops,
                Anchor::new(TextSize::new(4), Affinity::After),
            ),
            None
        );
    }

    #[test]
    fn layout_position_rejects_non_finite_measurements() {
        assert_eq!(LayoutPosition::new(f32::NAN, 0.0), None);
        assert_eq!(LayoutPosition::new(0.0, f32::INFINITY), None);
        assert_eq!(LayoutPosition::new(f32::NEG_INFINITY, 0.0), None);
    }

    #[test]
    fn scroll_adjustment_is_new_layout_position_minus_old_position() {
        let before = LayoutPosition::new(10.0, 100.0).unwrap();
        let after = LayoutPosition::new(12.0, 124.0).unwrap();
        let adjustment = ScrollAdjustment::between(before, after).unwrap();

        assert_eq!(adjustment.inline(), 2.0);
        assert_eq!(adjustment.block(), 24.0);
    }

    #[test]
    fn unchanged_anchor_geometry_needs_no_scroll_adjustment() {
        let position = LayoutPosition::new(12.0, 24.0).unwrap();

        assert_eq!(
            ScrollAdjustment::between(position, position),
            Some(ScrollAdjustment::ZERO)
        );
    }

    #[test]
    fn overflowing_measurement_delta_is_rejected() {
        let before = LayoutPosition::new(-f32::MAX, 0.0).unwrap();
        let after = LayoutPosition::new(f32::MAX, 0.0).unwrap();

        assert_eq!(ScrollAdjustment::between(before, after), None);
    }

    #[test]
    fn measurement_keeps_anchor_identity_with_scroll_request() {
        let before = projection("**bold**", &RevealContext::new());
        let after = projection(
            "**bold**",
            &RevealContext::new().with_caret(TextSize::new(3)),
        );
        let before_stops = ProjectedCaretStops::from_projection(&before);
        let after_stops = ProjectedCaretStops::from_projection(&after);
        let source = Anchor::new(TextSize::new(3), Affinity::After);
        let anchor =
            ReflowAnchor::same_source(&before, &before_stops, &after, &after_stops, source)
                .unwrap();
        let measurement = ReflowMeasurement::new(
            anchor,
            LayoutPosition::new(8.0, 40.0).unwrap(),
            LayoutPosition::new(8.0, 58.0).unwrap(),
        );

        assert_eq!(measurement.anchor(), anchor);
        assert_eq!(measurement.scroll_adjustment().unwrap().inline(), 0.0);
        assert_eq!(measurement.scroll_adjustment().unwrap().block(), 18.0);
    }
}
