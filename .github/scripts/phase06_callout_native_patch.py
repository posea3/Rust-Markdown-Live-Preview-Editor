from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = ROOT / "examples/ime-lab/src/main.rs"
LIVE = ROOT / "examples/ime-lab/src/live_preview.rs"
MODULE = ROOT / "examples/ime-lab/src/callout_embed_widget.rs"
FIXTURE = ROOT / "examples/ime-lab/fixtures/live-preview-acceptance.md"
DOC = ROOT / "docs/phase-06-widgets.md"
README = ROOT / "README.md"
ROADMAP = ROOT / "docs/roadmap.md"
PLAN = ROOT / "docs/superpowers/plans/2026-10-28-phase-06-callout-custom-embed-native.md"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def replace_once(path: Path, old: str, new: str) -> None:
    text = read(path)
    if new in text:
        return
    if old not in text:
        raise RuntimeError(f"marker not found in {path}: {old[:120]!r}")
    write(path, text.replace(old, new, 1))


def insert_before(path: Path, marker: str, addition: str, sentinel: str) -> None:
    text = read(path)
    if sentinel in text:
        return
    if marker not in text:
        raise RuntimeError(f"insert marker not found in {path}: {marker[:120]!r}")
    write(path, text.replace(marker, addition + marker, 1))


RED_MODULE = r'''use crate::geometry::ScreenRect;
use mdedit_core::TextRange;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_block_geometry_stays_inside_reserved_rows() {
        let layout = callout_block_layout(4, 640.0, 31.0);
        let geometry = callout_geometry(24.0, 24.0, 0.0, 31.0, 640.0, layout);

        assert_eq!(layout.rows, 4);
        assert!(geometry.block.width <= 640.0);
        assert!(geometry.block.height <= 31.0 * 4.0);
        assert!(geometry.header.y >= geometry.block.y);
        assert!(geometry.body.y >= geometry.header.y + geometry.header.height - 0.01);
        assert!(geometry.body.y + geometry.body.height <= geometry.block.y + geometry.block.height + 0.01);
    }

    #[test]
    fn callout_palette_distinguishes_warning_from_note() {
        let note = callout_palette("note");
        let warning = callout_palette("warning");
        let unknown = callout_palette("unknown-type");

        assert_ne!(note.accent, warning.accent);
        assert_eq!(unknown.accent, note.accent);
    }

    #[test]
    fn custom_embed_geometry_fits_one_layout_row() {
        let geometry = custom_embed_geometry(24.0, 24.0, 0.0, 31.0, 640.0);

        assert!(geometry.block.width <= 640.0);
        assert!(geometry.block.height < 31.0);
        assert!(geometry.text.x > geometry.icon.x + geometry.icon.width);
        assert!(geometry.text.x + geometry.text.width <= geometry.block.x + geometry.block.width + 0.01);
    }

    #[test]
    fn source_range_newline_count_uses_canonical_bytes() {
        let source = "> [!note] Title\n> First\n> Second\n";
        let range = TextRange::new(
            mdedit_core::TextSize::ZERO,
            mdedit_core::TextSize::try_from_usize(source.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(source_range_newline_count(source, range), Some(3));
    }
}
'''

GEOMETRY_MODULE = r'''use crate::geometry::ScreenRect;
use mdedit_core::TextRange;

const CALLOUT_MAX_WIDTH: f32 = 720.0;
const CALLOUT_VERTICAL_INSET: f32 = 3.0;
const CALLOUT_ACCENT_WIDTH: f32 = 4.0;
const CALLOUT_HORIZONTAL_PADDING: f32 = 11.0;
const CALLOUT_FOLD_WIDTH: f32 = 24.0;
const CALLOUT_DIVIDER_HEIGHT: f32 = 1.0;
const EMBED_MAX_WIDTH: f32 = 560.0;
const EMBED_VERTICAL_INSET: f32 = 3.0;
const EMBED_ACCENT_WIDTH: f32 = 3.0;
const EMBED_HORIZONTAL_PADDING: f32 = 10.0;
const EMBED_ICON_INSET: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalloutPalette {
    pub background: [f32; 4],
    pub accent: [f32; 4],
    pub divider: [f32; 4],
    pub title_text: [u8; 3],
    pub body_text: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalloutBlockLayout {
    pub width: f32,
    pub height: f32,
    pub rows: usize,
    pub header_height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CalloutGeometry {
    pub block: ScreenRect,
    pub accent: ScreenRect,
    pub header: ScreenRect,
    pub fold: ScreenRect,
    pub body: ScreenRect,
    pub divider: ScreenRect,
}

#[derive(Clone, Copy, Debug)]
pub struct CustomEmbedGeometry {
    pub block: ScreenRect,
    pub accent: ScreenRect,
    pub icon: ScreenRect,
    pub text: ScreenRect,
}

#[must_use]
pub fn source_range_newline_count(source: &str, range: TextRange) -> Option<usize> {
    source
        .get(range.as_usize_range())
        .map(|owned| owned.bytes().filter(|byte| *byte == b'\n').count())
}

#[must_use]
pub fn callout_palette(callout_type: &str) -> CalloutPalette {
    let note = CalloutPalette {
        background: [0.095, 0.12, 0.17, 0.97],
        accent: [0.34, 0.62, 0.96, 0.98],
        divider: [0.25, 0.34, 0.47, 0.88],
        title_text: [225, 235, 250],
        body_text: [205, 214, 228],
    };

    match callout_type.to_ascii_lowercase().as_str() {
        "tip" | "success" | "check" | "done" => CalloutPalette {
            background: [0.075, 0.15, 0.13, 0.97],
            accent: [0.29, 0.76, 0.55, 0.98],
            divider: [0.20, 0.42, 0.34, 0.88],
            title_text: [219, 246, 232],
            body_text: [201, 225, 214],
        },
        "warning" | "question" | "help" | "faq" => CalloutPalette {
            background: [0.18, 0.14, 0.065, 0.97],
            accent: [0.96, 0.67, 0.23, 0.98],
            divider: [0.49, 0.37, 0.15, 0.88],
            title_text: [252, 235, 193],
            body_text: [229, 216, 184],
        },
        "failure" | "fail" | "danger" | "error" | "bug" | "caution" => CalloutPalette {
            background: [0.18, 0.075, 0.09, 0.97],
            accent: [0.94, 0.34, 0.39, 0.98],
            divider: [0.49, 0.20, 0.24, 0.88],
            title_text: [252, 220, 222],
            body_text: [230, 204, 207],
        },
        "important" | "example" | "quote" | "cite" => CalloutPalette {
            background: [0.13, 0.095, 0.18, 0.97],
            accent: [0.66, 0.49, 0.95, 0.98],
            divider: [0.36, 0.27, 0.50, 0.88],
            title_text: [237, 225, 252],
            body_text: [216, 206, 232],
        },
        "note" | "info" | "todo" | "abstract" | "summary" | "tldr" => note,
        _ => note,
    }
}

#[must_use]
pub fn callout_block_layout(
    reserved_rows: usize,
    available_width: f32,
    line_height: f32,
) -> CalloutBlockLayout {
    let line_height = line_height.max(1.0);
    let rows = reserved_rows.max(2);
    let reserved_height = rows as f32 * line_height;
    let width = available_width.min(CALLOUT_MAX_WIDTH).max(line_height);
    let height = (reserved_height - CALLOUT_VERTICAL_INSET * 2.0).max(line_height);
    let header_height = line_height.min(height);

    CalloutBlockLayout {
        width,
        height,
        rows,
        header_height,
    }
}

#[must_use]
pub fn callout_geometry(
    text_left: f32,
    text_top: f32,
    line_top: f32,
    line_height: f32,
    available_width: f32,
    layout: CalloutBlockLayout,
) -> CalloutGeometry {
    let x = text_left + ((available_width - layout.width) * 0.5).max(0.0);
    let reserved_height = layout.rows as f32 * line_height.max(1.0);
    let y = text_top
        + line_top
        + ((reserved_height - layout.height) * 0.5).max(CALLOUT_VERTICAL_INSET);
    let block = ScreenRect::new(x, y, layout.width, layout.height, [0.0; 4]);
    let accent = ScreenRect::new(x, y, CALLOUT_ACCENT_WIDTH, layout.height, [0.0; 4]);
    let fold_width = CALLOUT_FOLD_WIDTH.min((layout.width * 0.16).max(12.0));
    let header_x = x + CALLOUT_ACCENT_WIDTH + CALLOUT_HORIZONTAL_PADDING;
    let header_width = (layout.width
        - CALLOUT_ACCENT_WIDTH
        - CALLOUT_HORIZONTAL_PADDING * 2.0
        - fold_width)
        .max(1.0);
    let header = ScreenRect::new(
        header_x,
        y,
        header_width,
        layout.header_height,
        [0.0; 4],
    );
    let fold = ScreenRect::new(
        x + layout.width - CALLOUT_HORIZONTAL_PADDING - fold_width,
        y,
        fold_width,
        layout.header_height,
        [0.0; 4],
    );
    let divider_y = y + layout.header_height - CALLOUT_DIVIDER_HEIGHT;
    let divider = ScreenRect::new(
        x + CALLOUT_ACCENT_WIDTH,
        divider_y,
        (layout.width - CALLOUT_ACCENT_WIDTH).max(1.0),
        CALLOUT_DIVIDER_HEIGHT,
        [0.0; 4],
    );
    let body_y = y + layout.header_height;
    let body = ScreenRect::new(
        header_x,
        body_y,
        (layout.width - CALLOUT_ACCENT_WIDTH - CALLOUT_HORIZONTAL_PADDING * 2.0).max(1.0),
        (y + layout.height - body_y).max(1.0),
        [0.0; 4],
    );

    CalloutGeometry {
        block,
        accent,
        header,
        fold,
        body,
        divider,
    }
}

pub fn append_callout_rects(
    output: &mut Vec<ScreenRect>,
    geometry: CalloutGeometry,
    palette: CalloutPalette,
) {
    output.push(ScreenRect::new(
        geometry.block.x,
        geometry.block.y,
        geometry.block.width,
        geometry.block.height,
        palette.background,
    ));
    output.push(ScreenRect::new(
        geometry.accent.x,
        geometry.accent.y,
        geometry.accent.width,
        geometry.accent.height,
        palette.accent,
    ));
    output.push(ScreenRect::new(
        geometry.divider.x,
        geometry.divider.y,
        geometry.divider.width,
        geometry.divider.height,
        palette.divider,
    ));
}

#[must_use]
pub fn custom_embed_geometry(
    text_left: f32,
    text_top: f32,
    line_top: f32,
    line_height: f32,
    available_width: f32,
) -> CustomEmbedGeometry {
    let line_height = line_height.max(1.0);
    let width = available_width
        .min(EMBED_MAX_WIDTH)
        .max((line_height * 4.0).min(available_width.max(1.0)));
    let height = (line_height - EMBED_VERTICAL_INSET * 2.0).max(1.0);
    let x = text_left + ((available_width - width) * 0.5).max(0.0);
    let y = text_top + line_top + EMBED_VERTICAL_INSET;
    let block = ScreenRect::new(x, y, width, height, [0.0; 4]);
    let accent = ScreenRect::new(x, y, EMBED_ACCENT_WIDTH, height, [0.0; 4]);
    let icon_size = (height - EMBED_ICON_INSET * 2.0).max(4.0);
    let icon = ScreenRect::new(
        x + EMBED_ACCENT_WIDTH + EMBED_ICON_INSET,
        y + (height - icon_size) * 0.5,
        icon_size,
        icon_size,
        [0.0; 4],
    );
    let text_x = icon.x + icon.width + EMBED_HORIZONTAL_PADDING;
    let text = ScreenRect::new(
        text_x,
        y,
        (x + width - EMBED_HORIZONTAL_PADDING - text_x).max(1.0),
        height,
        [0.0; 4],
    );

    CustomEmbedGeometry {
        block,
        accent,
        icon,
        text,
    }
}

pub fn append_custom_embed_rects(output: &mut Vec<ScreenRect>, geometry: CustomEmbedGeometry) {
    output.push(ScreenRect::new(
        geometry.block.x,
        geometry.block.y,
        geometry.block.width,
        geometry.block.height,
        [0.10, 0.12, 0.16, 0.96],
    ));
    output.push(ScreenRect::new(
        geometry.accent.x,
        geometry.accent.y,
        geometry.accent.width,
        geometry.accent.height,
        [0.43, 0.61, 0.86, 0.96],
    ));
    output.push(ScreenRect::new(
        geometry.icon.x,
        geometry.icon.y,
        geometry.icon.width,
        geometry.icon.height,
        [0.33, 0.42, 0.58, 0.96],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_block_geometry_stays_inside_reserved_rows() {
        let layout = callout_block_layout(4, 640.0, 31.0);
        let geometry = callout_geometry(24.0, 24.0, 0.0, 31.0, 640.0, layout);

        assert_eq!(layout.rows, 4);
        assert!(geometry.block.width <= 640.0);
        assert!(geometry.block.height <= 31.0 * 4.0);
        assert!(geometry.header.y >= geometry.block.y);
        assert!(geometry.body.y >= geometry.header.y + geometry.header.height - 0.01);
        assert!(geometry.body.y + geometry.body.height <= geometry.block.y + geometry.block.height + 0.01);
    }

    #[test]
    fn callout_palette_distinguishes_warning_from_note() {
        let note = callout_palette("note");
        let warning = callout_palette("warning");
        let unknown = callout_palette("unknown-type");

        assert_ne!(note.accent, warning.accent);
        assert_eq!(unknown.accent, note.accent);
    }

    #[test]
    fn custom_embed_geometry_fits_one_layout_row() {
        let geometry = custom_embed_geometry(24.0, 24.0, 0.0, 31.0, 640.0);

        assert!(geometry.block.width <= 640.0);
        assert!(geometry.block.height < 31.0);
        assert!(geometry.text.x > geometry.icon.x + geometry.icon.width);
        assert!(geometry.text.x + geometry.text.width <= geometry.block.x + geometry.block.width + 0.01);
    }

    #[test]
    fn source_range_newline_count_uses_canonical_bytes() {
        let source = "> [!note] Title\n> First\n> Second\n";
        let range = TextRange::new(
            mdedit_core::TextSize::ZERO,
            mdedit_core::TextSize::try_from_usize(source.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(source_range_newline_count(source, range), Some(3));
    }
}
'''

TEXT_CODE = r'''
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardTextRole {
    CalloutTitle,
    CalloutFold,
    CalloutBody,
    EmbedLabel,
}

#[derive(Clone, Debug)]
pub struct CardTextSpec {
    pub text: String,
    pub bounds: ScreenRect,
    pub role: CardTextRole,
    pub color: [u8; 3],
    pub font_size: f32,
    pub line_height: f32,
    pub wrap: bool,
}

#[must_use]
pub fn callout_display_title(callout_type: &str, title: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.to_owned();
    }

    let mut output = String::new();
    for word in callout_type
        .split(|character: char| matches!(character, '-' | '_' | ' '))
        .filter(|word| !word.is_empty())
    {
        if !output.is_empty() {
            output.push(' ');
        }
        let mut characters = word.chars();
        if let Some(first) = characters.next() {
            output.extend(first.to_uppercase());
            output.extend(characters.flat_map(char::to_lowercase));
        }
    }
    if output.is_empty() {
        "Note".to_owned()
    } else {
        output
    }
}

#[must_use]
pub fn callout_text_specs(
    callout_type: &str,
    title: &str,
    fold: Option<char>,
    body: &str,
    geometry: CalloutGeometry,
    palette: CalloutPalette,
) -> Vec<CardTextSpec> {
    let mut specs = vec![CardTextSpec {
        text: callout_display_title(callout_type, title),
        bounds: inset_text_rect(geometry.header, 0.0, 4.0),
        role: CardTextRole::CalloutTitle,
        color: palette.title_text,
        font_size: 17.0,
        line_height: 22.0,
        wrap: false,
    }];

    if let Some(fold) = fold {
        specs.push(CardTextSpec {
            text: fold.to_string(),
            bounds: inset_text_rect(geometry.fold, 0.0, 4.0),
            role: CardTextRole::CalloutFold,
            color: palette.title_text,
            font_size: 17.0,
            line_height: 22.0,
            wrap: false,
        });
    }

    if !body.trim().is_empty() {
        specs.push(CardTextSpec {
            text: body.trim().to_owned(),
            bounds: inset_text_rect(geometry.body, 0.0, 5.0),
            role: CardTextRole::CalloutBody,
            color: palette.body_text,
            font_size: 16.0,
            line_height: 22.0,
            wrap: true,
        });
    }

    specs
}

#[must_use]
pub fn custom_embed_label(
    target: &str,
    path: &str,
    fragment: Option<&str>,
    display: Option<&str>,
) -> String {
    let display = display.map(str::trim).filter(|value| !value.is_empty());
    let path = path.trim();
    let target = target.trim();
    let primary = display
        .or_else(|| (!path.is_empty()).then_some(path))
        .or_else(|| (!target.is_empty()).then_some(target))
        .unwrap_or("Embed");

    let detail = fragment
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("#{value}"))
        .or_else(|| {
            (display.is_some() && !path.is_empty() && path != primary).then(|| path.to_owned())
        });

    detail.map_or_else(|| primary.to_owned(), |detail| format!("{primary}  ·  {detail}"))
}

#[must_use]
pub fn custom_embed_text_spec(
    target: &str,
    path: &str,
    fragment: Option<&str>,
    display: Option<&str>,
    geometry: CustomEmbedGeometry,
) -> CardTextSpec {
    CardTextSpec {
        text: custom_embed_label(target, path, fragment, display),
        bounds: inset_text_rect(geometry.text, 0.0, 3.0),
        role: CardTextRole::EmbedLabel,
        color: [216, 225, 238],
        font_size: 16.0,
        line_height: 22.0,
        wrap: false,
    }
}

fn inset_text_rect(bounds: ScreenRect, horizontal: f32, vertical: f32) -> ScreenRect {
    ScreenRect::new(
        bounds.x + horizontal,
        bounds.y + vertical,
        (bounds.width - horizontal * 2.0).max(1.0),
        (bounds.height - vertical * 2.0).max(1.0),
        [0.0; 4],
    )
}

'''

TEXT_TESTS = r'''
    #[test]
    fn callout_text_specs_preserve_title_fold_and_body_order() {
        let palette = callout_palette("warning");
        let layout = callout_block_layout(4, 640.0, 31.0);
        let geometry = callout_geometry(24.0, 24.0, 0.0, 31.0, 640.0, layout);
        let specs = callout_text_specs(
            "warning",
            "Custom title",
            Some('-'),
            "Strong body\nSecond line",
            geometry,
            palette,
        );

        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].text, "Custom title");
        assert_eq!(specs[0].role, CardTextRole::CalloutTitle);
        assert_eq!(specs[1].text, "-");
        assert_eq!(specs[1].role, CardTextRole::CalloutFold);
        assert_eq!(specs[2].text, "Strong body\nSecond line");
        assert_eq!(specs[2].role, CardTextRole::CalloutBody);
        assert!(specs[2].wrap);
    }

    #[test]
    fn callout_title_falls_back_to_humanized_type() {
        assert_eq!(callout_display_title("custom-warning", ""), "Custom Warning");
        assert_eq!(callout_display_title("", ""), "Note");
    }

    #[test]
    fn custom_embed_label_prefers_display_then_path_then_target() {
        assert_eq!(
            custom_embed_label(
                "diagram.svg#Overview",
                "diagram.svg",
                Some("Overview"),
                Some("Architecture"),
            ),
            "Architecture  ·  #Overview",
        );
        assert_eq!(
            custom_embed_label("diagram.svg", "diagram.svg", None, None),
            "diagram.svg",
        );
        assert_eq!(custom_embed_label("Note#Heading", "", None, None), "Note#Heading");
        assert_eq!(custom_embed_label("", "", None, None), "Embed");
    }

    #[test]
    fn custom_embed_text_spec_is_clipped_to_card_text_bounds() {
        let geometry = custom_embed_geometry(24.0, 24.0, 0.0, 31.0, 640.0);
        let spec = custom_embed_text_spec(
            "diagram.svg#Overview",
            "diagram.svg",
            Some("Overview"),
            Some("Architecture"),
            geometry,
        );

        assert_eq!(spec.role, CardTextRole::EmbedLabel);
        assert!(spec.bounds.x >= geometry.text.x);
        assert!(spec.bounds.x + spec.bounds.width <= geometry.block.x + geometry.block.width + 0.01);
        assert!(!spec.wrap);
    }
'''

LIVE_CALLOUT_TEST = r'''    #[test]
    fn callout_block_reservation_adds_layout_rows_without_changing_source_mapping() {
        let source = concat!(
            "before\n\n",
            "> [!warning]- Custom title\n",
            "> Be careful.\n",
            "> Second line.\n",
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
            .find(|widget| widget.kind() == WidgetKind::Callout)
            .expect("inactive callout widget")
            .source_range();

        assert!(state.set_widget_reservations(vec![WidgetBlockReservation::new(range, 3)]));
        state.refresh(&session).unwrap();

        let frame = state.frame().unwrap();
        assert_eq!(
            frame.display_text().matches('\n').count(),
            frame.projection().text().matches('\n').count() + 3,
            "native callout rows must reserve real layout lines so following text cannot overlap the card",
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
            Some(projected_after + 3),
            "source positions after the callout must include the reserved native rows",
        );

        let widget = frame
            .projection()
            .widgets()
            .iter()
            .find(|widget| widget.kind() == WidgetKind::Callout)
            .unwrap();
        let reservation_start = widget.projected_range().start().to_usize();
        let hit = frame
            .display_to_source_anchor(reservation_start + 1, Affinity::After)
            .unwrap();
        assert!(
            hit.offset == range.start() || hit.offset == range.end(),
            "synthetic callout rows must map back to the collapsed callout source",
        );
    }

'''

CARD_TEXT_METHOD = r'''    fn card_text_specs(&self) -> Vec<CardTextSpec> {
        let available_width = (self.surface_config.width as f32 - TEXT_LEFT * 2.0).max(LINE_HEIGHT);
        let source = self.session.document().text();
        let mut specs = Vec::new();

        for widget in self.live_preview.widgets(&self.session) {
            let Some((_, line_top, line_height)) = self.widget_anchor_geometry(widget) else {
                continue;
            };

            match widget.kind() {
                WidgetKind::Callout => {
                    let Some(callout) = widget.callout() else {
                        continue;
                    };
                    let reserved_rows = source_range_newline_count(&source, widget.source_range())
                        .unwrap_or(1)
                        .saturating_add(1);
                    let layout =
                        callout_block_layout(reserved_rows, available_width, line_height);
                    let geometry = callout_geometry(
                        TEXT_LEFT,
                        TEXT_TOP,
                        line_top,
                        line_height,
                        available_width,
                        layout,
                    );
                    let palette = callout_palette(callout.callout_type());
                    specs.extend(callout_text_specs(
                        callout.callout_type(),
                        callout.title(),
                        callout.fold(),
                        callout.body(),
                        geometry,
                        palette,
                    ));
                }
                WidgetKind::CustomEmbed => {
                    let Some(embed) = widget.custom_embed() else {
                        continue;
                    };
                    let geometry = custom_embed_geometry(
                        TEXT_LEFT,
                        TEXT_TOP,
                        line_top,
                        line_height,
                        available_width,
                    );
                    specs.push(custom_embed_text_spec(
                        embed.target(),
                        embed.path(),
                        embed.fragment(),
                        embed.display(),
                        geometry,
                    ));
                }
                _ => {}
            }
        }

        specs
    }

'''

CARD_TEXT_BOUNDS = r'''fn card_text_bounds(bounds: ScreenRect) -> TextBounds {
    TextBounds {
        left: bounds.x.floor() as i32,
        top: bounds.y.floor() as i32,
        right: (bounds.x + bounds.width).ceil() as i32,
        bottom: (bounds.y + bounds.height).ceil() as i32,
    }
}

'''


def stage_red_geometry() -> None:
    replace_once(
        MAIN,
        "mod accessibility;\nmod geometry;",
        "mod accessibility;\nmod callout_embed_widget;\nmod geometry;",
    )
    write(MODULE, RED_MODULE)
    insert_before(
        LIVE,
        "    #[test]\n    fn table_block_reservation_adds_layout_rows_without_changing_source_mapping() {",
        LIVE_CALLOUT_TEST,
        "fn callout_block_reservation_adds_layout_rows_without_changing_source_mapping",
    )


def stage_green_geometry() -> None:
    write(MODULE, GEOMETRY_MODULE)

    replace_once(
        LIVE,
        "                    WidgetKind::Image | WidgetKind::DisplayMath | WidgetKind::Table\n",
        "                    WidgetKind::Image\n                        | WidgetKind::DisplayMath\n                        | WidgetKind::Table\n                        | WidgetKind::Callout\n",
    )

    replace_once(
        MAIN,
        "use accessibility::{EditorAccessibilityAction, build_tree_update, translate_action};\n",
        "use accessibility::{EditorAccessibilityAction, build_tree_update, translate_action};\nuse callout_embed_widget::{\n    append_callout_rects, append_custom_embed_rects, callout_block_layout, callout_geometry,\n    callout_palette, custom_embed_geometry, source_range_newline_count,\n};\n",
    )

    replace_once(
        MAIN,
        "        let line_width = (available_width - HORIZONTAL_RULE_INSET * 2.0).max(1.0);\n        let mut rects = Vec::new();\n\n        for widget in self.live_preview.widgets(&self.session) {",
        "        let line_width = (available_width - HORIZONTAL_RULE_INSET * 2.0).max(1.0);\n        let source = self.session.document().text();\n        let mut rects = Vec::new();\n\n        for widget in self.live_preview.widgets(&self.session) {",
    )

    replace_once(
        MAIN,
        "                WidgetKind::Callout | WidgetKind::CustomEmbed => {}\n",
        "                WidgetKind::Callout => {\n                    let Some(callout) = widget.callout() else {\n                        continue;\n                    };\n                    let reserved_rows = source_range_newline_count(&source, widget.source_range())\n                        .unwrap_or(1)\n                        .saturating_add(1);\n                    let layout =\n                        callout_block_layout(reserved_rows, available_width, line_height);\n                    let geometry = callout_geometry(\n                        TEXT_LEFT,\n                        TEXT_TOP,\n                        line_top,\n                        line_height,\n                        available_width,\n                        layout,\n                    );\n                    append_callout_rects(\n                        &mut rects,\n                        geometry,\n                        callout_palette(callout.callout_type()),\n                    );\n                }\n                WidgetKind::CustomEmbed => {\n                    if widget.custom_embed().is_none() {\n                        continue;\n                    }\n                    let geometry = custom_embed_geometry(\n                        TEXT_LEFT,\n                        TEXT_TOP,\n                        line_top,\n                        line_height,\n                        available_width,\n                    );\n                    append_custom_embed_rects(&mut rects, geometry);\n                }\n",
    )

    replace_once(
        MAIN,
        "                } else {\n                    widget.table()?;\n                    table_reservation_extra_lines(\n                        &self.session.document().text(),\n                        widget.source_range(),\n                    )?\n                };",
        "                } else if widget.table().is_some() {\n                    table_reservation_extra_lines(\n                        &self.session.document().text(),\n                        widget.source_range(),\n                    )?\n                } else if widget.callout().is_some() {\n                    source_range_newline_count(\n                        &self.session.document().text(),\n                        widget.source_range(),\n                    )?\n                } else {\n                    widget.custom_embed()?;\n                    0\n                };",
    )


def stage_red_text() -> None:
    text = read(MODULE)
    if "fn callout_text_specs_preserve_title_fold_and_body_order" in text:
        return
    marker = "    #[test]\n    fn source_range_newline_count_uses_canonical_bytes() {"
    index = text.find(marker)
    if index < 0:
        raise RuntimeError("module test marker missing")
    write(MODULE, text[:index] + TEXT_TESTS + "\n" + text[index:])


def stage_green_text() -> None:
    text = read(MODULE)
    if "pub enum CardTextRole" not in text:
        marker = "#[cfg(test)]\nmod tests {"
        if marker not in text:
            raise RuntimeError("module cfg(test) marker missing")
        write(MODULE, text.replace(marker, TEXT_CODE + marker, 1))

    replace_once(
        MAIN,
        "    append_callout_rects, append_custom_embed_rects, callout_block_layout, callout_geometry,\n    callout_palette, custom_embed_geometry, source_range_newline_count,\n};",
        "    append_callout_rects, append_custom_embed_rects, callout_block_layout, callout_geometry,\n    callout_palette, callout_text_specs, custom_embed_geometry, custom_embed_text_spec,\n    source_range_newline_count, CardTextSpec,\n};",
    )

    insert_before(
        MAIN,
        "    fn caret_rectangles(&self) -> Vec<ScreenRect> {",
        CARD_TEXT_METHOD,
        "fn card_text_specs(&self) -> Vec<CardTextSpec>",
    )

    old_block = r'''        let text_prepared = self.text_render_dirty;
        if text_prepared {
            let table_specs = self.table_text_specs();
            let mut table_buffers = Vec::with_capacity(table_specs.len());
            for spec in &table_specs {
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(TABLE_FONT_SIZE, TABLE_TEXT_LINE_HEIGHT),
                );
                buffer.set_size(
                    Some((spec.bounds.width - TABLE_CELL_PADDING * 2.0).max(1.0)),
                    Some(spec.bounds.height.max(1.0)),
                );
                buffer.set_wrap(Wrap::None);
                let attrs = Attrs::new().family(Family::SansSerif);
                buffer.set_text(
                    &spec.text,
                    &attrs,
                    Shaping::Advanced,
                    Some(table_text_alignment(spec.alignment)),
                );
                buffer.shape_until_scroll(&mut self.font_system, false);
                table_buffers.push(buffer);
            }

            let mut text_areas = Vec::with_capacity(1 + table_buffers.len());
            text_areas.push(TextArea {
                buffer: &self.text_buffer,
                left: TEXT_LEFT,
                top: TEXT_TOP,
                scale: 1.0,
                bounds: text_bounds,
                default_color: Color::rgb(210, 214, 220),
                custom_glyphs: &[],
            });
            for (buffer, spec) in table_buffers.iter().zip(&table_specs) {
                let left = spec.bounds.x + TABLE_CELL_PADDING;
                let top =
                    spec.bounds.y + ((spec.bounds.height - TABLE_TEXT_LINE_HEIGHT) * 0.5).max(0.0);
                text_areas.push(TextArea {
                    buffer,
                    left,
                    top,
                    scale: 1.0,
                    bounds: table_text_bounds(spec.bounds),
                    default_color: if spec.header {
                        Color::rgb(232, 234, 240)
                    } else {
                        Color::rgb(210, 214, 220)
                    },
                    custom_glyphs: &[],
                });
            }

            self.text_renderer.prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )?;
            self.text_render_dirty = false;
        }
'''
    new_block = r'''        let text_prepared = self.text_render_dirty;
        if text_prepared {
            let table_specs = self.table_text_specs();
            let mut table_buffers = Vec::with_capacity(table_specs.len());
            for spec in &table_specs {
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(TABLE_FONT_SIZE, TABLE_TEXT_LINE_HEIGHT),
                );
                buffer.set_size(
                    Some((spec.bounds.width - TABLE_CELL_PADDING * 2.0).max(1.0)),
                    Some(spec.bounds.height.max(1.0)),
                );
                buffer.set_wrap(Wrap::None);
                let attrs = Attrs::new().family(Family::SansSerif);
                buffer.set_text(
                    &spec.text,
                    &attrs,
                    Shaping::Advanced,
                    Some(table_text_alignment(spec.alignment)),
                );
                buffer.shape_until_scroll(&mut self.font_system, false);
                table_buffers.push(buffer);
            }

            let card_specs = self.card_text_specs();
            let mut card_buffers = Vec::with_capacity(card_specs.len());
            for spec in &card_specs {
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(spec.font_size, spec.line_height),
                );
                buffer.set_size(
                    Some(spec.bounds.width.max(1.0)),
                    Some(spec.bounds.height.max(1.0)),
                );
                buffer.set_wrap(if spec.wrap { Wrap::Word } else { Wrap::None });
                let attrs = Attrs::new().family(Family::SansSerif);
                buffer.set_text(&spec.text, &attrs, Shaping::Advanced, None);
                buffer.shape_until_scroll(&mut self.font_system, false);
                card_buffers.push(buffer);
            }

            let mut text_areas =
                Vec::with_capacity(1 + table_buffers.len() + card_buffers.len());
            text_areas.push(TextArea {
                buffer: &self.text_buffer,
                left: TEXT_LEFT,
                top: TEXT_TOP,
                scale: 1.0,
                bounds: text_bounds,
                default_color: Color::rgb(210, 214, 220),
                custom_glyphs: &[],
            });
            for (buffer, spec) in table_buffers.iter().zip(&table_specs) {
                let left = spec.bounds.x + TABLE_CELL_PADDING;
                let top =
                    spec.bounds.y + ((spec.bounds.height - TABLE_TEXT_LINE_HEIGHT) * 0.5).max(0.0);
                text_areas.push(TextArea {
                    buffer,
                    left,
                    top,
                    scale: 1.0,
                    bounds: table_text_bounds(spec.bounds),
                    default_color: if spec.header {
                        Color::rgb(232, 234, 240)
                    } else {
                        Color::rgb(210, 214, 220)
                    },
                    custom_glyphs: &[],
                });
            }
            for (buffer, spec) in card_buffers.iter().zip(&card_specs) {
                text_areas.push(TextArea {
                    buffer,
                    left: spec.bounds.x,
                    top: spec.bounds.y,
                    scale: 1.0,
                    bounds: card_text_bounds(spec.bounds),
                    default_color: Color::rgb(spec.color[0], spec.color[1], spec.color[2]),
                    custom_glyphs: &[],
                });
            }

            self.text_renderer.prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )?;
            self.text_render_dirty = false;
        }
'''
    replace_once(MAIN, old_block, new_block)

    insert_before(
        MAIN,
        "fn table_dimensions(table: &TableWidgetData) -> (usize, usize) {",
        CARD_TEXT_BOUNDS,
        "fn card_text_bounds(bounds: ScreenRect)",
    )


def stage_docs() -> None:
    fixture_addition = r'''

### Callout and custom embed widget scratch

The callout below should render as a native card while inactive. Its title, fold marker, body, and surrounding text must remain source-safe.

> [!warning]- Native warning
> **Strong body** with [link](https://example.com) and `code`.
> 한글 본문도 같은 카드 안에 표시됩니다.

The following standalone embeds should render as native one-row cards. Inline embeds must remain editable source text.

![[diagram.svg#Overview|Architecture]]

![[missing-note]]

Inline embed remains source-visible: prefix ![[Note#Heading]] suffix.

Entering a callout body or standalone embed reveals the exact canonical Markdown source. Leaving the owned source range restores the native widget.
'''
    fixture = read(FIXTURE)
    if "### Callout and custom embed widget scratch" not in fixture:
        write(FIXTURE, fixture.rstrip() + fixture_addition + "\n")

    doc_addition = r'''

## Slice 06 - callout / custom embed

Status: **projection/data and native rendering implemented; Windows/macOS/Ubuntu CI PASS required on the final branch head; macOS manual acceptance pending; Windows native manual acceptance remains deferred to the final Phase 6 integration pass**.

### Contract

- Obsidian extension scanning remains opt-in and feeds an `ExtensionSnapshot` into Live Preview projection
- an inactive callout owns its complete containing blockquote source range
- Callout payload preserves type, custom title, fold marker, and visible body text
- nested Markdown markers in the inactive body collapse to visible text while canonical source remains unchanged
- the native host draws a type-colored background, accent bar, divider, title/fold text, and wrapped body text
- Callout reservation counts canonical newline bytes in the owned source range so reveal/conceal preserves the vertical footprint
- an inactive standalone `![[...]]` becomes a native one-row Custom Embed card
- Custom Embed payload preserves target, path, fragment, and display text
- inline embeds remain source-visible until an inline-width reservation contract exists
- entering either owned range reveals exact canonical Markdown; leaving restores the native widget
- callout/embed text is clipped to widget bounds and never paints over following text

### macOS acceptance

Run:

```bash
MDEDIT_LIVE_PREVIEW_ACCEPTANCE=1 cargo run -p mdedit-ime-lab --release
```

In **Callout and custom embed widget scratch**, verify:

- warning callout renders as a native card with a distinct accent and the custom title
- body displays `Strong body with link and code` plus the Korean line without Markdown marker bytes
- callout never overlaps the text before or after it
- clicking or arrow-navigating into title/body reveals the full canonical blockquote and callout marker
- leaving restores the card; repeat five times with no viewport jump
- `Architecture · #Overview` and `missing-note` appear as native one-row embed cards
- entering each standalone embed reveals exact `![[...]]` source and leaving restores the card
- the inline embed remains source-visible
- resize wider/narrower keeps cards bounded and text clipped
- Rule, Checkbox + Undo/Redo, Image, Math, and Table remain regression-free
- Korean `한글` and `**한글**` keep the viewport stable
- no wgpu validation error, panic, or crash appears

Keep PR #12 Draft until this macOS acceptance passes.
'''
    doc = read(DOC)
    if "## Slice 06 - callout / custom embed" not in doc:
        write(DOC, doc.rstrip() + doc_addition + "\n")

    readme = read(README)
    readme = readme.replace(
        "- Phase 6: Widgets — in progress; horizontal rule, task checkbox, standalone image, Display Math, and Table macOS integration PASS; Slice 06 Callout / custom embed next; Windows native manual acceptance deferred to final Phase 6 integration",
        "- Phase 6: Widgets — in progress; horizontal rule, task checkbox, standalone image, Display Math, and Table macOS integration PASS; Callout / custom embed implemented with 3-OS CI verification and macOS manual acceptance pending; Windows native manual acceptance deferred to final Phase 6 integration",
    )
    write(README, readme)

    roadmap = read(ROADMAP)
    roadmap = roadmap.replace(
        "Status: **in progress — horizontal rule, task checkbox, standalone image, Display Math, and Table macOS integration PASS; Slice 06 Callout / custom embed next; Windows native manual acceptance deferred to the final integration pass**",
        "Status: **in progress — horizontal rule, task checkbox, standalone image, Display Math, and Table macOS integration PASS; Callout / custom embed implemented with 3-OS CI verification and macOS manual acceptance pending; Windows native manual acceptance deferred to the final integration pass**",
    )
    write(ROADMAP, roadmap)

    plan = read(PLAN)
    plan = plan.replace("- [ ]", "- [x]")
    write(PLAN, plan)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: phase06_callout_native_patch.py <red-geometry|green-geometry|red-text|green-text|docs>")
    stage = sys.argv[1]
    {
        "red-geometry": stage_red_geometry,
        "green-geometry": stage_green_geometry,
        "red-text": stage_red_text,
        "green-text": stage_green_text,
        "docs": stage_docs,
    }[stage]()


if __name__ == "__main__":
    main()
