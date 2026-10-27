from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = ROOT / "examples/ime-lab/src/main.rs"


def read() -> str:
    return MAIN.read_text(encoding="utf-8")


def write(text: str) -> None:
    MAIN.write_text(text, encoding="utf-8")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise RuntimeError(f"missing marker: {label}")
    return text.replace(old, new, 1)


def red_callout() -> None:
    text = read()
    sentinel = "fn first_widget_reservation_transition_restores_pre_transition_scroll"
    if sentinel in text:
        return
    marker = '''    #[test]\n    fn ime_viewport_anchor_keeps_exact_scroll_across_temporary_relayout() {'''
    test = '''    #[test]\n    fn first_widget_reservation_transition_restores_pre_transition_scroll() {\n        let before = Scroll::new(101, 21.0, 0.0);\n        let transient = Scroll::new(98, 21.0, 0.0);\n\n        let anchor = widget_reservation_viewport_anchor(before, true)\n            .expect("reservation transition must preserve the pre-transition viewport");\n\n        assert_ne!(transient, before);\n        assert_eq!(anchor.scroll, before);\n        assert!(widget_reservation_viewport_anchor(before, false).is_none());\n    }\n\n'''
    write(replace_once(text, marker, test + marker, "callout viewport test insertion"))


def green_callout() -> None:
    text = read()
    text = replace_once(
        text,
        '''    pending_ime_viewport_anchor: Option<ImeViewportAnchor>,\n    pending_history_viewport_anchor: Option<ImeViewportAnchor>,\n    #[cfg(target_os = "macos")]''',
        '''    pending_ime_viewport_anchor: Option<ImeViewportAnchor>,\n    pending_history_viewport_anchor: Option<ImeViewportAnchor>,\n    pending_widget_viewport_anchor: Option<ImeViewportAnchor>,\n    #[cfg(target_os = "macos")]''',
        "widget viewport field",
    )
    text = replace_once(
        text,
        '''            pending_ime_viewport_anchor: None,\n            pending_history_viewport_anchor: None,\n            #[cfg(target_os = "macos")]''',
        '''            pending_ime_viewport_anchor: None,\n            pending_history_viewport_anchor: None,\n            pending_widget_viewport_anchor: None,\n            #[cfg(target_os = "macos")]''',
        "widget viewport initialization",
    )
    text = replace_once(
        text,
        '''        let ime_viewport_anchor = self.pending_ime_viewport_anchor.take();\n        let history_viewport_anchor = self.pending_history_viewport_anchor.take();\n        let viewport_anchor = ime_viewport_anchor.or(history_viewport_anchor);''',
        '''        let ime_viewport_anchor = self.pending_ime_viewport_anchor.take();\n        let history_viewport_anchor = self.pending_history_viewport_anchor.take();\n        let widget_viewport_anchor = self.pending_widget_viewport_anchor.take();\n        let viewport_anchor = ime_viewport_anchor\n            .or(history_viewport_anchor)\n            .or(widget_viewport_anchor);''',
        "widget viewport consumption",
    )
    text = replace_once(
        text,
        '''        self.refresh_layout();\n        self.sync_image_widgets();\n        self.sync_math_widgets();\n        if self.update_block_widget_reservations() {\n            self.refresh_layout();\n        }''',
        '''        let widget_scroll_before = self.text_buffer.scroll();\n        self.refresh_layout();\n        self.sync_image_widgets();\n        self.sync_math_widgets();\n        let reservations_changed = self.update_block_widget_reservations();\n        if let Some(anchor) =\n            widget_reservation_viewport_anchor(widget_scroll_before, reservations_changed)\n        {\n            self.pending_widget_viewport_anchor = Some(anchor);\n            self.refresh_layout();\n        }''',
        "atomic widget reservation refresh",
    )
    marker = '''fn restore_viewport_anchor(\n    buffer: &mut Buffer,\n    font_system: &mut FontSystem,\n    anchor: ImeViewportAnchor,\n) -> bool {'''
    helper = '''fn widget_reservation_viewport_anchor(\n    scroll: Scroll,\n    reservations_changed: bool,\n) -> Option<ImeViewportAnchor> {\n    reservations_changed.then_some(ImeViewportAnchor { scroll })\n}\n\n'''
    text = replace_once(text, marker, helper + marker, "widget viewport helper")
    write(text)


def red_checkbox() -> None:
    text = read()
    sentinel = "fn task_checkbox_click_padding_toggles_and_round_trips_history"
    if sentinel in text:
        return
    marker = '''    #[test]\n    fn task_checkbox_geometry_stays_in_the_left_gutter_and_is_hittable() {'''
    test = '''    #[test]\n    fn task_checkbox_click_padding_toggles_and_round_trips_history() {\n        let source = "- [ ] unchecked task\\n";\n        let mut session = EditorSession::new(source).unwrap();\n        session.set_caret(Anchor::new(\n            TextSize::try_from_usize(source.len()).unwrap(),\n            Affinity::After,\n        ));\n        let mut live_preview = LivePreviewState::new();\n        live_preview.refresh(&session).unwrap();\n        let widget = live_preview\n            .widgets(&session)\n            .iter()\n            .find(|widget| widget.kind() == WidgetKind::TaskCheckbox)\n            .expect("task checkbox widget")\n            .clone();\n\n        let visual = task_checkbox_rect(0.0, 0.0, LINE_HEIGHT);\n        let hit = task_checkbox_hit_rect(visual);\n        let click_x = visual.x - 3.0;\n        let click_y = visual.y + visual.height * 0.5;\n\n        assert!(!point_in_rect(click_x, click_y, visual));\n        assert!(point_in_rect(click_x, click_y, hit));\n        let action = widget_action_at_pointer(vec![(widget, hit)], click_x, click_y)\n            .expect("padded checkbox target must resolve a widget action");\n        let WidgetAction::SetTaskChecked { marker_range, checked } = action;\n        assert!(checked);\n\n        session\n            .handle(EditorInput::WidgetReplace {\n                range: marker_range,\n                text: action.replacement_text().to_owned(),\n            })\n            .unwrap();\n        assert_eq!(session.document().text(), "- [x] unchecked task\\n");\n\n        session.handle(EditorInput::Undo).unwrap();\n        assert_eq!(session.document().text(), source);\n        session.handle(EditorInput::Redo).unwrap();\n        assert_eq!(session.document().text(), "- [x] unchecked task\\n");\n    }\n\n'''
    write(replace_once(text, marker, test + marker, "checkbox action test insertion"))


def green_checkbox() -> None:
    text = read()
    text = replace_once(
        text,
        '''const TASK_CHECKBOX_GAP: f32 = 5.0;\nconst TASK_CHECKBOX_BORDER: f32 = 1.5;''',
        '''const TASK_CHECKBOX_GAP: f32 = 5.0;\nconst TASK_CHECKBOX_HIT_PADDING: f32 = 5.0;\nconst TASK_CHECKBOX_BORDER: f32 = 1.5;''',
        "checkbox hit padding constant",
    )
    old_region = '''                WidgetKind::TaskCheckbox => {\n                    self.widget_anchor_geometry(widget)\n                        .map(|(anchor_x, line_top, line_height)| {\n                            (\n                                widget.clone(),\n                                task_checkbox_rect(anchor_x, line_top, line_height),\n                            )\n                        })\n                }'''
    new_region = '''                WidgetKind::TaskCheckbox => {\n                    self.widget_anchor_geometry(widget)\n                        .map(|(anchor_x, line_top, line_height)| {\n                            let visual = task_checkbox_rect(anchor_x, line_top, line_height);\n                            (widget.clone(), task_checkbox_hit_rect(visual))\n                        })\n                }'''
    text = replace_once(text, old_region, new_region, "checkbox hit region")
    old_action = '''        let action = self\n            .widget_hit_regions()\n            .into_iter()\n            .find(|(_, rect)| point_in_rect(pointer_x, pointer_y, *rect))\n            .and_then(|(widget, _)| widget.primary_action());'''
    new_action = '''        let action = widget_action_at_pointer(\n            self.widget_hit_regions(),\n            pointer_x,\n            pointer_y,\n        );'''
    text = replace_once(text, old_action, new_action, "checkbox action resolver")
    marker = '''fn append_task_checkbox_rects(output: &mut Vec<ScreenRect>, bounds: ScreenRect, checked: bool) {'''
    helper = '''fn task_checkbox_hit_rect(visual: ScreenRect) -> ScreenRect {\n    let left = (visual.x - TASK_CHECKBOX_HIT_PADDING).max(0.0);\n    let top = (visual.y - TASK_CHECKBOX_HIT_PADDING).max(0.0);\n    let right = visual.x + visual.width + TASK_CHECKBOX_HIT_PADDING;\n    let bottom = visual.y + visual.height + TASK_CHECKBOX_HIT_PADDING;\n    ScreenRect::new(\n        left,\n        top,\n        (right - left).max(visual.width),\n        (bottom - top).max(visual.height),\n        [0.0; 4],\n    )\n}\n\n'''
    text = replace_once(text, marker, helper + marker, "checkbox hit rect helper")
    marker = '''fn visual_horizontal_target(\n    buffer: &mut Buffer,'''
    helper = '''fn widget_action_at_pointer(\n    regions: impl IntoIterator<Item = (ProjectedWidget, ScreenRect)>,\n    pointer_x: f32,\n    pointer_y: f32,\n) -> Option<WidgetAction> {\n    regions\n        .into_iter()\n        .find(|(_, rect)| point_in_rect(pointer_x, pointer_y, *rect))\n        .and_then(|(widget, _)| widget.primary_action())\n}\n\n'''
    text = replace_once(text, marker, helper + marker, "widget action helper")
    write(text)


MODES = {
    "red-callout": red_callout,
    "green-callout": green_callout,
    "red-checkbox": red_checkbox,
    "green-checkbox": green_checkbox,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in MODES:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} <{'|'.join(MODES)}>")
    MODES[sys.argv[1]]()
