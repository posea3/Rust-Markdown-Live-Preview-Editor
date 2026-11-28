use std::{
    error::Error,
    fmt,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use mdedit_core::{DocumentSnapshot, Revision, TextRange, TextSize};

use crate::fallible::{ExtensionScanError, FallibleSyntaxExtension};
use crate::{ParseStatus, SyntaxKind, SyntaxNode, SyntaxSnapshot};

mod index;
mod profile;
use index::OverlapIndex;
#[cfg(test)]
mod indexed_tests;
#[cfg(test)]
mod scan_reference;

static CONFIGURATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Opaque process-local identity of one observed scanner configuration.
/// This is not a persistent ID, document identity, or configuration hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionConfigId(u64);

impl ExtensionConfigId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

fn allocate_configuration_id(sequence: &AtomicU64) -> ExtensionConfigId {
    let previous = sequence
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("extension configuration IDs exhausted");
    ExtensionConfigId(previous + 1)
}

/// Namespace owned by one registered scanner. Use a distinct ID per scanner.
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

/// A qualified syntax identity: the scanner namespace plus its local kind name.
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

/// Whether a candidate may coexist with a containing or contained candidate.
/// Identical ranges and crossing overlaps always conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionOverlapPolicy {
    Exclusive,
    AllowContained,
}

/// Candidate filtering policy. Scanners still receive the full canonical source.
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

    /// Constructs a UTF-8-aligned byte range within this source. Empty ranges
    /// are representable here but are rejected as extension candidates by `scan`.
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

/// Application-supplied synchronous scanner over canonical Markdown source.
///
/// Candidates are metadata claims, not source edits or native render requests.
/// The merge engine validates ranges and applies protection/overlap policies
/// after `scan` returns. Hosts own scanner registration and configuration epochs.
pub trait SyntaxExtension: Send + Sync {
    /// Namespace captured by `ExtensionSet` when this scanner is registered.
    /// Later changes to the returned ID do not rename that registration.
    fn id(&self) -> ExtensionId;

    fn scan_mode(&self) -> ExtensionScanMode {
        ExtensionScanMode::MarkdownText
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate>;
}

/// An invalid scanner registration. Scan-time failures use `ExtensionError`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionRegistrationError {
    DuplicateId(ExtensionId),
}

impl fmt::Display for ExtensionRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => {
                write!(
                    formatter,
                    "extension ID '{}' is already registered",
                    id.as_str()
                )
            }
        }
    }
}

impl Error for ExtensionRegistrationError {}

struct RegisteredExtension {
    id: ExtensionId,
    scanner: RegisteredScanner,
}

enum RegisteredScanner {
    Legacy(Box<dyn SyntaxExtension>),
    Fallible(Box<dyn FallibleSyntaxExtension>),
}

impl RegisteredScanner {
    fn scan_mode(&self) -> ExtensionScanMode {
        match self {
            Self::Legacy(scanner) => scanner.scan_mode(),
            Self::Fallible(scanner) => scanner.scan_mode(),
        }
    }

    fn scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        match self {
            Self::Legacy(scanner) => Ok(scanner.scan(context)),
            Self::Fallible(scanner) => scanner.try_scan(context),
        }
    }
}

#[derive(Default)]
pub struct ExtensionSet {
    extensions: Vec<RegisteredExtension>,
    configuration: OnceLock<ExtensionConfigId>,
}

impl ExtensionSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            extensions: Vec::new(),
            configuration: OnceLock::new(),
        }
    }

    /// Stable cache identity until successful registration or explicit
    /// invalidation. IDs are unique across sets within this process.
    ///
    /// # Panics
    ///
    /// Panics if the process has exhausted all `u64` configuration identities.
    #[must_use]
    pub fn configuration_id(&self) -> ExtensionConfigId {
        *self
            .configuration
            .get_or_init(|| allocate_configuration_id(&CONFIGURATION_SEQUENCE))
    }

    /// Invalidate cached snapshots after changing host-owned scanner settings.
    /// The next observation allocates a new identity. Scanner internals cannot
    /// be inspected automatically; hosts must call this method and rescan.
    pub fn invalidate_configuration(&mut self) {
        self.configuration.take();
    }

    /// Registers a scanner under its current ID. Recompute cached extension
    /// snapshots after registration or scanner configuration changes.
    ///
    /// # Panics
    ///
    /// Panics if the ID is already registered. Use `Self::try_register` when
    /// loading configuration that may contain duplicate scanner IDs.
    #[track_caller]
    pub fn register<E>(&mut self, extension: E)
    where
        E: SyntaxExtension + 'static,
    {
        if let Err(error) = self.try_register(extension) {
            panic!("{error}");
        }
    }

    /// Registers a scanner if its ID is unused, capturing the ID once.
    /// Rejection leaves existing registrations unchanged. Recompute cached
    /// extension snapshots after a successful registration.
    ///
    /// # Errors
    ///
    /// Returns `ExtensionRegistrationError::DuplicateId` if a scanner already
    /// owns this ID. The rejected scanner is not added to the set.
    pub fn try_register<E>(&mut self, extension: E) -> Result<(), ExtensionRegistrationError>
    where
        E: SyntaxExtension + 'static,
    {
        let id = extension.id();
        self.insert_registered(id, RegisteredScanner::Legacy(Box::new(extension)))
    }

    /// Register a fallible scanner under its current namespace. Successful
    /// registration invalidates configuration identity, as with legacy registration.
    ///
    /// # Errors
    ///
    /// Returns `ExtensionRegistrationError::DuplicateId` if either scanner
    /// interface already owns this namespace. The existing set is unchanged.
    pub fn try_register_fallible<E>(
        &mut self,
        extension: E,
    ) -> Result<(), ExtensionRegistrationError>
    where
        E: FallibleSyntaxExtension + 'static,
    {
        let id = extension.id();
        self.insert_registered(id, RegisteredScanner::Fallible(Box::new(extension)))
    }

    /// Register a fallible scanner for known-valid host configuration.
    ///
    /// # Panics
    ///
    /// Panics on duplicate IDs. Use `Self::try_register_fallible` to recover
    /// from configuration errors. Scan-time errors are returned by `Self::scan`.
    #[track_caller]
    pub fn register_fallible<E>(&mut self, extension: E)
    where
        E: FallibleSyntaxExtension + 'static,
    {
        if let Err(error) = self.try_register_fallible(extension) {
            panic!("{error}");
        }
    }

    /// Add a fallible scanner to this set.
    ///
    /// # Panics
    ///
    /// Panics on duplicate IDs, as with `Self::register_fallible`.
    #[must_use]
    #[track_caller]
    pub fn with_fallible_extension<E>(mut self, extension: E) -> Self
    where
        E: FallibleSyntaxExtension + 'static,
    {
        self.register_fallible(extension);
        self
    }

    fn insert_registered(
        &mut self,
        id: ExtensionId,
        scanner: RegisteredScanner,
    ) -> Result<(), ExtensionRegistrationError> {
        if self.extensions.iter().any(|registered| registered.id == id) {
            return Err(ExtensionRegistrationError::DuplicateId(id));
        }

        self.extensions.push(RegisteredExtension { id, scanner });
        self.invalidate_configuration();
        Ok(())
    }

    /// Adds a scanner to this set under its current ID.
    ///
    /// # Panics
    ///
    /// Panics if the ID is already registered, as with `Self::register`.
    #[must_use]
    #[track_caller]
    pub fn with_extension<E>(mut self, extension: E) -> Self
    where
        E: SyntaxExtension + 'static,
    {
        self.register(extension);
        self
    }

    /// Scans syntax from the same document snapshot and parser configuration.
    ///
    /// Revision and source-length checks reject stale inputs but do not prove
    /// document identity. Invalid individual candidates are reported through
    /// `ExtensionSnapshot::issues` without discarding unrelated valid matches.
    /// A fallible scanner's declared error returns `ExtensionError::ScannerFailed`
    /// immediately, without a partial snapshot or invoking later scanners.
    /// Panics propagate; earlier callback side effects are not rolled back.
    pub fn scan(
        &self,
        document: &DocumentSnapshot,
        syntax: &SyntaxSnapshot,
    ) -> Result<ExtensionSnapshot, ExtensionError> {
        let profile = profile::ScanProfile::new();
        profile.measure("validation", None, || {
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
            Ok(())
        })?;

        let configuration_id = self.configuration_id();
        let source = profile.measure("source_text", None, || document.text());
        let protected = profile.measure("protected_ranges", None, || {
            collect_protected_ranges(syntax.root())
        });
        let mut protected_index = None;
        let context = ExtensionScanContext {
            source: &source,
            syntax,
        };

        let mut pending = Vec::new();
        let mut issues = Vec::new();

        for (scanner_index, registered) in self.extensions.iter().enumerate() {
            let id = registered.id;
            let extension = &registered.scanner;
            let mode = extension.scan_mode();

            let candidates = profile
                .measure("scanner", Some(scanner_index), || extension.scan(&context))
                .map_err(|error| ExtensionError::ScannerFailed {
                    extension: id,
                    error,
                })?;
            profile.measure("candidate_filter", Some(scanner_index), || {
                for candidate in candidates {
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
                        && let Some(protected_slot) = protected_index
                            .get_or_insert_with(|| {
                                OverlapIndex::fully_active(
                                    protected.iter().map(|(range, _)| *range),
                                )
                            })
                            .first_matching(candidate.range, |_| true)
                    {
                        issues.push(ExtensionIssue {
                            kind: ExtensionKind {
                                extension: id,
                                name: candidate.name,
                            },
                            range: candidate.range,
                            reason: ExtensionIssueReason::SuppressedByCoreSyntax(
                                protected[protected_slot].1,
                            ),
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
            });
        }

        profile.measure("pending_sort", None, || {
            pending.sort_by(|left, right| {
                right
                    .priority
                    .cmp(&left.priority)
                    .then_with(|| left.range.start().cmp(&right.range.start()))
                    .then_with(|| right.range.end().cmp(&left.range.end()))
                    .then_with(|| left.kind.cmp(&right.kind))
            })
        });

        let mut accepted = profile.measure("conflict_merge", None, || {
            let mut accepted: Vec<ExtensionMatch> = Vec::new();
            let mut conflicts = OverlapIndex::new(pending.iter().map(|candidate| candidate.range));
            for (slot, candidate) in pending.into_iter().enumerate() {
                let conflict = conflicts.first_matching(candidate.range, |ordinal| {
                    !can_coexist(&accepted[ordinal], &candidate)
                });

                if let Some(ordinal) = conflict {
                    issues.push(ExtensionIssue {
                        kind: candidate.kind,
                        range: candidate.range,
                        reason: ExtensionIssueReason::Conflict {
                            kept: accepted[ordinal].kind,
                        },
                    });
                } else {
                    conflicts.activate(slot, accepted.len());
                    accepted.push(candidate);
                }
            }
            accepted
        });

        profile.measure("final_sort", None, || {
            accepted.sort_by(|left, right| {
                left.range
                    .start()
                    .cmp(&right.range.start())
                    .then_with(|| right.range.end().cmp(&left.range.end()))
                    .then_with(|| left.kind.cmp(&right.kind))
            })
        });

        Ok(ExtensionSnapshot {
            revision: syntax.revision(),
            source_len: syntax.source_len(),
            configuration_id,
            matches: accepted,
            issues,
        })
    }
}

fn can_coexist(left: &ExtensionMatch, right: &ExtensionMatch) -> bool {
    if left.range == right.range {
        return false;
    }

    let contained =
        range_contains(left.range, right.range) || range_contains(right.range, left.range);

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

fn collect_protected_ranges_into(node: &SyntaxNode, ranges: &mut Vec<(TextRange, SyntaxKind)>) {
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
    configuration_id: ExtensionConfigId,
    matches: Vec<ExtensionMatch>,
    issues: Vec<ExtensionIssue>,
}

impl ExtensionSnapshot {
    /// Configuration provenance captured when this snapshot was scanned.
    #[must_use]
    pub const fn configuration_id(&self) -> ExtensionConfigId {
        self.configuration_id
    }

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
    ScannerFailed {
        extension: ExtensionId,
        error: ExtensionScanError,
    },
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
            Self::ScannerFailed { extension, error } => write!(
                formatter,
                "extension '{}' failed: {error}",
                extension.as_str()
            ),
            Self::SyntaxUnavailable(status) => {
                write!(
                    formatter,
                    "extension scanning requires parsed syntax, got {status:?}"
                )
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
            Self::DocumentTooLarge => {
                write!(formatter, "document exceeds the supported source size")
            }
        }
    }
}

impl Error for ExtensionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ScannerFailed { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::{AtomicU64, Ordering, allocate_configuration_id};

    #[test]
    fn configuration_id_exhaustion_never_reuses_the_last_id() {
        let sequence = AtomicU64::new(u64::MAX - 1);
        assert_eq!(allocate_configuration_id(&sequence).get(), u64::MAX);
        assert!(std::panic::catch_unwind(|| allocate_configuration_id(&sequence)).is_err());
        assert_eq!(sequence.load(Ordering::Relaxed), u64::MAX);
    }
}
