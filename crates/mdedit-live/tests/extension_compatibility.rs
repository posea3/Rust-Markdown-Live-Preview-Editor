use mdedit_core::Document;
use mdedit_live::{Projection, RevealContext, RevealPolicy, WidgetKind};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionId, ExtensionScanContext,
    ExtensionSet, MarkdownDialect, MarkdownParser, ObsidianSyntaxExtension, PulldownCmarkParser,
    SyntaxExtension,
};

struct ApplicationScanner {
    kind: &'static str,
    marker: &'static str,
}

impl SyntaxExtension for ApplicationScanner {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.application")
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        context
            .source()
            .match_indices(self.marker)
            .filter_map(|(start, marker)| {
                context
                    .source_range(start, start + marker.len())
                    .map(|range| ExtensionCandidate::new(self.kind, range))
            })
            .collect()
    }
}

fn project(source: &str, extensions: &ExtensionSet) -> Projection {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
    let matches = extensions.scan(&document, &syntax).unwrap();
    let blocks = BlockCache::new().reconcile(&document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(&document, &syntax).unwrap();

    Projection::build_with_extensions(
        &document,
        &syntax,
        &blocks,
        &delimiters,
        &matches,
        RevealPolicy::ConcealInactive,
        &RevealContext::new(),
    )
    .unwrap()
}

#[test]
fn application_embed_kind_keeps_source_visible() {
    let extensions = ExtensionSet::new().with_extension(ApplicationScanner {
        kind: "embed",
        marker: "{{ref:한글}}",
    });
    let projection = project("{{ref:한글}}\n\n**ordinary**\n", &extensions);

    assert!(
        projection.widgets().is_empty(),
        "an application embed must not acquire the Obsidian native renderer"
    );
    assert_eq!(projection.text(), "{{ref:한글}}\n\nordinary\n");
}

#[test]
fn application_callout_kind_keeps_ordinary_blockquote_projection() {
    let extensions = ExtensionSet::new().with_extension(ApplicationScanner {
        kind: "callout",
        marker: "[!local]",
    });
    let projection = project(
        "> [!local] Team notice\n> **한글** body.\n\nafter\n",
        &extensions,
    );

    assert!(
        projection.widgets().is_empty(),
        "a foreign callout kind must not conceal the containing blockquote"
    );
    assert_eq!(
        projection.text(),
        "[!local] Team notice\n한글 body.\n\nafter\n"
    );
}

#[test]
fn application_and_obsidian_kinds_coexist_in_both_registration_orders() {
    let source = concat!(
        "{{ref:local}}\n\n",
        "> [!warning]- Native\n> **body**\n\n",
        "![[diagram.svg#Overview|Architecture]]\n",
    );

    for application_first in [true, false] {
        let application = ApplicationScanner {
            kind: "embed",
            marker: "{{ref:local}}",
        };
        let mut extensions = ExtensionSet::new();
        if application_first {
            extensions.register(application);
            extensions.register(ObsidianSyntaxExtension);
        } else {
            extensions.register(ObsidianSyntaxExtension);
            extensions.register(application);
        }

        let projection = project(source, &extensions);
        let kinds = projection
            .widgets()
            .iter()
            .map(|widget| widget.kind())
            .collect::<Vec<_>>();
        assert_eq!(kinds, [WidgetKind::Callout, WidgetKind::CustomEmbed]);
        assert!(projection.text().starts_with("{{ref:local}}\n\n"));
        assert_eq!(projection.widgets()[0].callout().unwrap().title(), "Native");
        assert_eq!(
            projection.widgets()[1].custom_embed().unwrap().display(),
            Some("Architecture")
        );
    }
}
