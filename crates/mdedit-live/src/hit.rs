use mdedit_core::{Affinity, Anchor, SelectionRange, TextSize};

use crate::{
    ProjectedCaretStop, ProjectedCaretStops, ProjectedSize, Projection, ProjectionBias,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitBias {
    Before,
    After,
}

impl HitBias {
    #[must_use]
    pub const fn projection_bias(self) -> ProjectionBias {
        match self {
            Self::Before => ProjectionBias::Before,
            Self::After => ProjectionBias::After,
        }
    }

    #[must_use]
    pub const fn affinity(self) -> Affinity {
        match self {
            Self::Before => Affinity::Before,
            Self::After => Affinity::After,
        }
    }

    const fn from_affinity(affinity: Affinity) -> Self {
        match affinity {
            Affinity::Before => Self::Before,
            Affinity::After => Self::After,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedHit {
    requested: ProjectedSize,
    projected: ProjectedSize,
    source: TextSize,
    bias: HitBias,
}

impl ProjectedHit {
    #[must_use]
    pub const fn requested(self) -> ProjectedSize {
        self.requested
    }

    #[must_use]
    pub const fn projected(self) -> ProjectedSize {
        self.projected
    }

    #[must_use]
    pub const fn source(self) -> TextSize {
        self.source
    }

    #[must_use]
    pub const fn bias(self) -> HitBias {
        self.bias
    }
}

impl ProjectedCaretStops {
    #[must_use]
    pub fn hit_test(&self, projected: ProjectedSize, bias: HitBias) -> Option<ProjectedHit> {
        let last = self.stops().last()?;
        if projected > last.projected() {
            return None;
        }

        let stop = self.nearest_stop(projected, bias.projection_bias())?;
        Some(ProjectedHit {
            requested: projected,
            projected: stop.projected(),
            source: source_for_bias(stop, bias),
            bias,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedSelectionEndpoint {
    projected: ProjectedSize,
    bias: HitBias,
    affinity: Affinity,
}

impl ProjectedSelectionEndpoint {
    #[must_use]
    pub const fn projected(self) -> ProjectedSize {
        self.projected
    }

    #[must_use]
    pub const fn bias(self) -> HitBias {
        self.bias
    }

    #[must_use]
    pub const fn affinity(self) -> Affinity {
        self.affinity
    }

    #[must_use]
    pub fn from_hit(hit: ProjectedHit) -> Self {
        Self {
            projected: hit.projected,
            bias: hit.bias,
            affinity: hit.bias.affinity(),
        }
    }

    #[must_use]
    pub fn from_source(
        projection: &Projection,
        stops: &ProjectedCaretStops,
        anchor: Anchor,
    ) -> Option<Self> {
        let projected = projection.map().source_to_projected(anchor.offset)?;
        let bias = source_endpoint_bias(stops.stop_at(projected), anchor);
        let hit = stops.hit_test(projected, bias)?;

        Some(Self {
            projected: hit.projected,
            bias,
            affinity: anchor.affinity,
        })
    }

    #[must_use]
    pub fn to_source(self, stops: &ProjectedCaretStops) -> Option<Anchor> {
        let hit = stops.hit_test(self.projected, self.bias)?;
        Some(Anchor::new(hit.source, self.affinity))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedSelection {
    anchor: ProjectedSelectionEndpoint,
    focus: ProjectedSelectionEndpoint,
}

impl ProjectedSelection {
    #[must_use]
    pub const fn new(
        anchor: ProjectedSelectionEndpoint,
        focus: ProjectedSelectionEndpoint,
    ) -> Self {
        Self { anchor, focus }
    }

    #[must_use]
    pub const fn anchor(self) -> ProjectedSelectionEndpoint {
        self.anchor
    }

    #[must_use]
    pub const fn focus(self) -> ProjectedSelectionEndpoint {
        self.focus
    }

    #[must_use]
    pub fn from_source(
        projection: &Projection,
        stops: &ProjectedCaretStops,
        selection: SelectionRange,
    ) -> Option<Self> {
        Some(Self {
            anchor: ProjectedSelectionEndpoint::from_source(projection, stops, selection.anchor)?,
            focus: ProjectedSelectionEndpoint::from_source(projection, stops, selection.head)?,
        })
    }

    #[must_use]
    pub fn to_source(self, stops: &ProjectedCaretStops) -> Option<SelectionRange> {
        Some(SelectionRange {
            anchor: self.anchor.to_source(stops)?,
            head: self.focus.to_source(stops)?,
        })
    }
}

const fn source_for_bias(stop: ProjectedCaretStop, bias: HitBias) -> TextSize {
    match bias {
        HitBias::Before => stop.source_before(),
        HitBias::After => stop.source_after(),
    }
}

fn source_endpoint_bias(stop: Option<ProjectedCaretStop>, anchor: Anchor) -> HitBias {
    let Some(stop) = stop else {
        return HitBias::from_affinity(anchor.affinity);
    };

    if stop.is_collapsed_boundary() {
        if anchor.offset == stop.source_before() && anchor.offset != stop.source_after() {
            return HitBias::Before;
        }
        if anchor.offset == stop.source_after() && anchor.offset != stop.source_before() {
            return HitBias::After;
        }
    }

    HitBias::from_affinity(anchor.affinity)
}

#[cfg(test)]
mod tests {
    use mdedit_core::{Document, TextSize};
    use mdedit_markdown::{
        BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    };

    use super::*;
    use crate::{RevealContext, RevealPolicy};

    fn projection(source: &str, policy: RevealPolicy) -> Projection {
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
            policy,
            &RevealContext::new(),
        )
        .unwrap()
    }

    #[test]
    fn projected_hit_snaps_inside_grapheme_to_requested_side() {
        let projection = projection("한글", RevealPolicy::SourceVisible);
        let stops = ProjectedCaretStops::from_projection(&projection);

        let before = stops
            .hit_test(ProjectedSize::new(1), HitBias::Before)
            .unwrap();
        let after = stops
            .hit_test(ProjectedSize::new(1), HitBias::After)
            .unwrap();

        assert_eq!(before.requested(), ProjectedSize::new(1));
        assert_eq!(before.projected(), ProjectedSize::ZERO);
        assert_eq!(before.source(), TextSize::ZERO);
        assert_eq!(after.projected(), ProjectedSize::new(3));
        assert_eq!(after.source(), TextSize::new(3));
    }

    #[test]
    fn collapsed_boundary_hit_chooses_explicit_source_edge() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        let opening_before = stops.hit_test(ProjectedSize::ZERO, HitBias::Before).unwrap();
        let opening_after = stops.hit_test(ProjectedSize::ZERO, HitBias::After).unwrap();
        let closing_before = stops
            .hit_test(ProjectedSize::new(4), HitBias::Before)
            .unwrap();
        let closing_after = stops
            .hit_test(ProjectedSize::new(4), HitBias::After)
            .unwrap();

        assert_eq!(opening_before.source(), TextSize::ZERO);
        assert_eq!(opening_after.source(), TextSize::new(2));
        assert_eq!(closing_before.source(), TextSize::new(6));
        assert_eq!(closing_after.source(), TextSize::new(8));
    }

    #[test]
    fn hit_outside_projected_document_is_rejected() {
        let projection = projection("abc", RevealPolicy::SourceVisible);
        let stops = ProjectedCaretStops::from_projection(&projection);

        assert_eq!(
            stops.hit_test(ProjectedSize::new(4), HitBias::Before),
            None
        );
        assert_eq!(
            stops.hit_test(ProjectedSize::new(4), HitBias::After),
            None
        );
    }

    #[test]
    fn source_selection_round_trip_preserves_anchor_focus_direction() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);
        let forward = SelectionRange {
            anchor: Anchor::new(TextSize::new(2), Affinity::After),
            head: Anchor::new(TextSize::new(6), Affinity::Before),
        };
        let reversed = SelectionRange {
            anchor: forward.head,
            head: forward.anchor,
        };

        let projected_forward =
            ProjectedSelection::from_source(&projection, &stops, forward).unwrap();
        let projected_reversed =
            ProjectedSelection::from_source(&projection, &stops, reversed).unwrap();

        assert_eq!(
            projected_forward.anchor().projected(),
            ProjectedSize::ZERO
        );
        assert_eq!(
            projected_forward.focus().projected(),
            ProjectedSize::new(4)
        );
        assert_eq!(projected_forward.to_source(&stops), Some(forward));
        assert_eq!(projected_reversed.to_source(&stops), Some(reversed));
        assert!(projected_reversed.to_source(&stops).unwrap().is_reversed());
    }

    #[test]
    fn selection_direction_survives_when_both_endpoints_share_collapsed_boundary() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);
        let forward = SelectionRange {
            anchor: Anchor::new(TextSize::ZERO, Affinity::After),
            head: Anchor::new(TextSize::new(2), Affinity::Before),
        };
        let reversed = SelectionRange {
            anchor: forward.head,
            head: forward.anchor,
        };

        let projected_forward =
            ProjectedSelection::from_source(&projection, &stops, forward).unwrap();
        let projected_reversed =
            ProjectedSelection::from_source(&projection, &stops, reversed).unwrap();

        assert_eq!(
            projected_forward.anchor().projected(),
            projected_forward.focus().projected()
        );
        assert_eq!(projected_forward.anchor().bias(), HitBias::Before);
        assert_eq!(projected_forward.focus().bias(), HitBias::After);
        assert_eq!(projected_forward.to_source(&stops), Some(forward));
        assert_eq!(projected_reversed.anchor().bias(), HitBias::After);
        assert_eq!(projected_reversed.focus().bias(), HitBias::Before);
        assert_eq!(projected_reversed.to_source(&stops), Some(reversed));
    }

    #[test]
    fn source_endpoint_inside_hidden_bytes_snaps_by_affinity() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);
        let before = ProjectedSelectionEndpoint::from_source(
            &projection,
            &stops,
            Anchor::new(TextSize::new(1), Affinity::Before),
        )
        .unwrap();
        let after = ProjectedSelectionEndpoint::from_source(
            &projection,
            &stops,
            Anchor::new(TextSize::new(1), Affinity::After),
        )
        .unwrap();

        assert_eq!(before.projected(), ProjectedSize::ZERO);
        assert_eq!(after.projected(), ProjectedSize::ZERO);
        assert_eq!(before.to_source(&stops).unwrap().offset, TextSize::ZERO);
        assert_eq!(
            after.to_source(&stops).unwrap().offset,
            TextSize::new(2)
        );
    }

    #[test]
    fn source_endpoint_inside_grapheme_snaps_once_by_affinity() {
        let projection = projection("한글", RevealPolicy::SourceVisible);
        let stops = ProjectedCaretStops::from_projection(&projection);
        let before = ProjectedSelectionEndpoint::from_source(
            &projection,
            &stops,
            Anchor::new(TextSize::new(1), Affinity::Before),
        )
        .unwrap();
        let after = ProjectedSelectionEndpoint::from_source(
            &projection,
            &stops,
            Anchor::new(TextSize::new(1), Affinity::After),
        )
        .unwrap();

        assert_eq!(before.projected(), ProjectedSize::ZERO);
        assert_eq!(after.projected(), ProjectedSize::new(3));
        assert_eq!(before.to_source(&stops).unwrap().offset, TextSize::ZERO);
        assert_eq!(
            after.to_source(&stops).unwrap().offset,
            TextSize::new(3)
        );
    }
}
