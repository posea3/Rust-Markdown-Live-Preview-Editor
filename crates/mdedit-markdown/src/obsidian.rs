use crate::{
    ExtensionCandidate, ExtensionId, ExtensionOverlapPolicy, ExtensionScanContext, ExtensionSet,
    SyntaxExtension,
};

pub const OBSIDIAN_EXTENSION_ID: ExtensionId = ExtensionId::new("obsidian");

pub const WIKILINK: &str = "wikilink";
pub const EMBED: &str = "embed";
pub const HIGHLIGHT: &str = "highlight";
pub const COMMENT: &str = "comment";
pub const CALLOUT: &str = "callout";
pub const BLOCK_ID: &str = "block-id";

#[derive(Clone, Copy, Debug, Default)]
pub struct ObsidianSyntaxExtension;

impl SyntaxExtension for ObsidianSyntaxExtension {
    fn id(&self) -> ExtensionId {
        OBSIDIAN_EXTENSION_ID
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        let source = context.source();
        let mut matches = Vec::new();

        scan_comments(context, source, &mut matches);
        scan_wikilinks(context, source, &mut matches);
        scan_highlights(context, source, &mut matches);
        scan_callouts(context, source, &mut matches);
        scan_block_ids(context, source, &mut matches);

        matches
    }
}

#[must_use]
pub fn obsidian_extension_set() -> ExtensionSet {
    ExtensionSet::new().with_extension(ObsidianSyntaxExtension)
}

fn scan_comments(
    context: &ExtensionScanContext<'_>,
    source: &str,
    output: &mut Vec<ExtensionCandidate>,
) {
    let mut cursor = 0;
    while let Some(open) = find_unescaped(source, "%%", cursor) {
        let Some(close) = find_unescaped(source, "%%", open + 2) else {
            break;
        };
        let end = close + 2;
        if let Some(range) = context.source_range(open, end) {
            output.push(
                ExtensionCandidate::new(COMMENT, range)
                    .with_priority(300)
                    .with_overlap_policy(ExtensionOverlapPolicy::Exclusive),
            );
        }
        cursor = end;
    }
}

fn scan_wikilinks(
    context: &ExtensionScanContext<'_>,
    source: &str,
    output: &mut Vec<ExtensionCandidate>,
) {
    let mut cursor = 0;
    while let Some(open) = find_unescaped(source, "[[", cursor) {
        let Some(close) = find_unescaped(source, "]]", open + 2) else {
            break;
        };

        let embed =
            open > 0 && source.as_bytes()[open - 1] == b'!' && !is_escaped(source, open - 1);
        let start = if embed { open - 1 } else { open };
        let end = close + 2;
        let body = &source[open + 2..close];

        if body.is_empty() {
            cursor = end;
            continue;
        }

        let Some(range) = context.source_range(start, end) else {
            cursor = end;
            continue;
        };

        let (target, display) = body
            .split_once('|')
            .map_or((body, None), |(left, right)| (left, Some(right)));
        let (path, fragment) = target
            .split_once('#')
            .map_or((target, None), |(left, right)| (left, Some(right)));

        let mut candidate = ExtensionCandidate::new(if embed { EMBED } else { WIKILINK }, range)
            .with_priority(220)
            .with_overlap_policy(if embed {
                ExtensionOverlapPolicy::Exclusive
            } else {
                ExtensionOverlapPolicy::AllowContained
            })
            .with_attribute("target", target)
            .with_attribute("path", path)
            .with_attribute("embed", if embed { "true" } else { "false" });

        if let Some(display) = display {
            candidate = candidate.with_attribute("display", display);
        }
        if let Some(fragment) = fragment {
            candidate = candidate.with_attribute("fragment", fragment);
            if let Some(block_id) = fragment.strip_prefix('^') {
                candidate = candidate.with_attribute("block-id", block_id);
            } else {
                candidate = candidate.with_attribute("heading", fragment);
            }
        }

        output.push(candidate);
        cursor = end;
    }
}

fn scan_highlights(
    context: &ExtensionScanContext<'_>,
    source: &str,
    output: &mut Vec<ExtensionCandidate>,
) {
    let mut cursor = 0;
    while let Some(open) = find_unescaped(source, "==", cursor) {
        let line_end = source[open + 2..]
            .find('\n')
            .map_or(source.len(), |offset| open + 2 + offset);
        let Some(close) = find_unescaped_bounded(source, "==", open + 2, line_end) else {
            cursor = open + 2;
            continue;
        };

        let end = close + 2;
        if close > open + 2
            && let Some(range) = context.source_range(open, end)
        {
            output.push(
                ExtensionCandidate::new(HIGHLIGHT, range)
                    .with_priority(120)
                    .with_overlap_policy(ExtensionOverlapPolicy::AllowContained),
            );
        }
        cursor = end;
    }
}

fn scan_callouts(
    context: &ExtensionScanContext<'_>,
    source: &str,
    output: &mut Vec<ExtensionCandidate>,
) {
    let mut line_start = 0;
    while line_start <= source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |offset| line_start + offset);
        let line = &source[line_start..line_end];

        if let Some((marker_start, marker_end, callout_type, fold, title)) =
            parse_callout_line(line)
            && let Some(range) =
                context.source_range(line_start + marker_start, line_start + marker_end)
        {
            let mut candidate = ExtensionCandidate::new(CALLOUT, range)
                .with_priority(180)
                .with_overlap_policy(ExtensionOverlapPolicy::Exclusive)
                .with_attribute("type", callout_type.to_ascii_lowercase());

            if let Some(fold) = fold {
                candidate = candidate.with_attribute("fold", fold.to_string());
            }
            if !title.is_empty() {
                candidate = candidate.with_attribute("title", title);
            }
            output.push(candidate);
        }

        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }
}

fn parse_callout_line(line: &str) -> Option<(usize, usize, &str, Option<char>, &str)> {
    let bytes = line.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() && cursor < 3 && bytes[cursor] == b' ' {
        cursor += 1;
    }

    let mut quote_count = 0;
    loop {
        if bytes.get(cursor) != Some(&b'>') {
            break;
        }
        quote_count += 1;
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
            cursor += 1;
        }
    }
    if quote_count == 0 || !line[cursor..].starts_with("[!") {
        return None;
    }

    let marker_start = cursor;
    let type_start = cursor + 2;
    let close = line[type_start..].find(']')? + type_start;
    let callout_type = &line[type_start..close];
    if callout_type.is_empty()
        || !callout_type
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return None;
    }

    cursor = close + 1;
    let fold = match bytes.get(cursor) {
        Some(b'+') => {
            cursor += 1;
            Some('+')
        }
        Some(b'-') => {
            cursor += 1;
            Some('-')
        }
        _ => None,
    };

    let title = line[cursor..].trim_start();
    Some((marker_start, cursor, callout_type, fold, title))
}

fn scan_block_ids(
    context: &ExtensionScanContext<'_>,
    source: &str,
    output: &mut Vec<ExtensionCandidate>,
) {
    let mut line_start = 0;
    while line_start <= source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |offset| line_start + offset);
        let line = source[line_start..line_end].trim_end_matches('\r');

        if let Some(token_start) = block_id_start(line) {
            let absolute_start = line_start + token_start;
            let absolute_end = line_start + line.len();
            if let Some(range) = context.source_range(absolute_start, absolute_end) {
                output.push(
                    ExtensionCandidate::new(BLOCK_ID, range)
                        .with_priority(140)
                        .with_overlap_policy(ExtensionOverlapPolicy::Exclusive)
                        .with_attribute("id", &line[token_start + 1..]),
                );
            }
        }

        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }
}

fn block_id_start(line: &str) -> Option<usize> {
    let caret = line.rfind('^')?;
    if caret == 0 || !line.as_bytes()[caret - 1].is_ascii_whitespace() {
        return None;
    }

    let id = &line[caret + 1..];
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }

    Some(caret)
}

fn find_unescaped(source: &str, needle: &str, from: usize) -> Option<usize> {
    find_unescaped_bounded(source, needle, from, source.len())
}

fn find_unescaped_bounded(source: &str, needle: &str, from: usize, end: usize) -> Option<usize> {
    let mut cursor = from;
    while cursor <= end {
        let relative = source[cursor..end].find(needle)?;
        let index = cursor + relative;
        if !is_escaped(source, index) {
            return Some(index);
        }
        cursor = index + needle.len();
    }
    None
}

fn is_escaped(source: &str, index: usize) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = index;
    let mut slashes = 0;

    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slashes += 1;
        cursor -= 1;
    }

    slashes % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdedit_core::Document;

    use crate::{ExtensionIssueReason, MarkdownDialect, MarkdownParser, PulldownCmarkParser};

    fn scan(source: &str) -> crate::ExtensionSnapshot {
        let document = Document::new(source).unwrap();
        let snapshot = document.snapshot();
        let syntax = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::obsidian());
        obsidian_extension_set().scan(&snapshot, &syntax).unwrap()
    }

    fn matches<'a>(
        snapshot: &'a crate::ExtensionSnapshot,
        name: &str,
    ) -> Vec<&'a crate::ExtensionMatch> {
        snapshot
            .matches()
            .iter()
            .filter(|item| item.kind().name() == name)
            .collect()
    }

    #[test]
    fn scans_obsidian_wikilinks_aliases_headings_blocks_and_embeds() {
        let source = concat!(
            "[[Note]] ",
            "[[Note#Heading|Alias]] ",
            "[[Note#^block-id]] ",
            "![[image.png|100]]"
        );
        let snapshot = scan(source);

        let links = matches(&snapshot, WIKILINK);
        assert_eq!(links.len(), 3);
        assert_eq!(links[1].attribute("target"), Some("Note#Heading"));
        assert_eq!(links[1].attribute("heading"), Some("Heading"));
        assert_eq!(links[1].attribute("display"), Some("Alias"));
        assert_eq!(links[2].attribute("block-id"), Some("block-id"));

        let embeds = matches(&snapshot, EMBED);
        assert_eq!(embeds.len(), 1);
        assert_eq!(embeds[0].attribute("path"), Some("image.png"));
        assert_eq!(embeds[0].attribute("display"), Some("100"));
    }

    #[test]
    fn scans_highlights_comments_callouts_and_block_ids() {
        let source = concat!(
            "==highlight==\n",
            "%% inline [[hidden]] %%\n",
            "> [!warning]- Custom title\n",
            "Paragraph text ^block-id\n"
        );
        let snapshot = scan(source);

        assert_eq!(matches(&snapshot, HIGHLIGHT).len(), 1);
        assert_eq!(matches(&snapshot, COMMENT).len(), 1);

        let callouts = matches(&snapshot, CALLOUT);
        assert_eq!(callouts.len(), 1);
        assert_eq!(callouts[0].attribute("type"), Some("warning"));
        assert_eq!(callouts[0].attribute("fold"), Some("-"));
        assert_eq!(callouts[0].attribute("title"), Some("Custom title"));

        let block_ids = matches(&snapshot, BLOCK_ID);
        assert_eq!(block_ids.len(), 1);
        assert_eq!(block_ids[0].attribute("id"), Some("block-id"));

        assert!(matches(&snapshot, WIKILINK).is_empty());
        assert!(snapshot.issues().iter().any(|issue| {
            issue.kind().name() == WIKILINK
                && matches!(issue.reason(), ExtensionIssueReason::Conflict { .. })
        }));
    }

    #[test]
    fn does_not_scan_obsidian_markup_inside_code_or_html() {
        let source = concat!(
            "\x60[[inline-code]]\x60\n\n",
            "\x60\x60\x60md\n==fenced==\n\x60\x60\x60\n\n",
            "<div>[[html-link]]</div>\n"
        );
        let snapshot = scan(source);

        assert!(snapshot.matches().is_empty());
        assert!(snapshot.issues().iter().all(|issue| {
            matches!(
                issue.reason(),
                ExtensionIssueReason::SuppressedByCoreSyntax(_)
            )
        }));
    }

    #[test]
    fn highlight_can_contain_a_wikilink_when_both_are_composable() {
        let source = "==See [[Note]] now==";
        let snapshot = scan(source);

        assert_eq!(matches(&snapshot, HIGHLIGHT).len(), 1);
        assert_eq!(matches(&snapshot, WIKILINK).len(), 1);
    }

    #[test]
    fn block_id_requires_whitespace_and_obsidian_id_characters() {
        let source = concat!(
            "valid ^quote-of-the-day\n",
            "invalid^missing-space\n",
            "invalid ^한글\n"
        );
        let snapshot = scan(source);

        let block_ids = matches(&snapshot, BLOCK_ID);
        assert_eq!(block_ids.len(), 1);
        assert_eq!(block_ids[0].attribute("id"), Some("quote-of-the-day"));
    }

    #[test]
    fn nested_callout_marker_is_recognized() {
        let snapshot = scan("> > [!todo]+ Nested\n");
        let callouts = matches(&snapshot, CALLOUT);

        assert_eq!(callouts.len(), 1);
        assert_eq!(callouts[0].attribute("type"), Some("todo"));
        assert_eq!(callouts[0].attribute("fold"), Some("+"));
    }
}
