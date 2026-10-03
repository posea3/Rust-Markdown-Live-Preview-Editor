use std::ops::Range;

use mdedit_core::{Affinity, Anchor, Revision, SelectionSet, TextRange};
use mdedit_input::EditorSession;
use mdedit_live::{
    HitBias, ProjectedCaretStops, ProjectedSelectionEndpoint, ProjectedSize, Projection,
    ReflowAnchor, RevealContext, RevealPolicy,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
};

pub struct LivePreviewState {
    block_cache: BlockCache,
    frame: Option<LivePreviewFrame>,
}

impl Default for LivePreviewState {
    fn default() -> Self {
        Self::new()
    }
}

impl LivePreviewState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            block_cache: BlockCache::new(),
            frame: None,
        }
    }

    pub fn refresh(&mut self, session: &EditorSession) -> Result<LivePreviewRefresh, String> {
        if self
            .frame
            .as_ref()
            .is_some_and(|frame| frame.matches(session))
        {
            return Ok(LivePreviewRefresh::unchanged());
        }

        let next = match LivePreviewFrame::build(session, &mut self.block_cache) {
            Ok(frame) => frame,
            Err(error) => {
                self.frame = None;
                return Err(error);
            }
        };
        let reflow = self
            .frame
            .as_ref()
            .and_then(|previous| previous.reflow_to(&next, session.selections().primary().head));
        self.frame = Some(next);
        Ok(LivePreviewRefresh {
            changed: true,
            reflow,
        })
    }

    #[must_use]
    pub fn display_text(&self) -> Option<&str> {
        self.frame.as_ref().map(LivePreviewFrame::display_text)
    }

    #[must_use]
    pub fn preedit_range(&self) -> Option<Range<usize>> {
        self.frame
            .as_ref()
            .and_then(LivePreviewFrame::preedit_range)
    }

    #[must_use]
    pub fn display_caret_offset(&self, session: &EditorSession) -> Option<usize> {
        self.frame
            .as_ref()
            .filter(|frame| frame.matches(session))
            .and_then(|frame| frame.display_caret_offset(session))
    }

    #[must_use]
    pub fn source_anchor_to_display(
        &self,
        session: &EditorSession,
        anchor: Anchor,
    ) -> Option<usize> {
        self.frame
            .as_ref()
            .filter(|frame| frame.matches(session))
            .and_then(|frame| frame.source_anchor_to_display(anchor))
    }

    #[must_use]
    pub fn display_to_source_anchor(
        &self,
        session: &EditorSession,
        display_offset: usize,
        affinity: Affinity,
    ) -> Option<Anchor> {
        self.frame
            .as_ref()
            .filter(|frame| frame.matches(session))
            .and_then(|frame| frame.display_to_source_anchor(display_offset, affinity))
    }

    #[cfg(test)]
    fn frame(&self) -> Option<&LivePreviewFrame> {
        self.frame.as_ref()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LivePreviewRefresh {
    changed: bool,
    reflow: Option<LivePreviewReflow>,
}

impl LivePreviewRefresh {
    const fn unchanged() -> Self {
        Self {
            changed: false,
            reflow: None,
        }
    }

    #[must_use]
    pub const fn changed(self) -> bool {
        self.changed
    }

    #[must_use]
    pub const fn reflow(self) -> Option<LivePreviewReflow> {
        self.reflow
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LivePreviewReflow {
    anchor: ReflowAnchor,
}

impl LivePreviewReflow {
    #[must_use]
    pub const fn anchor(self) -> ReflowAnchor {
        self.anchor
    }

    #[must_use]
    pub fn before_display_offset(self) -> usize {
        self.anchor.before_projected().projected().to_usize()
    }

    #[must_use]
    pub fn after_display_offset(self) -> usize {
        self.anchor.after_projected().projected().to_usize()
    }
}

struct LivePreviewFrame {
    revision: Revision,
    selections: SelectionSet,
    composition: Option<CompositionKey>,
    projection: Projection,
    stops: ProjectedCaretStops,
    display_text: String,
    overlay: Option<CompositionOverlay>,
    preedit_range: Option<Range<usize>>,
    composition_cursor: Option<usize>,
}

impl LivePreviewFrame {
    fn build(session: &EditorSession, block_cache: &mut BlockCache) -> Result<Self, String> {
        let document = session.document().snapshot();
        let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
        let blocks = block_cache
            .reconcile(&document, &syntax)
            .map_err(|error| format!("block reconciliation failed: {error}"))?;
        let delimiters = DelimiterResolver
            .resolve(&document, &syntax)
            .map_err(|error| format!("delimiter resolution failed: {error}"))?;

        let mut context = RevealContext::new();
        for selection in session.selections().ranges() {
            if selection.is_caret() {
                context = context.with_caret(selection.head.offset);
            } else {
                let (start, end) = selection.ordered_offsets();
                let range = TextRange::new(start, end)
                    .map_err(|error| format!("selection reveal range failed: {error}"))?;
                context = context.with_selection(range);
            }
        }
        if let Some(composition) = session.composition() {
            context = context.with_composition(composition.replace_range());
        }

        let projection = Projection::build(
            &document,
            &syntax,
            &blocks,
            &delimiters,
            RevealPolicy::ConcealInactive,
            &context,
        )
        .map_err(|error| format!("live projection failed: {error}"))?;
        let stops = ProjectedCaretStops::from_projection(&projection);

        let mut display_text = projection.text().to_owned();
        let mut overlay = None;
        let mut preedit_range = None;
        let mut composition_cursor = None;

        if let Some(composition) = session.composition() {
            let replace = composition.replace_range();
            let projected_start = projection
                .map()
                .source_to_projected(replace.start())
                .ok_or_else(|| "composition start is outside live projection".to_owned())?
                .to_usize();
            let projected_end = projection
                .map()
                .source_to_projected(replace.end())
                .ok_or_else(|| "composition end is outside live projection".to_owned())?
                .to_usize();

            if projected_start > projected_end
                || projected_end > display_text.len()
                || !display_text.is_char_boundary(projected_start)
                || !display_text.is_char_boundary(projected_end)
            {
                return Err("composition range is not representable in live projection".to_owned());
            }

            let projected_range = projected_start..projected_end;
            display_text.replace_range(projected_range.clone(), composition.preedit());

            let display_end = projected_start
                .checked_add(composition.preedit().len())
                .ok_or_else(|| "composition display range overflow".to_owned())?;
            let display_range = projected_start..display_end;
            let cursor_relative = composition
                .selection()
                .map_or(composition.preedit().len(), |selection| selection.end);
            composition_cursor = display_range.start.checked_add(cursor_relative);
            preedit_range = Some(display_range.clone());
            overlay = Some(CompositionOverlay {
                projected: projected_range,
                display: display_range,
            });
        }

        Ok(Self {
            revision: session.document().revision(),
            selections: session.selections().clone(),
            composition: session.composition().map(CompositionKey::from_state),
            projection,
            stops,
            display_text,
            overlay,
            preedit_range,
            composition_cursor,
        })
    }

    fn reflow_to(&self, next: &Self, source: Anchor) -> Option<LivePreviewReflow> {
        if self.revision != next.revision || self.overlay.is_some() || next.overlay.is_some() {
            return None;
        }

        ReflowAnchor::same_source(
            &self.projection,
            &self.stops,
            &next.projection,
            &next.stops,
            source,
        )
        .map(|anchor| LivePreviewReflow { anchor })
    }

    fn matches(&self, session: &EditorSession) -> bool {
        self.revision == session.document().revision()
            && &self.selections == session.selections()
            && match (&self.composition, session.composition()) {
                (None, None) => true,
                (Some(key), Some(composition)) => key.matches(composition),
                _ => false,
            }
    }

    fn display_text(&self) -> &str {
        &self.display_text
    }

    fn preedit_range(&self) -> Option<Range<usize>> {
        self.preedit_range.clone()
    }

    fn display_caret_offset(&self, session: &EditorSession) -> Option<usize> {
        if session.composition().is_some() {
            self.composition_cursor
        } else {
            self.source_anchor_to_display(session.selections().primary().head)
        }
    }

    fn source_anchor_to_display(&self, anchor: Anchor) -> Option<usize> {
        let endpoint =
            ProjectedSelectionEndpoint::from_source(&self.projection, &self.stops, anchor)?;
        let projected = endpoint.projected().to_usize();

        self.overlay.as_ref().map_or(Some(projected), |overlay| {
            overlay.projected_to_display(projected, anchor.affinity)
        })
    }

    fn display_to_source_anchor(
        &self,
        display_offset: usize,
        affinity: Affinity,
    ) -> Option<Anchor> {
        if display_offset > self.display_text.len()
            || !self.display_text.is_char_boundary(display_offset)
        {
            return None;
        }

        let projected = self
            .overlay
            .as_ref()
            .map_or(Some(display_offset), |overlay| {
                overlay.display_to_projected(display_offset, affinity)
            })?;
        let projected = ProjectedSize::new(u32::try_from(projected).ok()?);
        let bias = match affinity {
            Affinity::Before => HitBias::Before,
            Affinity::After => HitBias::After,
        };
        let hit = self.stops.hit_test(projected, bias)?;
        Some(Anchor::new(hit.source(), hit.bias().affinity()))
    }

    #[cfg(test)]
    fn projection(&self) -> &Projection {
        &self.projection
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompositionKey {
    replace_range: TextRange,
    preedit: String,
    selection: Option<Range<usize>>,
}

impl CompositionKey {
    fn from_state(composition: &mdedit_input::CompositionState) -> Self {
        Self {
            replace_range: composition.replace_range(),
            preedit: composition.preedit().to_owned(),
            selection: composition.selection(),
        }
    }

    fn matches(&self, composition: &mdedit_input::CompositionState) -> bool {
        self.replace_range == composition.replace_range()
            && self.preedit == composition.preedit()
            && self.selection == composition.selection()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompositionOverlay {
    projected: Range<usize>,
    display: Range<usize>,
}

impl CompositionOverlay {
    fn projected_to_display(&self, projected: usize, affinity: Affinity) -> Option<usize> {
        if projected < self.projected.start {
            return Some(projected);
        }
        if projected > self.projected.end {
            return projected
                .checked_sub(self.projected.end.checked_sub(self.projected.start)?)
                .and_then(|value| {
                    value.checked_add(self.display.end.checked_sub(self.display.start)?)
                });
        }

        match affinity {
            Affinity::Before => Some(self.display.start),
            Affinity::After => Some(self.display.end),
        }
    }

    fn display_to_projected(&self, display: usize, affinity: Affinity) -> Option<usize> {
        if display < self.display.start {
            return Some(display);
        }
        if display > self.display.end {
            return display
                .checked_sub(self.display.end.checked_sub(self.display.start)?)
                .and_then(|value| {
                    value.checked_add(self.projected.end.checked_sub(self.projected.start)?)
                });
        }

        match affinity {
            Affinity::Before => Some(self.projected.start),
            Affinity::After => Some(self.projected.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use mdedit_core::{Affinity, Anchor, TextSize};
    use mdedit_input::{EditorInput, EditorSession};

    use super::*;

    fn anchor(offset: u32, affinity: Affinity) -> Anchor {
        Anchor::new(TextSize::new(offset), affinity)
    }

    #[test]
    fn inactive_markers_are_concealed_in_native_display_frame() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(13, Affinity::After));

        let mut state = LivePreviewState::new();
        assert!(state.refresh(&session).unwrap().changed());

        let frame = state.frame().unwrap();
        assert_eq!(frame.display_text(), "bold tail");
        assert_eq!(frame.projection().text(), "bold tail");
    }

    #[test]
    fn caret_reveals_the_markdown_group_being_edited() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(3, Affinity::After));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();

        assert_eq!(state.frame().unwrap().display_text(), "**bold** tail");
    }

    #[test]
    fn collapsed_boundary_round_trips_through_native_mapping() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(13, Affinity::After));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();

        let opening_content_edge = anchor(2, Affinity::After);
        let display = state
            .source_anchor_to_display(&session, opening_content_edge)
            .unwrap();
        assert_eq!(display, 0);
        assert_eq!(
            state
                .display_to_source_anchor(&session, display, Affinity::After)
                .unwrap(),
            opening_content_edge
        );
    }

    #[test]
    fn ime_preedit_is_overlaid_on_projected_text_without_mutating_source() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(4, Affinity::After));
        session.handle(EditorInput::ImeEnabled).unwrap();
        session
            .handle(EditorInput::ImePreedit {
                text: "한".to_owned(),
                selection: Some("한".len().."한".len()),
            })
            .unwrap();

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();

        let frame = state.frame().unwrap();
        assert_eq!(session.document().text(), "**bold** tail");
        assert_eq!(frame.display_text(), "**bo한ld** tail");
        assert_eq!(frame.preedit_range(), Some(4..7));
        assert_eq!(frame.display_caret_offset(&session), Some(7));
    }

    #[test]
    fn frame_refreshes_when_only_reveal_context_changes() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(13, Affinity::After));

        let mut state = LivePreviewState::new();
        assert!(state.refresh(&session).unwrap().changed());
        assert!(!state.refresh(&session).unwrap().changed());
        assert_eq!(state.frame().unwrap().display_text(), "bold tail");

        session.set_caret(anchor(3, Affinity::After));
        assert!(state.refresh(&session).unwrap().changed());
        assert_eq!(state.frame().unwrap().display_text(), "**bold** tail");
    }

    #[test]
    fn refresh_exposes_same_source_reflow_across_reveal_change() {
        let mut session = EditorSession::new("**bold** tail").unwrap();
        session.set_caret(anchor(13, Affinity::After));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();
        assert_eq!(state.frame().unwrap().display_text(), "bold tail");

        session.set_caret(anchor(3, Affinity::After));
        let refresh = state.refresh(&session).unwrap();
        let reflow = refresh
            .reflow()
            .expect("same-revision reveal should reflow");

        assert_eq!(reflow.anchor().before_source(), anchor(3, Affinity::After));
        assert_eq!(reflow.before_display_offset(), 1);
        assert_eq!(reflow.after_display_offset(), 3);
    }

    #[test]
    fn composition_overlay_preserves_offsets_after_preedit() {
        let overlay = CompositionOverlay {
            projected: 2..5,
            display: 2..8,
        };

        assert_eq!(overlay.projected_to_display(7, Affinity::After), Some(10));
        assert_eq!(overlay.display_to_projected(10, Affinity::After), Some(7));
    }
}
