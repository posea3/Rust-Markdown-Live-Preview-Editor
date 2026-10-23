use std::ops::Range;

use accesskit::{
    Action, ActionData, ActionRequest, Affine, Node, NodeId, Rect, Role, TextPosition,
    TextSelection, TreeId, TreeInfo, TreeUpdate,
};
use mdedit_core::{Affinity, Anchor, SelectionRange, TextSize};
use unicode_segmentation::UnicodeSegmentation;

const WINDOW_ID: NodeId = NodeId(0);
const EDITOR_ID: NodeId = NodeId(1);
const TEXT_RUN_ID: NodeId = NodeId(2);

#[derive(Debug, PartialEq)]
pub enum EditorAccessibilityAction {
    Focus,
    SetSelection(SelectionRange),
    ReplaceSelectedText(String),
    SetValue(String),
}

pub fn build_tree_update(
    text: &str,
    selection: SelectionRange,
    width: u32,
    height: u32,
    scale_factor: f64,
    text_left: f32,
    text_top: f32,
) -> TreeUpdate {
    let scale_factor = scale_factor.max(f64::EPSILON);
    let logical_width = f64::from(width) / scale_factor;
    let logical_height = f64::from(height) / scale_factor;
    let logical_left = f64::from(text_left) / scale_factor;
    let logical_top = f64::from(text_top) / scale_factor;

    let root_bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: logical_width,
        y1: logical_height,
    };
    let editor_bounds = Rect {
        x0: logical_left,
        y0: logical_top,
        x1: (logical_width - logical_left).max(logical_left),
        y1: (logical_height - logical_top).max(logical_top),
    };

    let units = text_units(text);
    let mut root = Node::new(Role::Window);
    root.set_label("mdedit IME Lab");
    root.set_bounds(root_bounds);
    root.set_transform(Affine::scale(scale_factor));
    root.set_children(vec![EDITOR_ID]);

    let mut editor = Node::new(Role::MultilineTextInput);
    editor.set_label("Markdown source editor");
    editor.set_bounds(editor_bounds);
    editor.set_children(vec![TEXT_RUN_ID]);
    editor.add_action(Action::Focus);
    editor.add_action(Action::ReplaceSelectedText);
    editor.add_action(Action::SetTextSelection);
    editor.add_action(Action::SetValue);
    editor.set_text_selection(TextSelection {
        anchor: text_position(&units, text.len(), selection.anchor),
        focus: text_position(&units, text.len(), selection.head),
    });

    let mut text_run = Node::new(Role::TextRun);
    text_run.set_bounds(editor_bounds);
    text_run.set_value(text);
    text_run.set_character_lengths(
        units
            .iter()
            .map(|unit| {
                u8::try_from(unit.len())
                    .expect("accessibility text unit is split to at most u8::MAX bytes")
            })
            .collect::<Vec<_>>(),
    );

    TreeUpdate {
        nodes: vec![
            (WINDOW_ID, root),
            (EDITOR_ID, editor),
            (TEXT_RUN_ID, text_run),
        ],
        tree: Some(TreeInfo::new(WINDOW_ID)),
        tree_id: TreeId::ROOT,
        focus: EDITOR_ID,
    }
}

pub fn translate_action(request: ActionRequest, text: &str) -> Option<EditorAccessibilityAction> {
    if request.target_tree != TreeId::ROOT || request.target_node != EDITOR_ID {
        return None;
    }

    match (request.action, request.data) {
        (Action::Focus, _) => Some(EditorAccessibilityAction::Focus),
        (Action::SetTextSelection, Some(ActionData::SetTextSelection(selection))) => {
            let units = text_units(text);
            let anchor = anchor_from_position(&units, text.len(), selection.anchor)?;
            let head = anchor_from_position(&units, text.len(), selection.focus)?;
            Some(EditorAccessibilityAction::SetSelection(SelectionRange {
                anchor,
                head,
            }))
        }
        (Action::ReplaceSelectedText, Some(ActionData::Value(value))) => Some(
            EditorAccessibilityAction::ReplaceSelectedText(value.into_string()),
        ),
        (Action::SetValue, Some(ActionData::Value(value))) => {
            Some(EditorAccessibilityAction::SetValue(value.into_string()))
        }
        _ => None,
    }
}

fn text_units(text: &str) -> Vec<Range<usize>> {
    let mut units = Vec::new();

    for (start, grapheme) in text.grapheme_indices(true) {
        let end = start + grapheme.len();
        if grapheme.len() <= usize::from(u8::MAX) {
            units.push(start..end);
            continue;
        }

        for (relative_start, character) in grapheme.char_indices() {
            let character_start = start + relative_start;
            units.push(character_start..character_start + character.len_utf8());
        }
    }

    units
}

fn text_position(units: &[Range<usize>], text_len: usize, anchor: Anchor) -> TextPosition {
    TextPosition {
        node: TEXT_RUN_ID,
        character_index: character_index_for_anchor(units, text_len, anchor),
    }
}

fn character_index_for_anchor(units: &[Range<usize>], text_len: usize, anchor: Anchor) -> usize {
    let offset = anchor.offset.to_usize().min(text_len);

    for (index, unit) in units.iter().enumerate() {
        if offset <= unit.start {
            return index;
        }
        if offset < unit.end {
            return if anchor.affinity == Affinity::After {
                index + 1
            } else {
                index
            };
        }
    }

    units.len()
}

fn anchor_from_position(
    units: &[Range<usize>],
    text_len: usize,
    position: TextPosition,
) -> Option<Anchor> {
    if position.node != TEXT_RUN_ID || position.character_index > units.len() {
        return None;
    }

    let offset = if position.character_index == units.len() {
        text_len
    } else {
        units[position.character_index].start
    };
    let offset = TextSize::try_from_usize(offset).ok()?;
    Some(Anchor::new(offset, Affinity::After))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_lengths_follow_grapheme_selection_units() {
        let text = "a\r\n한e\u{301}👨‍👩‍👧‍👦";
        let units = text_units(text);
        let lengths = units.iter().map(|unit| unit.len()).collect::<Vec<_>>();

        assert_eq!(lengths.iter().sum::<usize>(), text.len());
        assert_eq!(units.len(), 5);
        assert_eq!(&text[units[1].clone()], "\r\n");
        assert_eq!(&text[units[2].clone()], "한");
        assert_eq!(&text[units[3].clone()], "e\u{301}");
        assert_eq!(&text[units[4].clone()], "👨‍👩‍👧‍👦");
    }

    #[test]
    fn selection_positions_round_trip_grapheme_boundaries() {
        let text = "a한👨‍👩‍👧‍👦z";
        let units = text_units(text);
        let boundary = units[2].start;
        let anchor = Anchor::new(TextSize::try_from_usize(boundary).unwrap(), Affinity::After);
        let position = text_position(&units, text.len(), anchor);
        let mapped = anchor_from_position(&units, text.len(), position).unwrap();

        assert_eq!(mapped.offset, anchor.offset);
    }

    #[test]
    fn invalid_text_run_position_is_rejected() {
        let text = "abc";
        let units = text_units(text);
        let invalid = TextPosition {
            node: NodeId(99),
            character_index: 1,
        };

        assert!(anchor_from_position(&units, text.len(), invalid).is_none());
    }
}
