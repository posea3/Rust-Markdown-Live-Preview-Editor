use mdedit_core::Document;
use mdedit_live::{Projection, RevealContext, RevealPolicy, WidgetKind};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionId, ExtensionScanContext,
    ExtensionScanError, ExtensionSet, FallibleSyntaxExtension, MarkdownDialect, MarkdownParser,
    ObsidianSyntaxExtension, PulldownCmarkParser, SyntaxExtension,
};

struct FallibleObsidian;
impl FallibleSyntaxExtension for FallibleObsidian {
    fn id(&self) -> ExtensionId {
        ObsidianSyntaxExtension.id()
    }
    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        Ok(ObsidianSyntaxExtension.scan(context))
    }
}

#[test]
fn successful_fallible_metadata_keeps_widget_mapping_and_source() {
    let source = "![[note#Heading|한글]]\n\n**text**\n";
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
    let extensions = ExtensionSet::new().with_fallible_extension(FallibleObsidian);
    let snapshot = extensions.scan(&document, &syntax).unwrap();
    let blocks = BlockCache::new().reconcile(&document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(&document, &syntax).unwrap();
    let projection = Projection::build_with_current_extensions(
        &document,
        &syntax,
        &blocks,
        &delimiters,
        &snapshot,
        &extensions,
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    )
    .unwrap();
    assert_eq!(projection.widgets().len(), 1);
    assert_eq!(projection.widgets()[0].kind(), WidgetKind::CustomEmbed);
    assert_eq!(
        projection.widgets()[0].custom_embed().unwrap().display(),
        Some("한글")
    );
    assert_eq!(projection.text(), "\n\ntext\n");
    assert_eq!(document.text(), source);
}
