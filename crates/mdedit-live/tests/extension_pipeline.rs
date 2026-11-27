//! Public consumer composition: these tests add no automatic host runtime.
use std::{
    convert::Infallible,
    error::Error,
    fmt, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use mdedit_core::{
    Change, ChangeSet, Document, DocumentSnapshot, TextRange, TextSize, Transaction,
    TransactionError, TransactionKind,
};
use mdedit_live::{
    HostRenderError, HostRenderInput, HostRenderKey, HostRendererRegistry, Projection,
    ProjectionBuildError, RevealContext, RevealPolicy, WidgetAction, WidgetKind,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionError, ExtensionId,
    ExtensionIssueReason, ExtensionScanContext, ExtensionScanError, ExtensionSet,
    ExtensionSnapshot, FallibleSyntaxExtension, MarkdownDialect, MarkdownParser,
    ObsidianSyntaxExtension, PulldownCmarkParser, SyntaxExtension,
};

const REFERENCES: ExtensionId = ExtensionId::new("consumer.references");

struct References;
impl SyntaxExtension for References {
    fn id(&self) -> ExtensionId {
        REFERENCES
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        context
            .source()
            .match_indices("{{ref:")
            .filter_map(|(start, _)| {
                let value_start = start + 6;
                let value_end = value_start + context.source()[value_start..].find("}}")?;
                Some(
                    ExtensionCandidate::new(
                        "reference",
                        context.source_range(start, value_end + 2)?,
                    )
                    .with_attribute("target", &context.source()[value_start..value_end]),
                )
            })
            .collect()
    }
}

struct FallibleReferences(Arc<AtomicBool>);
impl FallibleSyntaxExtension for FallibleReferences {
    fn id(&self) -> ExtensionId {
        REFERENCES
    }

    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        if self.0.load(Ordering::Relaxed) {
            Err(ExtensionScanError::new("reference index unavailable"))
        } else {
            Ok(References.scan(context))
        }
    }
}

fn scan(
    document: &DocumentSnapshot,
    dialect: MarkdownDialect,
    extensions: &ExtensionSet,
) -> ExtensionSnapshot {
    let syntax = PulldownCmarkParser.parse(document, &dialect);
    extensions.scan(document, &syntax).unwrap()
}

fn project(
    document: &DocumentSnapshot,
    dialect: MarkdownDialect,
    extensions: &ExtensionSet,
    metadata: &ExtensionSnapshot,
) -> Result<Projection, ProjectionBuildError> {
    let syntax = PulldownCmarkParser.parse(document, &dialect);
    let blocks = BlockCache::new().reconcile(document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(document, &syntax).unwrap();
    Projection::build_with_current_extensions(
        document,
        &syntax,
        &blocks,
        &delimiters,
        metadata,
        extensions,
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    )
}

fn source_visible(document: &DocumentSnapshot, dialect: MarkdownDialect) -> Projection {
    let syntax = PulldownCmarkParser.parse(document, &dialect);
    let blocks = BlockCache::new().reconcile(document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(document, &syntax).unwrap();
    Projection::build(
        document,
        &syntax,
        &blocks,
        &delimiters,
        RevealPolicy::SourceVisible,
        &RevealContext::new(),
    )
    .unwrap()
}

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

#[test]
fn commonmark_with_no_extensions_needs_no_host_renderers() {
    let document = Document::new("**한글**\r\n").unwrap().snapshot();
    let extensions = ExtensionSet::new();
    let metadata = scan(&document, MarkdownDialect::commonmark(), &extensions);
    let projection = project(
        &document,
        MarkdownDialect::commonmark(),
        &extensions,
        &metadata,
    )
    .unwrap();
    let registry = HostRendererRegistry::<(), (), Infallible>::new();
    registry.check_widget_support(&projection).unwrap();
    assert!(metadata.matches().is_empty());
    assert!(projection.widgets().is_empty());
    assert_eq!(projection.text(), "한글\r\n");
    assert_eq!(document.text(), "**한글**\r\n");
}

#[test]
fn gfm_host_action_preserves_unicode_crlf_and_requires_fresh_metadata_after_edit() {
    let source = "- [ ] 작업\r\n\r\n{{ref:설계}}\r\n\r\n`{{ref:보호}}`\r\n";
    let mut document = Document::new(source).unwrap();
    let extensions = ExtensionSet::new().with_extension(References);
    let metadata = scan(&document.snapshot(), MarkdownDialect::gfm(), &extensions);
    assert_eq!(metadata.matches().len(), 1);
    assert!(matches!(
        metadata.issues()[0].reason(),
        ExtensionIssueReason::SuppressedByCoreSyntax(_)
    ));
    let projection = project(
        &document.snapshot(),
        MarkdownDialect::gfm(),
        &extensions,
        &metadata,
    )
    .unwrap();
    let mut registry = HostRendererRegistry::<Vec<String>, Option<WidgetAction>, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            |_, input| {
                let HostRenderInput::Widget(widget) = input else {
                    unreachable!()
                };
                Ok(widget.primary_action())
            },
        )
        .unwrap();
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: REFERENCES,
                name: "reference",
            },
            |view, input| {
                let HostRenderInput::Extension(item) = input else {
                    unreachable!()
                };
                view.push(item.attribute("target").unwrap().to_owned());
                Ok(None)
            },
        )
        .unwrap();
    registry.check_widget_support(&projection).unwrap();
    let mut view = Vec::new();
    let action = registry
        .render(&mut view, HostRenderInput::Widget(&projection.widgets()[0]))
        .unwrap()
        .unwrap();
    registry
        .render(
            &mut view,
            HostRenderInput::Extension(&metadata.matches()[0]),
        )
        .unwrap();
    assert_eq!(view, vec!["설계"]);
    assert_eq!(
        action,
        WidgetAction::SetTaskChecked {
            marker_range: range(2, 5),
            checked: true
        }
    );
    assert_eq!(document.text(), source);

    let WidgetAction::SetTaskChecked { marker_range, .. } = action;
    document
        .apply(Transaction::new(
            projection.revision(),
            ChangeSet::single(Change::new(marker_range, action.replacement_text())),
            TransactionKind::Widget,
        ))
        .unwrap();
    assert_eq!(
        document.text(),
        "- [x] 작업\r\n\r\n{{ref:설계}}\r\n\r\n`{{ref:보호}}`\r\n"
    );
    assert!(matches!(
        project(
            &document.snapshot(),
            MarkdownDialect::gfm(),
            &extensions,
            &metadata
        ),
        Err(ProjectionBuildError::RevisionMismatch {
            input: "extensions",
            ..
        })
    ));
    let refreshed = scan(&document.snapshot(), MarkdownDialect::gfm(), &extensions);
    let projection = project(
        &document.snapshot(),
        MarkdownDialect::gfm(),
        &extensions,
        &refreshed,
    )
    .unwrap();
    registry.check_widget_support(&projection).unwrap();
    assert_eq!(
        registry
            .render(&mut view, HostRenderInput::Widget(&projection.widgets()[0]))
            .unwrap(),
        Some(WidgetAction::SetTaskChecked {
            marker_range: range(2, 5),
            checked: false
        })
    );
}

#[test]
fn delayed_checkbox_action_rejects_an_intervening_edit_on_its_originating_document() {
    let mut document = Document::new("- [ ] task\n").unwrap();
    let extensions = ExtensionSet::new();
    let metadata = scan(&document.snapshot(), MarkdownDialect::gfm(), &extensions);
    let projection = project(
        &document.snapshot(),
        MarkdownDialect::gfm(),
        &extensions,
        &metadata,
    )
    .unwrap();
    let mut registry = HostRendererRegistry::<(), WidgetAction, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            |_, input| {
                let HostRenderInput::Widget(widget) = input else {
                    unreachable!()
                };
                Ok(widget.primary_action().unwrap())
            },
        )
        .unwrap();
    registry.check_widget_support(&projection).unwrap();
    let rendered_revision = projection.revision();
    let action = registry
        .render(&mut (), HostRenderInput::Widget(&projection.widgets()[0]))
        .unwrap();

    document
        .apply(Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range(0, 0), "abc")),
            TransactionKind::Typing,
        ))
        .unwrap();
    let WidgetAction::SetTaskChecked { marker_range, .. } = action;
    let application = document.apply(Transaction::new(
        rendered_revision,
        ChangeSet::single(Change::new(marker_range, action.replacement_text())),
        TransactionKind::Widget,
    ));
    assert_eq!(
        document.text(),
        "abc- [ ] task\n",
        "a stale rendered action must preserve the intervening edit"
    );
    assert_eq!(
        application.unwrap_err(),
        TransactionError::StaleRevision {
            expected: rendered_revision,
            actual: document.revision()
        }
    );
}

#[test]
fn obsidian_scan_failure_uses_visible_source_then_invalidates_cached_metadata_on_recovery() {
    let source = "> [!note] 안내\r\n> 본문\r\n\r\n{{ref:설계}}\r\n";
    let document = Document::new(source).unwrap().snapshot();
    let fail = Arc::new(AtomicBool::new(false));
    let mut extensions = ExtensionSet::new()
        .with_extension(ObsidianSyntaxExtension)
        .with_fallible_extension(FallibleReferences(Arc::clone(&fail)));
    let cached = scan(&document, MarkdownDialect::obsidian(), &extensions);
    let projection = project(&document, MarkdownDialect::obsidian(), &extensions, &cached).unwrap();
    assert_eq!(projection.widgets().len(), 1);
    assert_eq!(projection.widgets()[0].kind(), WidgetKind::Callout);
    let mut registry = HostRendererRegistry::<(), &'static str, Infallible>::new();
    registry
        .try_register(HostRenderKey::Widget(WidgetKind::Callout), |_, _| {
            Ok("callout")
        })
        .unwrap();
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: REFERENCES,
                name: "reference",
            },
            |_, _| Ok("reference"),
        )
        .unwrap();
    registry.check_widget_support(&projection).unwrap();

    fail.store(true, Ordering::Relaxed);
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
    let failure = extensions.scan(&document, &syntax).unwrap_err();
    assert!(
        matches!(&failure, ExtensionError::ScannerFailed { extension, error } if *extension == REFERENCES && error.message() == "reference index unavailable")
    );
    // A matching old ID cannot turn the failed scan attempt into a success.
    assert_eq!(cached.configuration_id(), extensions.configuration_id());
    let fallback = source_visible(&document, MarkdownDialect::obsidian());
    registry.check_widget_support(&fallback).unwrap();
    assert!(fallback.widgets().is_empty());
    assert_eq!(fallback.text(), source);

    fail.store(false, Ordering::Relaxed);
    extensions.invalidate_configuration();
    assert!(matches!(
        project(&document, MarkdownDialect::obsidian(), &extensions, &cached),
        Err(ProjectionBuildError::ExtensionConfigurationMismatch { .. })
    ));
    let refreshed = scan(&document, MarkdownDialect::obsidian(), &extensions);
    let projection = project(
        &document,
        MarkdownDialect::obsidian(),
        &extensions,
        &refreshed,
    )
    .unwrap();
    registry.check_widget_support(&projection).unwrap();
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Widget(&projection.widgets()[0]))
            .unwrap(),
        "callout"
    );
    let reference = refreshed
        .matches()
        .iter()
        .find(|item| item.kind().extension() == REFERENCES)
        .unwrap();
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Extension(reference))
            .unwrap(),
        "reference"
    );
    assert_eq!(document.text(), source);
}

#[derive(Debug)]
struct ViewError(Box<dyn Error>);
impl fmt::Display for ViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0.as_ref(), formatter)
    }
}
impl Error for ViewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

fn render_for_host(
    registry: &mut HostRendererRegistry<(), (), ViewError>,
    input: HostRenderInput<'_>,
) -> Result<(), Box<dyn Error>> {
    registry.render(&mut (), input)?;
    Ok(())
}

#[test]
fn renderer_error_after_preflight_keeps_boxed_cause_and_can_rebuild_visible_source() {
    let source = "> [!note] n\n> b\n";
    let document = Document::new(source).unwrap().snapshot();
    let extensions = ExtensionSet::new().with_extension(ObsidianSyntaxExtension);
    let metadata = scan(&document, MarkdownDialect::obsidian(), &extensions);
    let projection = project(
        &document,
        MarkdownDialect::obsidian(),
        &extensions,
        &metadata,
    )
    .unwrap();
    let mut registry = HostRendererRegistry::<(), (), ViewError>::new();
    registry
        .try_register(HostRenderKey::Widget(WidgetKind::Callout), |_, _| {
            Err(ViewError(Box::new(io::Error::new(
                io::ErrorKind::NotFound,
                "callout resource missing",
            ))))
        })
        .unwrap();
    registry.check_widget_support(&projection).unwrap();
    let failure = render_for_host(
        &mut registry,
        HostRenderInput::Widget(&projection.widgets()[0]),
    )
    .unwrap_err();
    let dispatch_error = failure
        .downcast_ref::<HostRenderError<ViewError>>()
        .unwrap();
    assert!(
        matches!(dispatch_error, HostRenderError::RendererFailed { key: HostRenderKey::Widget(WidgetKind::Callout), source_range, .. } if *source_range == range(0, 16))
    );
    let view_error = failure
        .source()
        .unwrap()
        .downcast_ref::<ViewError>()
        .unwrap();
    assert_eq!(
        view_error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
    let fallback = source_visible(&document, MarkdownDialect::obsidian());
    registry.check_widget_support(&fallback).unwrap();
    assert_eq!(fallback.text(), source);
    assert_eq!(document.text(), source);
}
