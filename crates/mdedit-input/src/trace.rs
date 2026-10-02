use std::{fmt::Write as _, ops::Range};

use mdedit_core::{
    Affinity, Anchor, DeleteDirection, Movement, SelectionRange, SelectionSet, TextSize,
};
use thiserror::Error;

use crate::{EditorInput, EditorSession, SessionError};

pub const EDITOR_TRACE_VERSION: &str = "mdedit-editor-trace-v1";

#[derive(Clone, Debug, PartialEq)]
pub struct EditorTrace {
    initial_source: String,
    initial_selection: SelectionSet,
    events: Vec<EditorInput>,
}

impl EditorTrace {
    #[must_use]
    pub fn new(initial_source: String, initial_selection: SelectionSet) -> Self {
        Self {
            initial_source,
            initial_selection,
            events: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_session(session: &EditorSession) -> Self {
        Self::new(
            session.document().text(),
            session.selections().clone(),
        )
    }

    pub fn push(&mut self, input: EditorInput) {
        self.events.push(input);
    }

    #[must_use]
    pub fn events(&self) -> &[EditorInput] {
        &self.events
    }

    #[must_use]
    pub fn encode(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "{EDITOR_TRACE_VERSION}").expect("write to String");
        writeln!(&mut output, "start-source\t{}", encode_hex(&self.initial_source))
            .expect("write to String");
        writeln!(
            &mut output,
            "start-selection\t{}",
            encode_selection(&self.initial_selection)
        )
        .expect("write to String");

        for event in &self.events {
            writeln!(&mut output, "event\t{}", encode_input(event)).expect("write to String");
        }

        output
    }

    pub fn decode(trace: &str) -> Result<Self, TraceError> {
        let mut lines = trace
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty() && !line.starts_with('#'));

        let version = lines.next().ok_or(TraceError::MissingVersion)?;
        if version != EDITOR_TRACE_VERSION {
            return Err(TraceError::UnsupportedVersion(version.to_owned()));
        }

        let source_line = lines.next().ok_or(TraceError::MissingStartSource)?;
        let source = source_line
            .strip_prefix("start-source\t")
            .ok_or_else(|| TraceError::InvalidLine(source_line.to_owned()))?;
        let initial_source = decode_hex(source)?;

        let selection_line = lines.next().ok_or(TraceError::MissingStartSelection)?;
        let selection = selection_line
            .strip_prefix("start-selection\t")
            .ok_or_else(|| TraceError::InvalidLine(selection_line.to_owned()))?;
        let initial_selection = decode_selection(selection)?;

        let mut events = Vec::new();
        for line in lines {
            let event = line
                .strip_prefix("event\t")
                .ok_or_else(|| TraceError::InvalidLine(line.to_owned()))?;
            events.push(decode_input(event)?);
        }

        Ok(Self {
            initial_source,
            initial_selection,
            events,
        })
    }

    pub fn replay(&self) -> Result<EditorSession, TraceError> {
        let mut session = EditorSession::new(&self.initial_source)?;
        session.set_selection(self.initial_selection.clone());

        for event in &self.events {
            session.handle(event.clone())?;
        }

        Ok(session)
    }
}

fn encode_input(input: &EditorInput) -> String {
    match input {
        EditorInput::InsertText(text) => format!("insert\t{}", encode_hex(text)),
        EditorInput::ImeEnabled => "ime-enabled".to_owned(),
        EditorInput::ImePreedit { text, selection } => format!(
            "ime-preedit\t{}\t{}",
            encode_hex(text),
            encode_optional_range(selection.as_ref())
        ),
        EditorInput::ImeCommit(text) => format!("ime-commit\t{}", encode_hex(text)),
        EditorInput::ImeDisabled => "ime-disabled".to_owned(),
        EditorInput::Move { movement, extend } => format!(
            "move\t{}\t{}",
            encode_movement(*movement),
            encode_bool(*extend)
        ),
        EditorInput::Delete(direction) => format!("delete\t{}", encode_delete(*direction)),
        EditorInput::SetSelection(selection) => {
            format!("set-selection\t{}", encode_selection(selection))
        }
        EditorInput::Undo => "undo".to_owned(),
        EditorInput::Redo => "redo".to_owned(),
        EditorInput::Focused(focused) => format!("focused\t{}", encode_bool(*focused)),
    }
}

fn decode_input(line: &str) -> Result<EditorInput, TraceError> {
    let mut fields = line.split('\t');
    let kind = fields
        .next()
        .ok_or_else(|| TraceError::InvalidLine(line.to_owned()))?;

    let input = match kind {
        "insert" => EditorInput::InsertText(decode_hex(required_field(&mut fields, line)?)?),
        "ime-enabled" => EditorInput::ImeEnabled,
        "ime-preedit" => EditorInput::ImePreedit {
            text: decode_hex(required_field(&mut fields, line)?)?,
            selection: decode_optional_range(required_field(&mut fields, line)?)?,
        },
        "ime-commit" => EditorInput::ImeCommit(decode_hex(required_field(&mut fields, line)?)?),
        "ime-disabled" => EditorInput::ImeDisabled,
        "move" => EditorInput::Move {
            movement: decode_movement(required_field(&mut fields, line)?)?,
            extend: decode_bool(required_field(&mut fields, line)?)?,
        },
        "delete" => EditorInput::Delete(decode_delete(required_field(&mut fields, line)?)?),
        "set-selection" => {
            EditorInput::SetSelection(decode_selection(required_field(&mut fields, line)?)?)
        }
        "undo" => EditorInput::Undo,
        "redo" => EditorInput::Redo,
        "focused" => EditorInput::Focused(decode_bool(required_field(&mut fields, line)?)?),
        _ => return Err(TraceError::InvalidLine(line.to_owned())),
    };

    if fields.next().is_some() {
        return Err(TraceError::InvalidLine(line.to_owned()));
    }

    Ok(input)
}

fn required_field<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    line: &str,
) -> Result<&'a str, TraceError> {
    fields
        .next()
        .ok_or_else(|| TraceError::InvalidLine(line.to_owned()))
}

fn encode_selection(selection: &SelectionSet) -> String {
    let ranges = selection
        .ranges()
        .iter()
        .map(|range| format!("{},{}", encode_anchor(range.anchor), encode_anchor(range.head)))
        .collect::<Vec<_>>()
        .join(";");

    format!("{}|{ranges}", selection.primary_index())
}

fn decode_selection(encoded: &str) -> Result<SelectionSet, TraceError> {
    let (primary, ranges) = encoded
        .split_once('|')
        .ok_or_else(|| TraceError::InvalidSelection(encoded.to_owned()))?;
    let primary = primary
        .parse::<usize>()
        .map_err(|_| TraceError::InvalidSelection(encoded.to_owned()))?;

    let ranges = ranges
        .split(';')
        .map(|range| {
            let (anchor, head) = range
                .split_once(',')
                .ok_or_else(|| TraceError::InvalidSelection(encoded.to_owned()))?;
            Ok(SelectionRange {
                anchor: decode_anchor(anchor)?,
                head: decode_anchor(head)?,
            })
        })
        .collect::<Result<Vec<_>, TraceError>>()?;

    SelectionSet::new(ranges, primary).map_err(|_| TraceError::InvalidSelection(encoded.to_owned()))
}

fn encode_anchor(anchor: Anchor) -> String {
    let affinity = match anchor.affinity {
        Affinity::Before => 'b',
        Affinity::After => 'a',
    };
    format!("{}{affinity}", anchor.offset.get())
}

fn decode_anchor(encoded: &str) -> Result<Anchor, TraceError> {
    let split = encoded
        .len()
        .checked_sub(1)
        .ok_or_else(|| TraceError::InvalidAnchor(encoded.to_owned()))?;
    let (offset, affinity) = encoded.split_at(split);
    let offset = offset
        .parse::<u32>()
        .map_err(|_| TraceError::InvalidAnchor(encoded.to_owned()))?;
    let affinity = match affinity {
        "b" => Affinity::Before,
        "a" => Affinity::After,
        _ => return Err(TraceError::InvalidAnchor(encoded.to_owned())),
    };
    Ok(Anchor::new(TextSize::new(offset), affinity))
}

fn encode_optional_range(range: Option<&Range<usize>>) -> String {
    range.map_or_else(
        || "-".to_owned(),
        |range| format!("{}:{}", range.start, range.end),
    )
}

fn decode_optional_range(encoded: &str) -> Result<Option<Range<usize>>, TraceError> {
    if encoded == "-" {
        return Ok(None);
    }

    let (start, end) = encoded
        .split_once(':')
        .ok_or_else(|| TraceError::InvalidRange(encoded.to_owned()))?;
    let start = start
        .parse::<usize>()
        .map_err(|_| TraceError::InvalidRange(encoded.to_owned()))?;
    let end = end
        .parse::<usize>()
        .map_err(|_| TraceError::InvalidRange(encoded.to_owned()))?;
    Ok(Some(start..end))
}

const fn encode_movement(movement: Movement) -> &'static str {
    match movement {
        Movement::GraphemeBackward => "grapheme-backward",
        Movement::GraphemeForward => "grapheme-forward",
        Movement::WordBackward => "word-backward",
        Movement::WordForward => "word-forward",
        Movement::LineStart => "line-start",
        Movement::LineEnd => "line-end",
        Movement::DocumentStart => "document-start",
        Movement::DocumentEnd => "document-end",
    }
}

fn decode_movement(encoded: &str) -> Result<Movement, TraceError> {
    match encoded {
        "grapheme-backward" => Ok(Movement::GraphemeBackward),
        "grapheme-forward" => Ok(Movement::GraphemeForward),
        "word-backward" => Ok(Movement::WordBackward),
        "word-forward" => Ok(Movement::WordForward),
        "line-start" => Ok(Movement::LineStart),
        "line-end" => Ok(Movement::LineEnd),
        "document-start" => Ok(Movement::DocumentStart),
        "document-end" => Ok(Movement::DocumentEnd),
        _ => Err(TraceError::InvalidMovement(encoded.to_owned())),
    }
}

const fn encode_delete(direction: DeleteDirection) -> &'static str {
    match direction {
        DeleteDirection::Backward => "backward",
        DeleteDirection::Forward => "forward",
    }
}

fn decode_delete(encoded: &str) -> Result<DeleteDirection, TraceError> {
    match encoded {
        "backward" => Ok(DeleteDirection::Backward),
        "forward" => Ok(DeleteDirection::Forward),
        _ => Err(TraceError::InvalidDelete(encoded.to_owned())),
    }
}

const fn encode_bool(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn decode_bool(encoded: &str) -> Result<bool, TraceError> {
    match encoded {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(TraceError::InvalidBool(encoded.to_owned())),
    }
}

fn encode_hex(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len() * 2);
    for byte in text.as_bytes() {
        write!(&mut encoded, "{byte:02x}").expect("write to String");
    }
    encoded
}

fn decode_hex(encoded: &str) -> Result<String, TraceError> {
    if !encoded.len().is_multiple_of(2) {
        return Err(TraceError::InvalidHex(encoded.to_owned()));
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for chunk in encoded.as_bytes().chunks_exact(2) {
        let pair = std::str::from_utf8(chunk)
            .map_err(|_| TraceError::InvalidHex(encoded.to_owned()))?;
        let byte =
            u8::from_str_radix(pair, 16).map_err(|_| TraceError::InvalidHex(encoded.to_owned()))?;
        bytes.push(byte);
    }

    String::from_utf8(bytes).map_err(|_| TraceError::InvalidUtf8)
}

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("trace is missing its format version")]
    MissingVersion,

    #[error("unsupported editor trace version: {0}")]
    UnsupportedVersion(String),

    #[error("trace is missing its initial source")]
    MissingStartSource,

    #[error("trace is missing its initial selection")]
    MissingStartSelection,

    #[error("invalid trace line: {0}")]
    InvalidLine(String),

    #[error("invalid selection encoding: {0}")]
    InvalidSelection(String),

    #[error("invalid anchor encoding: {0}")]
    InvalidAnchor(String),

    #[error("invalid range encoding: {0}")]
    InvalidRange(String),

    #[error("invalid movement encoding: {0}")]
    InvalidMovement(String),

    #[error("invalid delete encoding: {0}")]
    InvalidDelete(String),

    #[error("invalid boolean encoding: {0}")]
    InvalidBool(String),

    #[error("invalid hexadecimal UTF-8 payload: {0}")]
    InvalidHex(String),

    #[error("trace payload is not valid UTF-8")]
    InvalidUtf8,

    #[error(transparent)]
    Session(#[from] SessionError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection() -> SelectionSet {
        SelectionSet::new(
            vec![SelectionRange {
                anchor: Anchor::new(TextSize::new(1), Affinity::Before),
                head: Anchor::new(TextSize::new(4), Affinity::After),
            }],
            0,
        )
        .unwrap()
    }

    #[test]
    fn trace_round_trips_all_input_kinds() {
        let mut trace = EditorTrace::new("a한🙂z".to_owned(), selection());
        for input in [
            EditorInput::InsertText("\t\n한🙂".to_owned()),
            EditorInput::ImeEnabled,
            EditorInput::ImePreedit {
                text: "かな".to_owned(),
                selection: Some(3..6),
            },
            EditorInput::ImeCommit("仮名".to_owned()),
            EditorInput::ImeDisabled,
            EditorInput::Move {
                movement: Movement::WordBackward,
                extend: true,
            },
            EditorInput::Delete(DeleteDirection::Forward),
            EditorInput::SetSelection(selection()),
            EditorInput::Undo,
            EditorInput::Redo,
            EditorInput::Focused(false),
        ] {
            trace.push(input);
        }

        let encoded = trace.encode();
        let decoded = EditorTrace::decode(&encoded).unwrap();

        assert_eq!(decoded, trace);
    }

    #[test]
    fn korean_empty_preedit_sequence_replays_as_one_commit() {
        let mut trace = EditorTrace::new(String::new(), SelectionSet::default());
        trace.push(EditorInput::ImeEnabled);
        trace.push(EditorInput::ImePreedit {
            text: "ㅎ".to_owned(),
            selection: Some(3..3),
        });
        trace.push(EditorInput::ImePreedit {
            text: "하".to_owned(),
            selection: Some(3..3),
        });
        trace.push(EditorInput::ImePreedit {
            text: "한".to_owned(),
            selection: Some(3..3),
        });
        trace.push(EditorInput::ImePreedit {
            text: String::new(),
            selection: None,
        });
        trace.push(EditorInput::ImeCommit("한".to_owned()));

        let mut session = EditorTrace::decode(&trace.encode()).unwrap().replay().unwrap();

        assert_eq!(session.document().text(), "한");
        assert!(session.composition().is_none());
        session.undo().unwrap();
        assert_eq!(session.document().text(), "");
    }

    #[test]
    fn focus_loss_trace_discards_preedit_without_mutating_source() {
        let mut session = EditorSession::new("abc").unwrap();
        session.set_caret(Anchor::new(TextSize::new(1), Affinity::After));
        let mut trace = EditorTrace::from_session(&session);
        trace.push(EditorInput::ImeEnabled);
        trace.push(EditorInput::ImePreedit {
            text: "한".to_owned(),
            selection: Some(3..3),
        });
        trace.push(EditorInput::Focused(false));

        let session = EditorTrace::decode(&trace.encode()).unwrap().replay().unwrap();

        assert_eq!(session.document().text(), "abc");
        assert!(session.composition().is_none());
        assert!(!session.focused());
    }
}
