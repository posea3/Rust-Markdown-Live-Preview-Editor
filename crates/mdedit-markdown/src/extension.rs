use std::{error::Error, fmt};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};

use crate::{ParseStatus, SyntaxKind, SyntaxNode, SyntaxSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExtensionId(&'static str);

impl ExtensionId {
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExtensionKind {
    extension: ExtensionId,
    name: &'static str,
}

impl ExtensionKind {
    #[must_use]
    pub const fn extension(self) -> ExtensionId {
        self.extension
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionOverlapPolicy {
    Exclusive,
    AllowContained,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionScanMode {
    MarkdownText,
    AllSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionAttribute {
    key: &'static str,
    value: String,
}

impl ExtensionAttribute {
    #[must_use]
    pub const fn key(&self) -> &'static str {
        self.key
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionCandidate {
    name: &'static str,
    range: TextRange,
    priority: u16,
    overlap: ExtensionOverlapPolicy,
    attributes: Vec<ExtensionAttribute>,
}

impl ExtensionCandidate {
    #[must_use]
    pub const fn new(name: &'static str, range: TextRange) -> Self {
        Self {
            name,
            range,
            priority: 100,
            overlap: ExtensionOverlapPolicy::Exclusive,
            attributes: Vec::new(),
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub const fn with_overlap_policy(mut self, overlap: ExtensionOverlapPolicy) -> Self {
        self.overlap = overlap;
        self
    }

    #[must_use]
    pub fn with_attribute(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.attributes.push(ExtensionAttribute {
            key,
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionMatch {
    kind: ExtensionKind,
    range: TextRange,
    priority: u16,
    overlap: ExtensionOverlapPolicy,
    attributes: Vec<ExtensionAttribute>,
}

impl ExtensionMatch {
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub const fn priority(&self) -> u16 {
        self.priority
    }

    #[must_use]
    pub const fn overlap_policy(&self) -> ExtensionOverlapPolicy {
        self.overlap
    }

    #[must_use]
    pub fn attributes(&self) -> &[ExtensionAttribute] {
        &self.attributes
    }

    #[must_use]
    pub fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.key == key)
            .map(ExtensionAttribute::value)
    }
}

pub struct ExtensionScanContext<'a> {
    source: &'a str,
    syntax: &'a SyntaxSnapshot,
}

impl<'a> ExtensionScanContext<'a> {
    #[must_use]
    pub const fn source(&self) -> &'a str {
        self.source
    }

    #[must_use]
    pub const fn syntax(&self) -> &'a SyntaxSnapshot {
        self.syntax
    }

    #[must_use]
    pub fn source_range(&self, start: usize, end: usize) -> Option<TextRange> {
        if start > end
            || end > self.source.len()
            || !self.source.is_char_boundary(start)
            || !self.source.is_char_boundary(end)
        {
            return None;
        }

        let start = TextSize::try_from_usize(start).ok()?;
        let end = TextSize::try_from_usize(end).ok()?;
        TextRange::new(start, end).ok()
    }
}

pub trait SyntaxExtension: Send + Sync {
    fn id(&self) -> ExtensionId;

    fn scan_mode(&self) -> ExtensionScanMode {
        ExtensionScanMode::MarkdownText
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate>;
}

#[derive(Default)]
pub struct ExtensionSet {
    extensions: Vec<Box<dyn SyntaxExtension>>,
}

impl ExtensionSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register<E>(&mut self, extension: E)
    where
        E: SyntaxExtension + 'static,
    {
        self.extensions.push(Box::new(extension));
    }

    #[must_use]
    pub fn with_extension<E>(mut self, extension: E) -> Self
    where
        E: SyntaxExtension + 'static,
    {
        self.register(extension);
        self
    }

    pub fn scan(
        &self,
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
    ) -> Result<ExtensionSnapshot, ExtensionError> {
        if syntax.status() != ParseStatus::Parsed {
            return Err(ExtensionError::SyntaxUnavailable(syntax.status()));
        }
        if document.revision() != syntax.revision() {
            return Err(ExtensionError::RevisionMismatch {
                document: document.revision(),
                syntax: syntax.revision(),
            });
        }

        let source_len = document
            .len()
            .map_err(|_| ExtensionError::DocumentTooLarge)?;
        if source_len != syntax.source_len() {
            return Err(ExtensionError::SourceLengthMismatch {
                document: source_len,
                syntax: syntax.source_len(),
            });
        }

        let source = document.text();
        let protected = collect_protected_ranges(syntax.root());
        let context = ExtensionScanContext {
            source: &source,
            syntax,
        };

        let mut pending = Vec::new();
        let mut issues = Vec::new();

        for extension in &self.extensions {
            let id = extension.id();
            let mode = extension.scan_mode();

            for candidate in extension.scan(&context) {
                let range = candidate.range.as_usize_range();
                if candidate.range.is_empty()
                    || range.end > source.len()
                    || !source.is_char_boundary(range.start)
                    || !source.is_char_boundary(range.end)
                {
                    issues.push(ExtensionIssue {
                        kind: ExtensionKind {
                            extension: id,
                            name: candidate.name,
                        },
                        range: candidate.range,
                        reason: ExtensionIssueReason::InvalidRange,
                    });
                    continue;
                }

                if mode == ExtensionScanMode::MarkdownText
                    && let Some((_, core_kind)) = protected
                        .iter()
                        .find(|(protected_range, _)| ranges_overlap(*protected_range, candidate.range))
                {
                    issues.push(ExtensionIssue {
                        kind: ExtensionKind {
                            extension: id,
                            name: candidate.name,
                        },
                        range: candidate.range,
                        reason: ExtensionIssueReason::SuppressedByCoreSyntax(*core_kind),
                    });
                    continue;
                }

                pending.push(ExtensionMatch {
                    kind: ExtensionKind {
                        extension: id,
                        name: candidate.name,
                    },
                    range: candidate.range,
                    priority: candidate.priority,
                    overlap: candidate.overlap,
                    attributes: candidate.attributes,
                });
            }
        }

        pending.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.range.start().cmp(&right.range.start()))
                .then_with(|| right.range.end().cmp(&left.range.end()))
                .then_with(|| left.kind.cmp(&right.kind))
        });

        let mut accepted: Vec<ExtensionMatch> = Vec::new();
        for candidate in pending {
            let conflict = accepted.iter().find(|existing| {
                ranges_overlap(existing.range, candidate.range)
                    && !can_coexist(existing, &candidate)
            });

            if let Some(existing) = conflict {
                issues.push(ExtensionIssue {
                    kind: candidate.kind,
                    range: candidate.range,
                    reason: ExtensionIssueReason::Conflict {
                        kept: existing.kind,
                    },
                });
            } else {
                accepted.push(candidate);
            }
        }

        accepted.sort_by(|left, right| {
            left.range
                .start()
                .cmp(&right.range.start())
                .then_with(|| right.range.end().cmp(&left.range.end()))
                .then_with(|| left.kind.cmp(&right.kind))
        });

        Ok(ExtensionSnapshot {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            matches: accepted,
            issues,
        })
    }
}

fn can_coexist(left: &ExtensionMatch, right: &ExtensionMatch) -> bool {
    if left.range == right.range {
        return false;
    }

    let contained = range_contains(left.range, right.range)
        || range_contains(right.range, left.range);

    contained
        && left.overlap == ExtensionOverlapPolicy::AllowContained
        && right.overlap == ExtensionOverlapPolicy::AllowContained
}

fn ranges_overlap(left: TextRange, right: TextRange) -> bool {
    left.start() < right.end() && right.start() < left.end()
}

fn range_contains(outer: TextRange, inner: TextRange) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

fn collect_protected_ranges(node: &SyntaxNode) -> Vec<(TextRange, SyntaxKind)> {
    let mut ranges = Vec::new();
    collect_protected_ranges_into(node, &mut ranges);
    ranges
}

fn collect_protected_ranges_into(
    node: &SyntaxNode,
    ranges: &mut Vec<(TextRange, SyntaxKind)>,
) {
    if is_protected_kind(node.kind()) {
        ranges.push((node.range(), node.kind()));
        return;
    }

    for child in node.children() {
        collect_protected_ranges_into(child, ranges);
    }
}

const fn is_protected_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::RawSource
            | SyntaxKind::CodeBlock
            | SyntaxKind::InlineCode
            | SyntaxKind::HtmlBlock
            | SyntaxKind::Html
            | SyntaxKind::InlineHtml
            | SyntaxKind::MetadataBlock
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionSnapshot {
    revision: Revision,
    source_len: TextSize,
    matches: Vec<ExtensionMatch>,
    issues: Vec<ExtensionIssue>,
}

impl ExtensionSnapshot {
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn source_len(&self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub fn matches(&self) -> &[ExtensionMatch] {
        &self.matches
    }

    #[must_use]
    pub fn issues(&self) -> &[ExtensionIssue] {
        &self.issues
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionIssueReason {
    InvalidRange,
    SuppressedByCoreSyntax(SyntaxKind),
    Conflict { kept: ExtensionKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtensionIssue {
    kind: ExtensionKind,
    range: TextRange,
    reason: ExtensionIssueReason,
}

impl ExtensionIssue {
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub const fn reason(&self) -> ExtensionIssueReason {
        self.reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionError {
    SyntaxUnavailable(ParseStatus),
    RevisionMismatch {
        document: Revision,
        syntax: Revision,
    },
    SourceLengthMismatch {
        document: TextSize,
        syntax: TextSize,
    },
    DocumentTooLarge,
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyntaxUnavailable(status) => {
                write!(formatter, "extension scanning requires parsed syntax, got {status:?}")
            }
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
            Self::DocumentTooLarge => write!(formatter, "document exceeds the supported source size"),
        }
    }
}

impl Error for ExtensionError {}
