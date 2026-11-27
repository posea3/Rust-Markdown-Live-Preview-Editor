use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use mdedit_core::{
    Change, ChangeSet, Document, DocumentSnapshot, TextRange, TextSize, Transaction,
    TransactionKind,
};
use mdedit_live::{Projection, ProjectionBuildError, RevealContext, RevealPolicy, WidgetKind};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionId, ExtensionScanContext,
    ExtensionSet, ExtensionSnapshot, MarkdownDialect, MarkdownParser, ObsidianSyntaxExtension,
    PulldownCmarkParser, SyntaxExtension,
};

struct OptionalObsidian(Arc<AtomicBool>);

impl SyntaxExtension for OptionalObsidian {
    fn id(&self) -> ExtensionId {
        ObsidianSyntaxExtension.id()
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        if self.0.load(Ordering::Relaxed) {
            ObsidianSyntaxExtension.scan(context)
        } else {
            Vec::new()
        }
    }
}

fn scan(document: &DocumentSnapshot, extensions: &ExtensionSet) -> ExtensionSnapshot {
    let syntax = PulldownCmarkParser.parse(document, &MarkdownDialect::obsidian());
    extensions.scan(document, &syntax).unwrap()
}

fn project(
    document: &DocumentSnapshot,
    extensions: &ExtensionSet,
    snapshot: &ExtensionSnapshot,
) -> Result<Projection, ProjectionBuildError> {
    let syntax = PulldownCmarkParser.parse(document, &MarkdownDialect::obsidian());
    let blocks = BlockCache::new().reconcile(document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(document, &syntax).unwrap();
    Projection::build_with_current_extensions(
        document,
        &syntax,
        &blocks,
        &delimiters,
        snapshot,
        extensions,
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    )
}

#[test]
fn current_snapshot_keeps_native_widget_projection_and_canonical_source() {
    let source = "![[note]]\n\n**text**\n";
    let document = Document::new(source).unwrap().snapshot();
    let extensions = ExtensionSet::new().with_extension(ObsidianSyntaxExtension);
    let snapshot = scan(&document, &extensions);
    let projection = project(&document, &extensions, &snapshot).unwrap();
    assert_eq!(projection.widgets().len(), 1);
    assert_eq!(projection.widgets()[0].kind(), WidgetKind::CustomEmbed);
    assert_eq!(projection.text(), "\n\ntext\n");
    assert_eq!(document.text(), source);
}

#[test]
fn registration_rejects_cached_data_even_without_a_source_edit() {
    let document = Document::new("![[note]]\n").unwrap().snapshot();
    let mut extensions = ExtensionSet::new();
    let cached = scan(&document, &extensions);
    extensions.register(ObsidianSyntaxExtension);
    assert_eq!(
        project(&document, &extensions, &cached).unwrap_err(),
        ProjectionBuildError::ExtensionConfigurationMismatch {
            expected: extensions.configuration_id(),
            snapshot: cached.configuration_id(),
        }
    );
    let refreshed = scan(&document, &extensions);
    assert_eq!(
        project(&document, &extensions, &refreshed)
            .unwrap()
            .widgets()
            .len(),
        1
    );
}

#[test]
fn invalidated_settings_cannot_restore_a_cached_concealed_widget() {
    let document = Document::new("> [!warning] Notice\n> **body**\n")
        .unwrap()
        .snapshot();
    let enabled = Arc::new(AtomicBool::new(true));
    let mut extensions = ExtensionSet::new().with_extension(OptionalObsidian(Arc::clone(&enabled)));
    let cached = scan(&document, &extensions);
    assert_eq!(
        project(&document, &extensions, &cached)
            .unwrap()
            .widgets()
            .len(),
        1
    );
    assert_eq!(
        project(&document, &extensions, &cached).unwrap().widgets()[0].kind(),
        WidgetKind::Callout
    );

    enabled.store(false, Ordering::Relaxed);
    extensions.invalidate_configuration();
    assert!(matches!(
        project(&document, &extensions, &cached),
        Err(ProjectionBuildError::ExtensionConfigurationMismatch { .. })
    ));
    let refreshed = scan(&document, &extensions);
    let projection = project(&document, &extensions, &refreshed).unwrap();
    assert!(projection.widgets().is_empty());
    assert_eq!(projection.text(), "[!warning] Notice\nbody\n");
}

#[test]
fn equal_document_metadata_does_not_make_unrelated_sets_interchangeable() {
    let document = Document::new("**text**").unwrap().snapshot();
    let original = ExtensionSet::new();
    let unrelated = ExtensionSet::new();
    let cached = scan(&document, &original);
    assert!(matches!(
        project(&document, &unrelated, &cached),
        Err(ProjectionBuildError::ExtensionConfigurationMismatch { .. })
    ));
}

#[test]
fn current_configuration_retains_revision_and_length_validation() {
    let mut document = Document::new("first").unwrap();
    let initial = document.snapshot();
    let extensions = ExtensionSet::new();
    let cached = scan(&initial, &extensions);
    let change = Change::new(
        TextRange::new(TextSize::ZERO, TextSize::new(5)).unwrap(),
        "other",
    );
    document
        .apply(Transaction::new(
            document.revision(),
            ChangeSet::new(vec![change]).unwrap(),
            TransactionKind::Programmatic,
        ))
        .unwrap();
    assert!(matches!(
        project(&document.snapshot(), &extensions, &cached),
        Err(ProjectionBuildError::RevisionMismatch {
            input: "extensions",
            ..
        })
    ));
    let different_length = Document::new("longer source").unwrap().snapshot();
    assert!(matches!(
        project(&different_length, &extensions, &cached),
        Err(ProjectionBuildError::SourceLengthMismatch {
            input: "extensions",
            ..
        })
    ));
}
