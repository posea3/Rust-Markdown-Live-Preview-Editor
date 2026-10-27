from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "examples/ime-lab/src/callout_embed_widget.rs"


def read() -> str:
    return MODULE.read_text(encoding="utf-8")


def write(text: str) -> None:
    MODULE.write_text(text, encoding="utf-8")


def red() -> None:
    text = read()
    sentinel = "fn single_line_callout_without_trailing_newline_fits_one_layout_row"
    if sentinel in text:
        return
    marker = "    #[test]\n    fn callout_palette_distinguishes_warning_from_note() {"
    test = '''    #[test]\n    fn single_line_callout_without_trailing_newline_fits_one_layout_row() {\n        let layout = callout_block_layout(1, 640.0, 31.0);\n        let geometry = callout_geometry(24.0, 24.0, 0.0, 31.0, 640.0, layout);\n\n        assert_eq!(layout.rows, 1);\n        assert!(geometry.block.y + geometry.block.height <= 24.0 + 31.0 + 0.01);\n        assert!(geometry.body.height <= 0.01);\n    }\n\n'''
    if marker not in text:
        raise RuntimeError("callout test insertion marker missing")
    write(text.replace(marker, test + marker, 1))


def green() -> None:
    text = read()
    old_rows = "    let rows = reserved_rows.max(2);"
    new_rows = "    let rows = reserved_rows.max(1);"
    old_height = (
        "    let height = (reserved_height - CALLOUT_VERTICAL_INSET * 2.0).max(line_height);"
    )
    new_height = (
        "    let height = (reserved_height - CALLOUT_VERTICAL_INSET * 2.0).max(1.0);"
    )
    old_body = "        (y + layout.height - body_y).max(1.0),"
    new_body = "        (y + layout.height - body_y).max(0.0),"
    if old_rows in text:
        text = text.replace(old_rows, new_rows, 1)
    if old_height in text:
        text = text.replace(old_height, new_height, 1)
    if old_body in text:
        text = text.replace(old_body, new_body, 1)
    if new_rows not in text or new_height not in text or new_body not in text:
        raise RuntimeError("single-line callout production markers missing")
    write(text)


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in {"red", "green"}:
        raise SystemExit("usage: phase06_callout_single_line_fix.py <red|green>")
    {"red": red, "green": green}[sys.argv[1]]()
