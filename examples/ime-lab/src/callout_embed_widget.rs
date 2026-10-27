use crate::geometry::ScreenRect;
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
