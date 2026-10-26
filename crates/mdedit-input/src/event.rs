use std::ops::Range;

use mdedit_core::{DeleteDirection, Movement, SelectionSet, TextRange};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorInput {
    InsertText(String),
    WidgetReplace {
        range: TextRange,
        text: String,
    },
    ImeEnabled,
    ImePreedit {
        text: String,
        selection: Option<Range<usize>>,
    },
    ImeCommit(String),
    ImeDisabled,
    Move {
        movement: Movement,
        extend: bool,
    },
    Delete(DeleteDirection),
    SetSelection(SelectionSet),
    Undo,
    Redo,
    Focused(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformRequest {
    RequestRedraw,
    SetImeAllowed(bool),
    SetImeCursorArea(Rect),
    ClipboardWrite(String),
}
