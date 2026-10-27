from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = ROOT / "examples/ime-lab/src/main.rs"
MODULE = ROOT / "examples/ime-lab/src/callout_embed_widget.rs"

module = MODULE.read_text(encoding="utf-8")
module = module.replace(
    ".split(|character: char| matches!(character, '-' | '_' | ' '))",
    ".split(['-', '_', ' '])",
)
MODULE.write_text(module, encoding="utf-8")

main = MAIN.read_text(encoding="utf-8")
main = main.replace(
    "    source_range_newline_count, CardTextSpec,\n};",
    "    source_range_newline_count, CardTextRole, CardTextSpec,\n};",
)
main = main.replace(
    "                let attrs = Attrs::new().family(Family::SansSerif);\n                buffer.set_text(&spec.text, &attrs, Shaping::Advanced, None);\n                buffer.shape_until_scroll(&mut self.font_system, false);\n                card_buffers.push(buffer);",
    "                let attrs = Attrs::new().family(Family::SansSerif);\n                let alignment = match spec.role {\n                    CardTextRole::CalloutFold => Align::Center,\n                    CardTextRole::CalloutTitle\n                    | CardTextRole::CalloutBody\n                    | CardTextRole::EmbedLabel => Align::Left,\n                };\n                buffer.set_text(\n                    &spec.text,\n                    &attrs,\n                    Shaping::Advanced,\n                    Some(alignment),\n                );\n                buffer.shape_until_scroll(&mut self.font_system, false);\n                card_buffers.push(buffer);",
)
MAIN.write_text(main, encoding="utf-8")
