use crate::geometry::ScreenRect;
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
    let y =
        text_top + line_top + ((reserved_height - layout.height) * 0.5).max(CALLOUT_VERTICAL_INSET);
    let block = ScreenRect::new(x, y, layout.width, layout.height, [0.0; 4]);
    let accent = ScreenRect::new(x, y, CALLOUT_ACCENT_WIDTH, layout.height, [0.0; 4]);
    let fold_width = CALLOUT_FOLD_WIDTH.min((layout.width * 0.16).max(12.0));
    let header_x = x + CALLOUT_ACCENT_WIDTH + CALLOUT_HORIZONTAL_PADDING;
    let header_width =
        (layout.width - CALLOUT_ACCENT_WIDTH - CALLOUT_HORIZONTAL_PADDING * 2.0 - fold_width)
            .max(1.0);
    let header = ScreenRect::new(header_x, y, header_width, layout.header_height, [0.0; 4]);
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
        .split(['-', '_', ' '])
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

    detail.map_or_else(
        || primary.to_owned(),
        |detail| format!("{primary}  ·  {detail}"),
    )
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
        assert!(
            geometry.body.y + geometry.body.height
                <= geometry.block.y + geometry.block.height + 0.01
        );
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
        assert!(
            geometry.text.x + geometry.text.width <= geometry.block.x + geometry.block.width + 0.01
        );
    }

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
        assert_eq!(
            callout_display_title("custom-warning", ""),
            "Custom Warning"
        );
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
        assert_eq!(
            custom_embed_label("Note#Heading", "", None, None),
            "Note#Heading"
        );
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
        assert!(
            spec.bounds.x + spec.bounds.width <= geometry.block.x + geometry.block.width + 0.01
        );
        assert!(!spec.wrap);
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
