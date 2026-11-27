use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use mdedit_core::{Document, TextRange, TextSize};
use mdedit_markdown::{
    ExtensionCandidate, ExtensionError, ExtensionId, ExtensionIssueReason,
    ExtensionRegistrationError, ExtensionScanContext, ExtensionScanError, ExtensionScanMode,
    ExtensionSet, FallibleSyntaxExtension, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    SyntaxExtension, SyntaxKind,
};

fn claims(
    context: &ExtensionScanContext<'_>,
    token: &str,
    kind: &'static str,
) -> Vec<ExtensionCandidate> {
    context
        .source()
        .match_indices(token)
        .map(|(start, token)| {
            ExtensionCandidate::new(
                kind,
                context.source_range(start, start + token.len()).unwrap(),
            )
        })
        .collect()
}

struct Legacy {
    id: &'static str,
    token: &'static str,
    calls: Arc<AtomicUsize>,
}

impl Legacy {
    fn new(id: &'static str, token: &'static str) -> Self {
        Self {
            id,
            token,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl SyntaxExtension for Legacy {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }
    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        claims(context, self.token, "legacy")
    }
}

struct Fallible {
    id: &'static str,
    fail: Arc<AtomicBool>,
    changed_id: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

impl Fallible {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            fail: Arc::new(AtomicBool::new(false)),
            changed_id: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl FallibleSyntaxExtension for Fallible {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(if self.changed_id.load(Ordering::Relaxed) {
            "example.changed"
        } else {
            self.id
        })
    }
    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail.load(Ordering::Relaxed) {
            Err(ExtensionScanError::new("인덱스 unavailable"))
        } else {
            Ok(claims(context, "fallible", "fallible"))
        }
    }
}

fn scan(
    source: &str,
    extensions: &ExtensionSet,
) -> Result<mdedit_markdown::ExtensionSnapshot, ExtensionError> {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    extensions.scan(&document, &syntax)
}

#[test]
fn fallible_and_legacy_scanners_share_successful_merge_and_provenance() {
    let extensions = ExtensionSet::new()
        .with_extension(Legacy::new("example.legacy", "legacy"))
        .with_fallible_extension(Fallible::new("example.fallible"));
    let result = scan("legacy fallible", &extensions).unwrap();
    assert!(result.issues().is_empty());
    assert_eq!(result.matches().len(), 2);
    assert_eq!(result.matches()[0].range().as_usize_range(), 0..6);
    assert_eq!(result.matches()[1].range().as_usize_range(), 7..15);
    assert_eq!(
        result.matches()[1].kind().extension().as_str(),
        "example.fallible"
    );
    assert_eq!(result.configuration_id(), extensions.configuration_id());
}

#[test]
fn failure_reports_captured_namespace_and_owned_source_cause() {
    let scanner = Fallible::new("example.original");
    let changed = Arc::clone(&scanner.changed_id);
    scanner.fail.store(true, Ordering::Relaxed);
    let mut extensions = ExtensionSet::new().with_fallible_extension(scanner);
    changed.store(true, Ordering::Relaxed);
    extensions.invalidate_configuration();
    let error = scan("fallible", &extensions).unwrap_err();
    assert!(
        matches!(&error, ExtensionError::ScannerFailed { extension, error }
        if extension.as_str() == "example.original" && error.message() == "인덱스 unavailable")
    );
    assert!(error.to_string().contains("example.original"));
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<ExtensionScanError>()
            .unwrap()
            .message(),
        "인덱스 unavailable"
    );
}

#[test]
fn first_failure_discards_prior_results_and_skips_later_scanners() {
    let first = Legacy::new("example.first", "legacy");
    let first_calls = Arc::clone(&first.calls);
    let failing = Fallible::new("example.failed");
    failing.fail.store(true, Ordering::Relaxed);
    let later = Legacy::new("example.later", "later");
    let later_calls = Arc::clone(&later.calls);
    let extensions = ExtensionSet::new()
        .with_extension(first)
        .with_fallible_extension(failing)
        .with_extension(later);
    let source = "legacy fallible later";
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    assert!(matches!(
        extensions.scan(&document, &syntax),
        Err(ExtensionError::ScannerFailed { .. })
    ));
    assert_eq!(first_calls.load(Ordering::Relaxed), 1);
    assert_eq!(later_calls.load(Ordering::Relaxed), 0);
    assert_eq!(document.text(), source);
}

#[test]
fn document_validation_precedes_fallible_callbacks() {
    let scanner = Fallible::new("example.failed");
    scanner.fail.store(true, Ordering::Relaxed);
    let calls = Arc::clone(&scanner.calls);
    let extensions = ExtensionSet::new().with_fallible_extension(scanner);
    let original = Document::new("fallible").unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&original, &MarkdownDialect::gfm());
    let wrong_length = Document::new("short").unwrap().snapshot();
    assert!(matches!(
        extensions.scan(&wrong_length, &syntax),
        Err(ExtensionError::SourceLengthMismatch { .. })
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn fallible_default_mode_respects_protected_code() {
    let extensions = ExtensionSet::new().with_fallible_extension(Fallible::new("example.filtered"));
    let result = scan("`fallible` fallible", &extensions).unwrap();
    assert_eq!(result.matches().len(), 1);
    assert_eq!(result.matches()[0].range().as_usize_range(), 11..19);
    assert_eq!(result.issues().len(), 1);
    assert_eq!(
        result.issues()[0].reason(),
        ExtensionIssueReason::SuppressedByCoreSyntax(SyntaxKind::InlineCode)
    );
}

struct AllSource(Fallible);
impl FallibleSyntaxExtension for AllSource {
    fn id(&self) -> ExtensionId {
        self.0.id()
    }
    fn scan_mode(&self) -> ExtensionScanMode {
        ExtensionScanMode::AllSource
    }
    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        self.0.try_scan(context)
    }
}

#[test]
fn fallible_all_source_mode_is_forwarded() {
    let extensions =
        ExtensionSet::new().with_fallible_extension(AllSource(Fallible::new("example.all")));
    let result = scan("`fallible` fallible", &extensions).unwrap();
    assert_eq!(result.matches().len(), 2);
    assert!(result.issues().is_empty());
}

struct InvalidCandidates;
impl FallibleSyntaxExtension for InvalidCandidates {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.invalid")
    }
    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        Ok(vec![
            ExtensionCandidate::new("valid", context.source_range(0, 3).unwrap()),
            ExtensionCandidate::new(
                "invalid",
                TextRange::new(TextSize::new(1), TextSize::new(2)).unwrap(),
            ),
        ])
    }
}

#[test]
fn invalid_candidates_remain_diagnostics_rather_than_declared_errors() {
    let extensions = ExtensionSet::new().with_fallible_extension(InvalidCandidates);
    let result = scan("한글", &extensions).unwrap();
    assert_eq!(result.matches().len(), 1);
    assert_eq!(result.issues().len(), 1);
    assert_eq!(
        result.issues()[0].reason(),
        ExtensionIssueReason::InvalidRange
    );
}

#[test]
fn both_interfaces_share_duplicate_rejection_without_changing_configuration() {
    for fallible_first in [false, true] {
        let mut extensions = ExtensionSet::new();
        if fallible_first {
            extensions.register_fallible(Fallible::new("example.shared"));
        } else {
            extensions.register(Legacy::new("example.shared", "legacy"));
        }
        let original = scan("legacy fallible", &extensions).unwrap();
        let result = if fallible_first {
            extensions.try_register(Legacy::new("example.shared", "legacy"))
        } else {
            extensions.try_register_fallible(Fallible::new("example.shared"))
        };
        assert_eq!(
            result,
            Err(ExtensionRegistrationError::DuplicateId(ExtensionId::new(
                "example.shared"
            )))
        );
        assert_eq!(scan("legacy fallible", &extensions).unwrap(), original);
    }
}

#[test]
#[should_panic(expected = "example.shared")]
fn fallible_registration_convenience_rejects_duplicates() {
    let mut extensions = ExtensionSet::new();
    extensions.register_fallible(Fallible::new("example.shared"));
    extensions.register_fallible(Fallible::new("example.shared"));
}

#[test]
#[should_panic(expected = "example.shared")]
fn fallible_builder_rejects_legacy_namespace_conflicts() {
    let _ = ExtensionSet::new()
        .with_extension(Legacy::new("example.shared", "legacy"))
        .with_fallible_extension(Fallible::new("example.shared"));
}

struct Panicking;
impl FallibleSyntaxExtension for Panicking {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.panic")
    }
    fn try_scan(
        &self,
        _: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        panic!("callback panic remains a panic");
    }
}

#[test]
#[should_panic(expected = "callback panic remains a panic")]
fn callback_panics_are_not_reclassified_as_reported_errors() {
    let extensions = ExtensionSet::new().with_fallible_extension(Panicking);
    let _ = scan("fallible", &extensions);
}

#[test]
fn host_can_invalidate_settings_and_retry_after_failure() {
    let scanner = Fallible::new("example.retry");
    let fail = Arc::clone(&scanner.fail);
    fail.store(true, Ordering::Relaxed);
    let mut extensions = ExtensionSet::new().with_fallible_extension(scanner);
    let original_id = extensions.configuration_id();
    assert!(matches!(
        scan("fallible", &extensions),
        Err(ExtensionError::ScannerFailed { .. })
    ));
    assert_eq!(extensions.configuration_id(), original_id);
    fail.store(false, Ordering::Relaxed);
    extensions.invalidate_configuration();
    let refreshed = scan("fallible", &extensions).unwrap();
    assert_eq!(refreshed.matches().len(), 1);
    assert_ne!(refreshed.configuration_id(), original_id);
}
