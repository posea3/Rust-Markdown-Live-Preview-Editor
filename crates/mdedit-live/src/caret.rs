use mdedit_core::TextSize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{ProjectedSize, Projection, ProjectionBias};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretDirection {
    Backward,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedCaretStop {
    projected: ProjectedSize,
    source_before: TextSize,
    source_after: TextSize,
}

impl ProjectedCaretStop {
    #[must_use]
    pub const fn projected(self) -> ProjectedSize {
        self.projected
    }

    #[must_use]
    pub const fn source_before(self) -> TextSize {
        self.source_before
    }

    #[must_use]
    pub const fn source_after(self) -> TextSize {
        self.source_after
    }

    #[must_use]
    pub const fn is_collapsed_boundary(self) -> bool {
        self.source_before.get() != self.source_after.get()
    }

    #[must_use]
    pub const fn source_for_entry(self, direction: CaretDirection) -> TextSize {
        match direction {
            CaretDirection::Backward => self.source_after,
            CaretDirection::Forward => self.source_before,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedCaretStops {
    stops: Vec<ProjectedCaretStop>,
}

impl ProjectedCaretStops {
    #[must_use]
    pub fn from_projection(projection: &Projection) -> Self {
        let mut boundaries = Vec::new();
        boundaries.push(0_usize);
        boundaries.extend(
            projection
                .text()
                .grapheme_indices(true)
                .map(|(start, grapheme)| start + grapheme.len()),
        );
        boundaries.sort_unstable();
        boundaries.dedup();

        let stops = boundaries
            .into_iter()
            .filter_map(|boundary| {
                let projected = u32::try_from(boundary).ok().map(ProjectedSize::new)?;
                let source_before = projection
                    .map()
                    .projected_to_source(projected, ProjectionBias::Before)?;
                let source_after = projection
                    .map()
                    .projected_to_source(projected, ProjectionBias::After)?;
                Some(ProjectedCaretStop {
                    projected,
                    source_before,
                    source_after,
                })
            })
            .collect();

        Self { stops }
    }

    #[must_use]
    pub fn stops(&self) -> &[ProjectedCaretStop] {
        &self.stops
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.stops.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stops.is_empty()
    }

    #[must_use]
    pub fn stop_at(&self, projected: ProjectedSize) -> Option<ProjectedCaretStop> {
        self.stops
            .binary_search_by_key(&projected, |stop| stop.projected)
            .ok()
            .map(|index| self.stops[index])
    }

    #[must_use]
    pub fn move_from_source(
        &self,
        projection: &Projection,
        source: TextSize,
        direction: CaretDirection,
    ) -> Option<TextSize> {
        let projected = projection.map().source_to_projected(source)?;
        let index = match self
            .stops
            .binary_search_by_key(&projected, |stop| stop.projected)
        {
            Ok(index) => index,
            Err(index) => match direction {
                CaretDirection::Backward => index.checked_sub(1)?,
                CaretDirection::Forward => index,
            },
        };

        let target = match direction {
            CaretDirection::Backward => index.checked_sub(1).and_then(|i| self.stops.get(i)),
            CaretDirection::Forward => self.stops.get(index + 1),
        }?;

        Some(target.source_for_entry(direction))
    }

    #[must_use]
    pub fn nearest_stop(
        &self,
        projected: ProjectedSize,
        bias: ProjectionBias,
    ) -> Option<ProjectedCaretStop> {
        match self
            .stops
            .binary_search_by_key(&projected, |stop| stop.projected)
        {
            Ok(index) => self.stops.get(index).copied(),
            Err(index) => match bias {
                ProjectionBias::Before => index
                    .checked_sub(1)
                    .and_then(|previous| self.stops.get(previous))
                    .copied(),
                ProjectionBias::After => self.stops.get(index).copied(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use mdedit_core::{Document, TextSize};
    use mdedit_markdown::{
        BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    };

    use super::*;
    use crate::{Projection, RevealContext, RevealPolicy};

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
    fn inactive_hidden_delimiters_do_not_create_extra_caret_stops() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        assert_eq!(projection.text(), "bold");
        assert_eq!(stops.len(), 5);
        assert_eq!(
            stops
                .stops()
                .iter()
                .map(|stop| stop.projected().get())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn backward_entry_uses_content_side_after_opening_marker() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        let target = stops
            .move_from_source(
                &projection,
                TextSize::new(3),
                CaretDirection::Backward,
            )
            .unwrap();

        assert_eq!(target, TextSize::new(2));
    }

    #[test]
    fn forward_entry_uses_content_side_before_closing_marker() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        let target = stops
            .move_from_source(
                &projection,
                TextSize::new(5),
                CaretDirection::Forward,
            )
            .unwrap();

        assert_eq!(target, TextSize::new(6));
    }

    #[test]
    fn repeated_movement_never_walks_hidden_source_offsets() {
        let projection = projection("**bold**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        let mut source = TextSize::new(2);
        let mut visited = vec![source.get()];
        while let Some(next) =
            stops.move_from_source(&projection, source, CaretDirection::Forward)
        {
            source = next;
            visited.push(source.get());
        }

        assert_eq!(visited, vec![2, 3, 4, 5, 6]);
        assert!(!visited.iter().any(|offset| matches!(offset, 1 | 7 | 8)));
    }

    #[test]
    fn structural_padding_collapses_into_the_same_projected_stop() {
        let projection = projection("# Heading\n", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);
        let first = stops.stop_at(ProjectedSize::ZERO).unwrap();

        assert!(first.is_collapsed_boundary());
        assert_eq!(first.source_before(), TextSize::ZERO);
        assert_eq!(first.source_after(), TextSize::new(2));
    }

    #[test]
    fn unicode_graphemes_produce_one_visible_step_each() {
        let projection = projection("**한👨‍👩‍👧‍👦e\u{301}**", RevealPolicy::ConcealInactive);
        let stops = ProjectedCaretStops::from_projection(&projection);

        assert_eq!(projection.text(), "한👨‍👩‍👧‍👦e\u{301}");
        assert_eq!(stops.len(), 4);

        let mut source = TextSize::new(2);
        let mut moves = 0;
        while let Some(next) =
            stops.move_from_source(&projection, source, CaretDirection::Forward)
        {
            source = next;
            moves += 1;
        }
        assert_eq!(moves, 3);
    }

    #[test]
    fn source_visible_mode_uses_grapheme_stops_without_projection_collapse() {
        let projection = projection("a한🙂", RevealPolicy::SourceVisible);
        let stops = ProjectedCaretStops::from_projection(&projection);

        assert_eq!(stops.len(), 4);
        assert!(
            stops
                .stops()
                .iter()
                .all(|stop| !stop.is_collapsed_boundary())
        );
    }

    #[test]
    fn nearest_stop_snaps_to_requested_side() {
        let projection = projection("한글", RevealPolicy::SourceVisible);
        let stops = ProjectedCaretStops::from_projection(&projection);

        assert_eq!(
            stops
                .nearest_stop(ProjectedSize::new(1), ProjectionBias::Before)
                .unwrap()
                .projected(),
            ProjectedSize::ZERO
        );
        assert_eq!(
            stops
                .nearest_stop(ProjectedSize::new(1), ProjectionBias::After)
                .unwrap()
                .projected(),
            ProjectedSize::new(3)
        );
    }
}
