use std::ops::Range;

use mdedit_core::{Affinity, Anchor, Revision, SelectionSet, TextRange};
use mdedit_input::EditorSession;
use mdedit_live::{
    CaretDirection, HitBias, ProjectedCaretStops, ProjectedSelectionEndpoint, ProjectedSize,
    ProjectedWidget, Projection, ReflowAnchor, RevealContext, RevealPolicy, WidgetKind,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    obsidian_extension_set,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetBlockReservation {
    source_range: TextRange,
    extra_lines: u16,
}

impl WidgetBlockReservation {
    #[must_use]
    pub const fn new(source_range: TextRange, extra_lines: u16) -> Self {
        Self {
            source_range,
            extra_lines,
        }
    }
}

pub struct LivePreviewState {
    block_cache: BlockCache,
    frame: Option<LivePreviewFrame>,
    reservations: Vec<WidgetBlockReservation>,
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
            reservations: Vec::new(),
        }
    }

    pub fn set_widget_reservations(
        &mut self,
        mut reservations: Vec<WidgetBlockReservation>,
    ) -> bool {
        reservations.retain(|reservation| reservation.extra_lines > 0);
        reservations.sort_by_key(|reservation| {
            (
                reservation.source_range.start().get(),
                reservation.source_range.end().get(),
                reservation.extra_lines,
            )
        });
        reservations.dedup();

        if self.reservations == reservations {
            return false;
        }

        self.reservations = reservations;
        true
    }

    pub fn refresh(&mut self, session: &EditorSession) -> Result<LivePreviewRefresh, String> {
        if self
            .frame
            .as_ref()
            .is_some_and(|frame| frame.matches(session, &self.reservations))
        {
            return Ok(LivePreviewRefresh::unchanged());
        }

        if session.composition().is_some()
            && let Some(frame) = self.frame.as_mut()
            && frame.can_reuse_projection_for_composition(session, &self.reservations)
        {
            frame.update_composition_overlay(session)?;
            return Ok(LivePreviewRefresh {
                changed: true,
                reflow: None,
            });
        }

        let next = match LivePreviewFrame::build(session, &mut self.block_cache, &self.reservations)
        {
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
            .filter(|frame| frame.matches(session, &self.reservations))
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
            .filter(|frame| frame.matches(session, &self.reservations))
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
            .filter(|frame| frame.matches(session, &self.reservations))
            .and_then(|frame| frame.display_to_source_anchor(display_offset, affinity))
    }

    #[must_use]
    pub fn move_source_caret(
        &self,
        session: &EditorSession,
        anchor: Anchor,
        direction: CaretDirection,
    ) -> Option<Anchor> {
        self.frame
            .as_ref()
            .filter(|frame| frame.matches(session, &self.reservations))
            .and_then(|frame| frame.move_source_caret(anchor, direction))
    }

    #[must_use]
    pub fn widgets<'a>(&'a self, session: &EditorSession) -> &'a [ProjectedWidget] {
        self.frame
            .as_ref()
            .filter(|frame| frame.matches(session, &self.reservations))
            .map_or(&[], |frame| frame.projection.widgets())
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
    before_display_offset: usize,
    after_display_offset: usize,
}

impl LivePreviewReflow {
    #[must_use]
    pub const fn anchor(self) -> ReflowAnchor {
        self.anchor
    }

    #[must_use]
    pub const fn before_display_offset(self) -> usize {
        self.before_display_offset
    }

    #[must_use]
    pub const fn after_display_offset(self) -> usize {
        self.after_display_offset
    }
}

struct LivePreviewFrame {
    revision: Revision,
    selections: SelectionSet,
    composition: Option<CompositionKey>,
    projection: Projection,
    stops: ProjectedCaretStops,
    display_text: String,
    reservations: Vec<WidgetBlockReservation>,
    reservation_overlay: ReservationOverlay,
    overlay: Option<CompositionOverlay>,
    preedit_range: Option<Range<usize>>,
    composition_cursor: Option<usize>,
}

impl LivePreviewFrame {
    fn build(
        session: &EditorSession,
        block_cache: &mut BlockCache,
        reservations: &[WidgetBlockReservation],
    ) -> Result<Self, String> {
        let document = session.document().snapshot();
        let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
        let extensions = obsidian_extension_set()
            .scan(&document, &syntax)
            .map_err(|error| format!("extension scan failed: {error}"))?;
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
        // Composition is a temporary display overlay. It must not change the
        // reveal/conceal projection itself; rebuilding the projection on every
        // preedit update can remap the caret and reflow the viewport on Windows.
        let projection = Projection::build_with_extensions(
            &document,
            &syntax,
            &blocks,
            &delimiters,
            &extensions,
            RevealPolicy::ConcealInactive,
            &context,
        )
        .map_err(|error| format!("live projection failed: {error}"))?;
        let stops = ProjectedCaretStops::from_projection(&projection);

        let display_text = projection.text().to_owned();
        let mut frame = Self {
            revision: session.document().revision(),
            selections: session.selections().clone(),
            composition: None,
            projection,
            stops,
            display_text,
            reservations: reservations.to_vec(),
            reservation_overlay: ReservationOverlay::default(),
            overlay: None,
            preedit_range: None,
            composition_cursor: None,
        };
        frame.update_composition_overlay(session)?;
        Ok(frame)
    }

    fn can_reuse_projection_for_composition(
        &self,
        session: &EditorSession,
        reservations: &[WidgetBlockReservation],
    ) -> bool {
        self.revision == session.document().revision()
            && &self.selections == session.selections()
            && self.reservations == reservations
            && session.composition().is_some()
    }

    fn update_composition_overlay(&mut self, session: &EditorSession) -> Result<(), String> {
        self.display_text = self.projection.text().to_owned();
        self.reservation_overlay = ReservationOverlay::apply(
            &mut self.display_text,
            &self.projection,
            &self.reservations,
        )?;
        self.overlay = None;
        self.preedit_range = None;
        self.composition_cursor = None;

        let Some(composition) = session.composition() else {
            self.composition = None;
            return Ok(());
        };

        let replace = composition.replace_range();
        let projected_start = self
            .projection
            .map()
            .source_to_projected(replace.start())
            .ok_or_else(|| "composition start is outside live projection".to_owned())?
            .to_usize();
        let projected_end = self
            .projection
            .map()
            .source_to_projected(replace.end())
            .ok_or_else(|| "composition end is outside live projection".to_owned())?
            .to_usize();
        let reserved_start = self
            .reservation_overlay
            .projected_to_display(projected_start, Affinity::Before)
            .ok_or_else(|| "composition start is outside reserved layout".to_owned())?;
        let reserved_end = self
            .reservation_overlay
            .projected_to_display(projected_end, Affinity::After)
            .ok_or_else(|| "composition end is outside reserved layout".to_owned())?;

        if reserved_start > reserved_end
            || reserved_end > self.display_text.len()
            || !self.display_text.is_char_boundary(reserved_start)
            || !self.display_text.is_char_boundary(reserved_end)
        {
            return Err("composition range is not representable in live projection".to_owned());
        }

        let reserved_range = reserved_start..reserved_end;
        self.display_text
            .replace_range(reserved_range.clone(), composition.preedit());

        let display_end = reserved_start
            .checked_add(composition.preedit().len())
            .ok_or_else(|| "composition display range overflow".to_owned())?;
        let display_range = reserved_start..display_end;
        let cursor_relative = composition
            .selection()
            .map_or(composition.preedit().len(), |selection| selection.end);

        self.composition = Some(CompositionKey::from_state(composition));
        self.composition_cursor = display_range.start.checked_add(cursor_relative);
        self.preedit_range = Some(display_range.clone());
        self.overlay = Some(CompositionOverlay {
            reserved: reserved_range,
            display: display_range,
        });

        Ok(())
    }

    fn reflow_to(&self, next: &Self, source: Anchor) -> Option<LivePreviewReflow> {
        if self.revision != next.revision || self.overlay.is_some() || next.overlay.is_some() {
            return None;
        }

        let anchor = ReflowAnchor::same_source(
            &self.projection,
            &self.stops,
            &next.projection,
            &next.stops,
            source,
        )?;
        Some(LivePreviewReflow {
            anchor,
            before_display_offset: self.source_anchor_to_display(source)?,
            after_display_offset: next.source_anchor_to_display(source)?,
        })
    }

    fn matches(&self, session: &EditorSession, reservations: &[WidgetBlockReservation]) -> bool {
        self.revision == session.document().revision()
            && &self.selections == session.selections()
            && self.reservations == reservations
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

        let reserved = self
            .reservation_overlay
            .projected_to_display(projected, anchor.affinity)?;
        self.overlay.as_ref().map_or(Some(reserved), |overlay| {
            overlay.reserved_to_display(reserved, anchor.affinity)
        })
    }

    fn move_source_caret(&self, anchor: Anchor, direction: CaretDirection) -> Option<Anchor> {
        let offset = self
            .stops
            .move_from_source(&self.projection, anchor.offset, direction)?;
        let affinity = match direction {
            CaretDirection::Backward => Affinity::Before,
            CaretDirection::Forward => Affinity::After,
        };
        Some(Anchor::new(offset, affinity))
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

        let reserved = self
            .overlay
            .as_ref()
            .map_or(Some(display_offset), |overlay| {
                overlay.display_to_reserved(display_offset, affinity)
            })?;
        let projected = self
            .reservation_overlay
            .display_to_projected(reserved, affinity)?;
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
struct ReservationEntry {
    projected_anchor: usize,
    display: Range<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReservationOverlay {
    entries: Vec<ReservationEntry>,
}

impl ReservationOverlay {
    fn apply(
        display_text: &mut String,
        projection: &Projection,
        reservations: &[WidgetBlockReservation],
    ) -> Result<Self, String> {
        let mut specs = Vec::<(usize, usize)>::new();

        for reservation in reservations {
            let Some(widget) = projection.widgets().iter().find(|widget| {
                matches!(
                    widget.kind(),
                    WidgetKind::Image | WidgetKind::DisplayMath | WidgetKind::Table
                ) && widget.source_range() == reservation.source_range
            }) else {
                continue;
            };
            let anchor = widget.projected_range().start().to_usize();
            if anchor > display_text.len() || !display_text.is_char_boundary(anchor) {
                return Err("widget reservation anchor is outside projected text".to_owned());
            }
            specs.push((anchor, usize::from(reservation.extra_lines)));
        }

        specs.sort_unstable_by_key(|(anchor, _)| *anchor);
        let mut merged = Vec::<(usize, usize)>::new();
        for (anchor, count) in specs {
            if let Some((last_anchor, last_count)) = merged.last_mut()
                && *last_anchor == anchor
            {
                *last_count = last_count.saturating_add(count);
            } else {
                merged.push((anchor, count));
            }
        }

        let mut added = 0usize;
        let mut entries = Vec::with_capacity(merged.len());
        for (anchor, count) in &merged {
            let start = anchor
                .checked_add(added)
                .ok_or_else(|| "widget reservation offset overflow".to_owned())?;
            let end = start
                .checked_add(*count)
                .ok_or_else(|| "widget reservation offset overflow".to_owned())?;
            entries.push(ReservationEntry {
                projected_anchor: *anchor,
                display: start..end,
            });
            added = added
                .checked_add(*count)
                .ok_or_else(|| "widget reservation offset overflow".to_owned())?;
        }

        for (anchor, count) in merged.into_iter().rev() {
            if count > 0 {
                display_text.insert_str(anchor, &"\n".repeat(count));
            }
        }

        Ok(Self { entries })
    }

    fn projected_to_display(&self, projected: usize, affinity: Affinity) -> Option<usize> {
        let mut added = 0usize;
        for entry in &self.entries {
            if projected < entry.projected_anchor {
                break;
            }
            if projected == entry.projected_anchor {
                return match affinity {
                    Affinity::Before => projected.checked_add(added),
                    Affinity::After => Some(entry.display.end),
                };
            }
            added = added.checked_add(entry.display.end.checked_sub(entry.display.start)?)?;
        }
        projected.checked_add(added)
    }

    fn display_to_projected(&self, display: usize, _affinity: Affinity) -> Option<usize> {
        let mut added = 0usize;
        for entry in &self.entries {
            if display < entry.display.start {
                break;
            }
            if display <= entry.display.end {
                return Some(entry.projected_anchor);
            }
            added = added.checked_add(entry.display.end.checked_sub(entry.display.start)?)?;
        }
        display.checked_sub(added)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompositionOverlay {
    reserved: Range<usize>,
    display: Range<usize>,
}

impl CompositionOverlay {
    fn reserved_to_display(&self, reserved: usize, affinity: Affinity) -> Option<usize> {
        if reserved < self.reserved.start {
            return Some(reserved);
        }
        if reserved > self.reserved.end {
            return reserved
                .checked_sub(self.reserved.end.checked_sub(self.reserved.start)?)
                .and_then(|value| {
                    value.checked_add(self.display.end.checked_sub(self.display.start)?)
                });
        }

        match affinity {
            Affinity::Before => Some(self.display.start),
            Affinity::After => Some(self.display.end),
        }
    }

    fn display_to_reserved(&self, display: usize, affinity: Affinity) -> Option<usize> {
        if display < self.display.start {
            return Some(display);
        }
        if display > self.display.end {
            return display
                .checked_sub(self.display.end.checked_sub(self.display.start)?)
                .and_then(|value| {
                    value.checked_add(self.reserved.end.checked_sub(self.reserved.start)?)
                });
        }

        match affinity {
            Affinity::Before => Some(self.reserved.start),
            Affinity::After => Some(self.reserved.end),
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
    fn image_block_reservation_adds_layout_rows_without_changing_source_mapping() {
        let source = "before\n\n![alt](image.png)\n\nafter";
        let mut session = EditorSession::new(source).unwrap();
        session.set_caret(anchor(
            u32::try_from(source.len()).unwrap(),
            Affinity::After,
        ));

        let marker_start = source.find("![alt]").unwrap();
        let marker_end = marker_start + "![alt](image.png)".len();
        let range = TextRange::new(
            TextSize::try_from_usize(marker_start).unwrap(),
            TextSize::try_from_usize(marker_end).unwrap(),
        )
        .unwrap();

        let mut state = LivePreviewState::new();
        state.set_widget_reservations(vec![WidgetBlockReservation::new(range, 3)]);
        state.refresh(&session).unwrap();

        let frame = state.frame().unwrap();
        assert_eq!(
            frame.display_text().matches('\n').count(),
            frame.projection().text().matches('\n').count() + 3
        );

        let source_after = Anchor::new(
            TextSize::try_from_usize(source.len()).unwrap(),
            Affinity::After,
        );
        let projected_after =
            ProjectedSelectionEndpoint::from_source(frame.projection(), &frame.stops, source_after)
                .unwrap()
                .projected()
                .to_usize();
        assert_eq!(
            frame.source_anchor_to_display(source_after),
            Some(projected_after + 3)
        );

        let widget = frame
            .projection()
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Image)
            .unwrap();
        let reservation_start = widget.projected_range().start().to_usize();
        let hit = frame
            .display_to_source_anchor(reservation_start + 1, Affinity::After)
            .unwrap();
        assert!(
            hit.offset == range.start() || hit.offset == range.end(),
            "synthetic image rows must map back to the collapsed image source"
        );
    }

    #[test]
    fn obsidian_extension_widgets_reach_live_preview_frame() {
        let source = concat!(
            "> [!tip] Native callout\n",
            "> Body text.\n",
            "\n",
            "![[diagram.svg|Architecture]]\n",
        );
        let mut session = EditorSession::new(source).unwrap();
        session.set_caret(anchor(
            u32::try_from(source.len()).unwrap(),
            Affinity::After,
        ));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();

        let widgets = state.widgets(&session);
        assert!(
            widgets
                .iter()
                .any(|widget| widget.kind() == WidgetKind::Callout),
            "LivePreviewState must scan Obsidian callouts before building the projection",
        );
        assert!(
            widgets
                .iter()
                .any(|widget| widget.kind() == WidgetKind::CustomEmbed),
            "LivePreviewState must scan standalone Obsidian embeds before building the projection",
        );
    }

    #[test]
    fn table_block_reservation_adds_layout_rows_without_changing_source_mapping() {
        let source = concat!(
            "before\n\n",
            "| Name | Score |\n",
            "| :--- | ---: |\n",
            "| Ada | 42 |\n",
            "| Bob | 7 |\n",
            "\nafter",
        );
        let mut session = EditorSession::new(source).unwrap();
        session.set_caret(anchor(
            u32::try_from(source.len()).unwrap(),
            Affinity::After,
        ));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();
        let range = state
            .frame()
            .unwrap()
            .projection()
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Table)
            .expect("inactive table widget")
            .source_range();

        assert!(state.set_widget_reservations(vec![WidgetBlockReservation::new(range, 2)]));
        state.refresh(&session).unwrap();

        let frame = state.frame().unwrap();
        assert_eq!(
            frame.display_text().matches('\n').count(),
            frame.projection().text().matches('\n').count() + 2,
            "native table rows must reserve real layout lines so following text cannot overlap the grid",
        );

        let source_after = Anchor::new(
            TextSize::try_from_usize(source.len()).unwrap(),
            Affinity::After,
        );
        let projected_after =
            ProjectedSelectionEndpoint::from_source(frame.projection(), &frame.stops, source_after)
                .unwrap()
                .projected()
                .to_usize();
        assert_eq!(
            frame.source_anchor_to_display(source_after),
            Some(projected_after + 2),
            "source positions after the table must include the reserved native rows",
        );

        let widget = frame
            .projection()
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Table)
            .unwrap();
        let reservation_start = widget.projected_range().start().to_usize();
        let hit = frame
            .display_to_source_anchor(reservation_start + 1, Affinity::After)
            .unwrap();
        assert!(
            hit.offset == range.start() || hit.offset == range.end(),
            "synthetic table rows must map back to the collapsed table source",
        );
    }

    #[test]
    fn adding_table_reservation_produces_reflow_offsets_for_same_source_anchor() {
        let source = concat!(
            "before\n\n",
            "| Name | Score |\n",
            "| --- | ---: |\n",
            "| Ada | 42 |\n",
            "\nafter",
        );
        let mut session = EditorSession::new(source).unwrap();
        session.set_caret(anchor(
            u32::try_from(source.len()).unwrap(),
            Affinity::After,
        ));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();
        let before = state
            .display_caret_offset(&session)
            .expect("caret before reservation");
        let range = state
            .frame()
            .unwrap()
            .projection()
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Table)
            .expect("inactive table widget")
            .source_range();

        assert!(state.set_widget_reservations(vec![WidgetBlockReservation::new(range, 2)]));
        let refresh = state.refresh(&session).unwrap();
        let reflow = refresh.reflow().expect("table reservation reflow");

        assert_eq!(reflow.before_display_offset(), before);
        assert_eq!(
            reflow.after_display_offset(),
            before + 2,
            "table reservation must participate in reflow just like image/math block reservations",
        );
    }

    #[test]
    fn adding_image_reservation_produces_reflow_offsets_for_same_source_anchor() {
        let source = "before\n\n![alt](image.png)\n\nafter";
        let mut session = EditorSession::new(source).unwrap();
        session.set_caret(anchor(
            u32::try_from(source.len()).unwrap(),
            Affinity::After,
        ));

        let marker_start = source.find("![alt]").unwrap();
        let marker_end = marker_start + "![alt](image.png)".len();
        let range = TextRange::new(
            TextSize::try_from_usize(marker_start).unwrap(),
            TextSize::try_from_usize(marker_end).unwrap(),
        )
        .unwrap();

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();
        let before = state
            .display_caret_offset(&session)
            .expect("caret before reservation");

        assert!(state.set_widget_reservations(vec![WidgetBlockReservation::new(range, 4,)]));
        let refresh = state.refresh(&session).unwrap();
        let reflow = refresh.reflow().expect("reservation reflow");

        assert_eq!(reflow.before_display_offset(), before);
        assert_eq!(reflow.after_display_offset(), before + 4);
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
    fn ime_preedit_keeps_the_precomposition_projection_stable() {
        let mut session = EditorSession::new("**bold**\nplain").unwrap();
        session.set_caret(anchor(14, Affinity::After));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();
        let projection_before = state.frame().unwrap().projection().text().to_owned();
        assert_eq!(projection_before, "bold\nplain");

        session.handle(EditorInput::ImeEnabled).unwrap();
        session
            .handle(EditorInput::ImePreedit {
                text: "한".to_owned(),
                selection: Some("한".len().."한".len()),
            })
            .unwrap();

        state.refresh(&session).unwrap();
        let frame = state.frame().unwrap();

        assert_eq!(frame.projection().text(), projection_before);
        assert_eq!(frame.display_text(), "bold\nplain한");
        assert_eq!(session.document().text(), "**bold**\nplain");
    }

    #[test]
    fn projected_caret_fallback_crosses_line_boundaries() {
        let mut session = EditorSession::new("first\nsecond").unwrap();
        session.set_caret(anchor(5, Affinity::After));

        let mut state = LivePreviewState::new();
        state.refresh(&session).unwrap();

        let next = state
            .move_source_caret(
                &session,
                session.selections().primary().head,
                CaretDirection::Forward,
            )
            .unwrap();
        assert_eq!(next.offset.to_usize(), 6);

        session.set_caret(next);
        state.refresh(&session).unwrap();
        let previous = state
            .move_source_caret(
                &session,
                session.selections().primary().head,
                CaretDirection::Backward,
            )
            .unwrap();
        assert_eq!(previous.offset.to_usize(), 5);
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
            reserved: 2..5,
            display: 2..8,
        };

        assert_eq!(overlay.reserved_to_display(7, Affinity::After), Some(10));
        assert_eq!(overlay.display_to_reserved(10, Affinity::After), Some(7));
    }
}
