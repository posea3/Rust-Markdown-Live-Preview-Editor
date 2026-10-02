use std::{error::Error, fmt};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};

use crate::{SyntaxKind, SyntaxNode, SyntaxSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DelimiterKind {
    HeadingPrefix,
    HeadingSuffix,
    SetextUnderline,
    EmphasisOpen,
    EmphasisClose,
    StrongOpen,
    StrongClose,
    StrikethroughOpen,
    StrikethroughClose,
    InlineCodeOpen,
    InlineCodeClose,
    BlockQuoteMarker,
    ListMarker,
    LinkLabelOpen,
    LinkLabelClose,
    LinkDestination,
    AutolinkOpen,
    AutolinkClose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DelimiterSpan {
    kind: DelimiterKind,
    range: TextRange,
    owner: SyntaxKind,
}

impl DelimiterSpan {
    #[must_use]
    pub const fn kind(&self) -> DelimiterKind {
        self.kind
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub const fn owner(&self) -> SyntaxKind {
        self.owner
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelimiterIssueReason {
    MissingContentBoundary,
    UnexpectedSourceForm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DelimiterIssue {
    owner: SyntaxKind,
    range: TextRange,
    reason: DelimiterIssueReason,
}

impl DelimiterIssue {
    #[must_use]
    pub const fn owner(&self) -> SyntaxKind {
        self.owner
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub const fn reason(&self) -> DelimiterIssueReason {
        self.reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelimiterSnapshot {
    revision: Revision,
    source_len: TextSize,
    spans: Vec<DelimiterSpan>,
    issues: Vec<DelimiterIssue>,
}

impl DelimiterSnapshot {
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn source_len(&self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub fn spans(&self) -> &[DelimiterSpan] {
        &self.spans
    }

    #[must_use]
    pub fn issues(&self) -> &[DelimiterIssue] {
        &self.issues
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DelimiterResolver;

impl DelimiterResolver {
    pub fn resolve(
        &self,
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
    ) -> Result<DelimiterSnapshot, DelimiterError> {
        if document.revision() != syntax.revision() {
            return Err(DelimiterError::RevisionMismatch {
                document: document.revision(),
                syntax: syntax.revision(),
            });
        }

        let document_len = document
            .len()
            .map_err(|_| DelimiterError::DocumentTooLarge)?;
        if document_len != syntax.source_len() {
            return Err(DelimiterError::SourceLengthMismatch {
                document: document_len,
                syntax: syntax.source_len(),
            });
        }

        let source = document.text();
        let mut output = ResolveOutput::default();
        resolve_node(&source, syntax.root(), &mut output)?;

        output.spans.sort_by_key(|span| {
            (
                span.range.start().get(),
                span.range.end().get(),
                delimiter_kind_code(span.kind),
            )
        });
        output
            .spans
            .dedup_by(|left, right| left.kind == right.kind && left.range == right.range);

        Ok(DelimiterSnapshot {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            spans: output.spans,
            issues: output.issues,
        })
    }
}

#[derive(Default)]
struct ResolveOutput {
    spans: Vec<DelimiterSpan>,
    issues: Vec<DelimiterIssue>,
}

fn resolve_node(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    validate_node_range(source, node.range())?;

    match node.kind() {
        SyntaxKind::Heading(_) => resolve_heading(source, node, output)?,
        SyntaxKind::Emphasis => resolve_wrapper(
            source,
            node,
            output,
            DelimiterKind::EmphasisOpen,
            DelimiterKind::EmphasisClose,
            WrapperKind::Emphasis,
        )?,
        SyntaxKind::Strong => resolve_wrapper(
            source,
            node,
            output,
            DelimiterKind::StrongOpen,
            DelimiterKind::StrongClose,
            WrapperKind::Strong,
        )?,
        SyntaxKind::Strikethrough => resolve_wrapper(
            source,
            node,
            output,
            DelimiterKind::StrikethroughOpen,
            DelimiterKind::StrikethroughClose,
            WrapperKind::Strikethrough,
        )?,
        SyntaxKind::InlineCode => resolve_inline_code(source, node, output)?,
        SyntaxKind::BlockQuote => resolve_block_quote(source, node, output)?,
        SyntaxKind::ListItem => resolve_list_item(source, node, output)?,
        SyntaxKind::Link => resolve_link(source, node, output)?,
        _ => {}
    }

    for child in node.children() {
        resolve_node(source, child, output)?;
    }

    Ok(())
}

fn resolve_heading(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    let range = node.range().as_usize_range();
    let slice = &source[range.clone()];

    if let Some((start, end)) = atx_prefix(slice) {
        push_span(
            output,
            source,
            node.kind(),
            DelimiterKind::HeadingPrefix,
            range.start + start,
            range.start + end,
        )?;

        if let Some((start, end)) = atx_suffix(slice) {
            push_span(
                output,
                source,
                node.kind(),
                DelimiterKind::HeadingSuffix,
                range.start + start,
                range.start + end,
            )?;
        }
        return Ok(());
    }

    if let Some((start, end)) = setext_underline(slice) {
        push_span(
            output,
            source,
            node.kind(),
            DelimiterKind::SetextUnderline,
            range.start + start,
            range.start + end,
        )?;
        return Ok(());
    }

    issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
    Ok(())
}

fn atx_prefix(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() && cursor < 3 && bytes[cursor] == b' ' {
        cursor += 1;
    }

    let marker_start = cursor;
    while cursor < bytes.len() && cursor - marker_start < 6 && bytes[cursor] == b'#' {
        cursor += 1;
    }

    let marker_len = cursor - marker_start;
    if marker_len == 0 {
        return None;
    }
    if cursor < bytes.len() && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
        return None;
    }

    Some((marker_start, cursor))
}

fn atx_suffix(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let mut end = trim_line_ending_end(bytes);
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }

    let marker_end = end;
    while end > 0 && bytes[end - 1] == b'#' {
        end -= 1;
    }
    if end == marker_end {
        return None;
    }

    if end == 0 || !matches!(bytes[end - 1], b' ' | b'\t') {
        return None;
    }

    Some((end, marker_end))
}

fn setext_underline(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let logical_end = trim_line_ending_end(bytes);
    let line_start = slice[..logical_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line = &slice[line_start..logical_end];
    let line_bytes = line.as_bytes();

    let mut cursor = 0;
    while cursor < line_bytes.len() && cursor < 3 && line_bytes[cursor] == b' ' {
        cursor += 1;
    }
    let marker_start = cursor;
    let marker = *line_bytes.get(cursor)?;
    if !matches!(marker, b'=' | b'-') {
        return None;
    }
    while cursor < line_bytes.len() && line_bytes[cursor] == marker {
        cursor += 1;
    }
    let marker_end = cursor;
    while cursor < line_bytes.len() && matches!(line_bytes[cursor], b' ' | b'\t') {
        cursor += 1;
    }
    if cursor != line_bytes.len() {
        return None;
    }

    Some((line_start + marker_start, line_start + marker_end))
}

fn trim_line_ending_end(bytes: &[u8]) -> usize {
    let mut end = bytes.len();
    if end > 0 && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    end
}

#[derive(Clone, Copy)]
enum WrapperKind {
    Emphasis,
    Strong,
    Strikethrough,
}

fn resolve_wrapper(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
    open_kind: DelimiterKind,
    close_kind: DelimiterKind,
    wrapper: WrapperKind,
) -> Result<(), DelimiterError> {
    let Some(first) = node.children().first() else {
        issue(output, node, DelimiterIssueReason::MissingContentBoundary);
        return Ok(());
    };
    let Some(last) = node.children().last() else {
        issue(output, node, DelimiterIssueReason::MissingContentBoundary);
        return Ok(());
    };

    let open = absolute_range(node.range().start(), first.range().start())?;
    let close = absolute_range(last.range().end(), node.range().end())?;

    if !valid_wrapper(source, open, wrapper) || !valid_wrapper(source, close, wrapper) {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_existing_span(output, node.kind(), open_kind, open);
    push_existing_span(output, node.kind(), close_kind, close);
    Ok(())
}

fn valid_wrapper(source: &str, range: TextRange, wrapper: WrapperKind) -> bool {
    let slice = &source[range.as_usize_range()];
    match wrapper {
        WrapperKind::Emphasis => {
            slice.len() == 1 && matches!(slice.as_bytes(), [b'*'] | [b'_'])
        }
        WrapperKind::Strong => {
            slice.len() == 2 && (slice.as_bytes() == b"**" || slice.as_bytes() == b"__")
        }
        WrapperKind::Strikethrough => slice.as_bytes() == b"~~",
    }
}

fn resolve_inline_code(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    let range = node.range().as_usize_range();
    let slice = &source[range.clone()];
    let open_len = slice.bytes().take_while(|byte| *byte == b'\x60').count();
    let close_len = slice
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\x60')
        .count();

    if open_len == 0 || open_len != close_len || open_len * 2 > slice.len() {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::InlineCodeOpen,
        range.start,
        range.start + open_len,
    )?;
    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::InlineCodeClose,
        range.end - close_len,
        range.end,
    )?;
    Ok(())
}

fn resolve_block_quote(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    let range = node.range().as_usize_range();
    let slice = &source[range.clone()];
    let mut relative_line_start = 0;

    while relative_line_start < slice.len() {
        let line_end = slice[relative_line_start..]
            .find('\n')
            .map_or(slice.len(), |offset| relative_line_start + offset);
        let line = &slice[relative_line_start..line_end];
        let mut cursor = 0;

        while cursor < line.len() && cursor < 3 && line.as_bytes()[cursor] == b' ' {
            cursor += 1;
        }

        let mut found = false;
        loop {
            if line.as_bytes().get(cursor) != Some(&b'>') {
                break;
            }
            found = true;
            let marker = range.start + relative_line_start + cursor;
            push_span(
                output,
                source,
                node.kind(),
                DelimiterKind::BlockQuoteMarker,
                marker,
                marker + 1,
            )?;
            cursor += 1;
            if matches!(line.as_bytes().get(cursor), Some(b' ' | b'\t')) {
                cursor += 1;
            }
        }

        if !found && relative_line_start == 0 {
            issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        }

        if line_end == slice.len() {
            break;
        }
        relative_line_start = line_end + 1;
    }

    Ok(())
}

fn resolve_list_item(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    let range = node.range().as_usize_range();
    let slice = &source[range.clone()];
    let first_line_end = slice.find('\n').unwrap_or(slice.len());
    let line = &slice[..first_line_end];
    let bytes = line.as_bytes();

    let mut cursor = 0;
    while cursor < bytes.len() && cursor < 3 && bytes[cursor] == b' ' {
        cursor += 1;
    }
    let marker_start = cursor;

    if matches!(bytes.get(cursor), Some(b'-' | b'+' | b'*')) {
        cursor += 1;
    } else {
        let digit_start = cursor;
        while cursor < bytes.len() && cursor - digit_start < 9 && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == digit_start || !matches!(bytes.get(cursor), Some(b'.' | b')')) {
            issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
            return Ok(());
        }
        cursor += 1;
    }

    if cursor >= bytes.len() || !matches!(bytes[cursor], b' ' | b'\t') {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::ListMarker,
        range.start + marker_start,
        range.start + cursor,
    )?;
    Ok(())
}

fn resolve_link(
    source: &str,
    node: &SyntaxNode,
    output: &mut ResolveOutput,
) -> Result<(), DelimiterError> {
    let Some(first) = node.children().first() else {
        issue(output, node, DelimiterIssueReason::MissingContentBoundary);
        return Ok(());
    };
    let Some(last) = node.children().last() else {
        issue(output, node, DelimiterIssueReason::MissingContentBoundary);
        return Ok(());
    };

    let range = node.range().as_usize_range();
    let content_start = first.range().start().to_usize();
    let content_end = last.range().end().to_usize();

    if content_start <= range.start || content_end > range.end {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    if source.as_bytes().get(range.start) == Some(&b'<')
        && source.as_bytes().get(range.end.saturating_sub(1)) == Some(&b'>')
    {
        push_span(
            output,
            source,
            node.kind(),
            DelimiterKind::AutolinkOpen,
            range.start,
            range.start + 1,
        )?;
        push_span(
            output,
            source,
            node.kind(),
            DelimiterKind::AutolinkClose,
            range.end - 1,
            range.end,
        )?;
        return Ok(());
    }

    if source.as_bytes().get(range.start) != Some(&b'[')
        || source.as_bytes().get(content_start.saturating_sub(1)) != Some(&b'[')
    {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::LinkLabelOpen,
        range.start,
        range.start + 1,
    )?;

    let suffix = &source[content_end..range.end];
    if !suffix.starts_with(']') {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::LinkLabelClose,
        content_end,
        content_end + 1,
    )?;

    let destination_start = content_end + 1;
    if destination_start == range.end {
        return Ok(());
    }

    let destination = &source[destination_start..range.end];
    let valid_destination = (destination.starts_with('(') && destination.ends_with(')'))
        || (destination.starts_with('[') && destination.ends_with(']'));
    if !valid_destination {
        issue(output, node, DelimiterIssueReason::UnexpectedSourceForm);
        return Ok(());
    }

    push_span(
        output,
        source,
        node.kind(),
        DelimiterKind::LinkDestination,
        destination_start,
        range.end,
    )?;
    Ok(())
}

fn push_span(
    output: &mut ResolveOutput,
    source: &str,
    owner: SyntaxKind,
    kind: DelimiterKind,
    start: usize,
    end: usize,
) -> Result<(), DelimiterError> {
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(DelimiterError::InvalidDelimiterBounds { start, end });
    }

    let start = TextSize::try_from_usize(start).map_err(|_| DelimiterError::DocumentTooLarge)?;
    let end = TextSize::try_from_usize(end).map_err(|_| DelimiterError::DocumentTooLarge)?;
    let range =
        TextRange::new(start, end).map_err(|_| DelimiterError::InvalidDelimiterRange)?;
    push_existing_span(output, owner, kind, range);
    Ok(())
}

fn push_existing_span(
    output: &mut ResolveOutput,
    owner: SyntaxKind,
    kind: DelimiterKind,
    range: TextRange,
) {
    if !range.is_empty() {
        output.spans.push(DelimiterSpan { kind, range, owner });
    }
}

fn issue(output: &mut ResolveOutput, node: &SyntaxNode, reason: DelimiterIssueReason) {
    output.issues.push(DelimiterIssue {
        owner: node.kind(),
        range: node.range(),
        reason,
    });
}

fn validate_node_range(source: &str, range: TextRange) -> Result<(), DelimiterError> {
    let range_usize = range.as_usize_range();
    if range_usize.start > range_usize.end
        || range_usize.end > source.len()
        || !source.is_char_boundary(range_usize.start)
        || !source.is_char_boundary(range_usize.end)
    {
        return Err(DelimiterError::InvalidSyntaxRange(range));
    }
    Ok(())
}

fn absolute_range(start: TextSize, end: TextSize) -> Result<TextRange, DelimiterError> {
    TextRange::new(start, end).map_err(|_| DelimiterError::InvalidSyntaxRange(TextRange::empty(start)))
}

const fn delimiter_kind_code(kind: DelimiterKind) -> u8 {
    match kind {
        DelimiterKind::HeadingPrefix => 0,
        DelimiterKind::HeadingSuffix => 1,
        DelimiterKind::SetextUnderline => 2,
        DelimiterKind::EmphasisOpen => 3,
        DelimiterKind::EmphasisClose => 4,
        DelimiterKind::StrongOpen => 5,
        DelimiterKind::StrongClose => 6,
        DelimiterKind::StrikethroughOpen => 7,
        DelimiterKind::StrikethroughClose => 8,
        DelimiterKind::InlineCodeOpen => 9,
        DelimiterKind::InlineCodeClose => 10,
        DelimiterKind::BlockQuoteMarker => 11,
        DelimiterKind::ListMarker => 12,
        DelimiterKind::LinkLabelOpen => 13,
        DelimiterKind::LinkLabelClose => 14,
        DelimiterKind::LinkDestination => 15,
        DelimiterKind::AutolinkOpen => 16,
        DelimiterKind::AutolinkClose => 17,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelimiterError {
    RevisionMismatch {
        document: Revision,
        syntax: Revision,
    },
    SourceLengthMismatch {
        document: TextSize,
        syntax: TextSize,
    },
    InvalidSyntaxRange(TextRange),
    InvalidDelimiterBounds {
        start: usize,
        end: usize,
    },
    InvalidDelimiterRange,
    DocumentTooLarge,
}

impl fmt::Display for DelimiterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RevisionMismatch { document, syntax } => write!(
                formatter,
                "document revision {} does not match syntax revision {}",
                document.get(),
                syntax.get()
            ),
            Self::SourceLengthMismatch { document, syntax } => write!(
                formatter,
                "document length {} does not match syntax length {}",
                document.get(),
                syntax.get()
            ),
            Self::InvalidSyntaxRange(range) => {
                write!(formatter, "syntax range {range:?} is invalid for delimiter resolution")
            }
            Self::InvalidDelimiterBounds { start, end } => {
                write!(formatter, "delimiter bounds {start}..{end} are invalid")
            }
            Self::InvalidDelimiterRange => write!(formatter, "delimiter range is invalid"),
            Self::DocumentTooLarge => write!(formatter, "document exceeds the supported source size"),
        }
    }
}

impl Error for DelimiterError {}

#[cfg(test)]
mod tests {
    use super::*;
    use mdedit_core::{Change, ChangeSet, Document, Transaction, TransactionKind};

    use crate::{MarkdownDialect, MarkdownParser, PulldownCmarkParser};

    fn resolve(source: &str, dialect: MarkdownDialect) -> DelimiterSnapshot {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &dialect);
        DelimiterResolver.resolve(&snapshot, &syntax).unwrap()
    }

    fn slices<'a>(
        source: &'a str,
        snapshot: &'a DelimiterSnapshot,
        kind: DelimiterKind,
    ) -> Vec<&'a str> {
        snapshot
            .spans()
            .iter()
            .filter(|span| span.kind() == kind)
            .map(|span| &source[span.range().as_usize_range()])
            .collect()
    }

    #[test]
    fn resolves_atx_and_setext_heading_markers() {
        let atx = "# Heading ###\n";
        let atx_snapshot = resolve(atx, MarkdownDialect::commonmark());
        assert_eq!(
            slices(atx, &atx_snapshot, DelimiterKind::HeadingPrefix),
            vec!["#"]
        );
        assert_eq!(
            slices(atx, &atx_snapshot, DelimiterKind::HeadingSuffix),
            vec!["###"]
        );

        let setext = "Heading\n===\n";
        let setext_snapshot = resolve(setext, MarkdownDialect::commonmark());
        assert_eq!(
            slices(setext, &setext_snapshot, DelimiterKind::SetextUnderline),
            vec!["==="]
        );
    }

    #[test]
    fn resolves_nested_emphasis_strong_and_strikethrough_markers() {
        let source = "***x*** and ~~gone~~";
        let snapshot = resolve(source, MarkdownDialect::gfm());

        assert_eq!(
            slices(source, &snapshot, DelimiterKind::EmphasisOpen),
            vec!["*"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::EmphasisClose),
            vec!["*"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::StrongOpen),
            vec!["**"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::StrongClose),
            vec!["**"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::StrikethroughOpen),
            vec!["~~"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::StrikethroughClose),
            vec!["~~"]
        );
    }

    #[test]
    fn resolves_variable_length_inline_code_fences() {
        let source = "\x60\x60code \x60 span\x60\x60";
        let snapshot = resolve(source, MarkdownDialect::commonmark());

        assert_eq!(
            slices(source, &snapshot, DelimiterKind::InlineCodeOpen),
            vec!["\x60\x60"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::InlineCodeClose),
            vec!["\x60\x60"]
        );
    }

    #[test]
    fn resolves_block_quote_and_list_markers() {
        let quote = "> one\n> two\n";
        let quote_snapshot = resolve(quote, MarkdownDialect::commonmark());
        assert_eq!(
            slices(quote, &quote_snapshot, DelimiterKind::BlockQuoteMarker),
            vec![">", ">"]
        );

        let list = "- one\n- two\n";
        let list_snapshot = resolve(list, MarkdownDialect::commonmark());
        assert_eq!(
            slices(list, &list_snapshot, DelimiterKind::ListMarker),
            vec!["-", "-"]
        );

        let ordered = "10. ten\n11) eleven\n";
        let ordered_snapshot = resolve(ordered, MarkdownDialect::commonmark());
        assert_eq!(
            slices(ordered, &ordered_snapshot, DelimiterKind::ListMarker),
            vec!["10.", "11)"]
        );
    }

    #[test]
    fn resolves_inline_link_label_and_destination_syntax() {
        let source = "[site](https://example.com)";
        let snapshot = resolve(source, MarkdownDialect::commonmark());

        assert_eq!(
            slices(source, &snapshot, DelimiterKind::LinkLabelOpen),
            vec!["["]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::LinkLabelClose),
            vec!["]"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::LinkDestination),
            vec!["(https://example.com)"]
        );
        assert!(snapshot.issues().is_empty());
    }

    #[test]
    fn resolves_autolink_markers_without_hiding_the_url_text() {
        let source = "<https://example.com>";
        let snapshot = resolve(source, MarkdownDialect::commonmark());

        assert_eq!(
            slices(source, &snapshot, DelimiterKind::AutolinkOpen),
            vec!["<"]
        );
        assert_eq!(
            slices(source, &snapshot, DelimiterKind::AutolinkClose),
            vec![">"]
        );
    }

    #[test]
    fn escaped_source_that_is_not_markup_produces_no_wrapper_delimiters() {
        let source = r"\*literal\*";
        let snapshot = resolve(source, MarkdownDialect::commonmark());

        assert!(slices(source, &snapshot, DelimiterKind::EmphasisOpen).is_empty());
        assert!(slices(source, &snapshot, DelimiterKind::EmphasisClose).is_empty());
    }

    #[test]
    fn rejects_revision_mismatch() {
        let mut document = Document::new("**a**").unwrap();
        let old_snapshot = document.snapshot();
        let syntax =
            PulldownCmarkParser.parse(&old_snapshot, &MarkdownDialect::commonmark());

        let transaction = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(
                TextRange::new(TextSize::ZERO, TextSize::new(1)).unwrap(),
                "_",
            )),
            TransactionKind::Programmatic,
        );
        document.apply(transaction).unwrap();

        let error = DelimiterResolver
            .resolve(&document.snapshot(), &syntax)
            .unwrap_err();
        assert!(matches!(error, DelimiterError::RevisionMismatch { .. }));
    }
}
