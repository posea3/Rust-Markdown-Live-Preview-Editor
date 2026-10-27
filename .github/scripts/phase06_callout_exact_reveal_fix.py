from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROJECTION = ROOT / "crates/mdedit-live/src/projection.rs"


def read() -> str:
    return PROJECTION.read_text(encoding="utf-8")


def write(text: str) -> None:
    PROJECTION.write_text(text, encoding="utf-8")


def red() -> None:
    text = read()
    sentinel = "fn callout_source_reveal_restores_nested_markers_and_widgets"
    if sentinel in text:
        return
    marker = "    #[test]\n    fn inactive_obsidian_embed_becomes_custom_embed_widget() {"
    test = '''    #[test]\n    fn callout_source_reveal_restores_nested_markers_and_widgets() {\n        let source = concat!(\n            \"> [!note] Title\\n\",\n            \"> - [ ] **Task**\\n\",\n        );\n        let marker = source.find(\"Title\").unwrap();\n        let projection = build_with_obsidian_extensions(\n            source,\n            RevealPolicy::ConcealInactive,\n            &RevealContext::new().with_caret(TextSize::try_from_usize(marker).unwrap()),\n        );\n\n        assert_eq!(\n            projection.text(),\n            source,\n            \"activating the callout must reveal nested delimiter, quote, list, and task markers across the owned source range\",\n        );\n        assert!(projection.widgets().iter().all(|widget| {\n            !matches!(widget.kind(), WidgetKind::Callout | WidgetKind::TaskCheckbox)\n        }));\n    }\n\n'''
    if marker not in text:
        raise RuntimeError("callout reveal test insertion marker missing")
    write(text.replace(marker, test + marker, 1))


def green() -> None:
    text = read()
    text = text.replace(
        "!shadowed_by_inactive_widget(&widget_candidates, span.source_range)",
        "!shadowed_by_widget_owner(&widget_candidates, span.source_range)",
    )

    old_shadow = '''fn shadowed_by_inactive_widget(widgets: &[WidgetCandidate], range: TextRange) -> bool {\n    widgets\n        .iter()\n        .any(|widget| !widget.active && range_contains(widget.conceal_range, range))\n}\n'''
    new_shadow = '''fn shadowed_by_widget_owner(widgets: &[WidgetCandidate], range: TextRange) -> bool {\n    widgets.iter().any(|widget| {\n        (!widget.active && range_contains(widget.conceal_range, range))\n            || (matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed)\n                && range_contains(widget.source_range, range))\n    })\n}\n'''
    if old_shadow in text:
        text = text.replace(old_shadow, new_shadow, 1)

    old_normalize = '''fn normalize_widget_candidate_ownership(mut widgets: Vec<WidgetCandidate>) -> Vec<WidgetCandidate> {\n    let inactive_extension_owners = widgets\n        .iter()\n        .filter(|widget| {\n            !widget.active && matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed)\n        })\n        .map(|widget| (widget.kind, widget.source_range))\n        .collect::<Vec<_>>();\n\n    widgets.retain(|widget| {\n        if matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed) {\n            return true;\n        }\n\n        !inactive_extension_owners\n            .iter()\n            .any(|(_, range)| range_contains(*range, widget.source_range))\n    });\n'''
    new_normalize = '''fn normalize_widget_candidate_ownership(mut widgets: Vec<WidgetCandidate>) -> Vec<WidgetCandidate> {\n    let extension_owner_ranges = widgets\n        .iter()\n        .filter(|widget| matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed))\n        .map(|widget| widget.source_range)\n        .collect::<Vec<_>>();\n\n    widgets.retain(|widget| {\n        if matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed) {\n            return true;\n        }\n\n        !extension_owner_ranges\n            .iter()\n            .any(|range| range_contains(*range, widget.source_range))\n    });\n'''
    if old_normalize in text:
        text = text.replace(old_normalize, new_normalize, 1)

    required = [
        "fn shadowed_by_widget_owner",
        "let extension_owner_ranges",
        "!shadowed_by_widget_owner(&widget_candidates, span.source_range)",
    ]
    missing = [item for item in required if item not in text]
    if missing:
        raise RuntimeError(f"callout exact reveal production markers missing: {missing}")
    write(text)


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in {"red", "green"}:
        raise SystemExit("usage: phase06_callout_exact_reveal_fix.py <red|green>")
    {"red": red, "green": green}[sys.argv[1]]()
