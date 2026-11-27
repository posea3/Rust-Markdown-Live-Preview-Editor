//! Command-capture example, not a native graphics renderer. Run with:
//! cargo run -p mdedit-live --example host_renderers

use std::{convert::Infallible, error::Error};

use mdedit_core::{Document, DocumentSnapshot};
use mdedit_live::{
    HostRenderInput, HostRenderKey, HostRendererRegistry, Projection, RevealContext, RevealPolicy,
    WidgetAction, WidgetKind,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionId, ExtensionScanContext,
    ExtensionSet, ExtensionSnapshot, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    SyntaxExtension,
};

const NOTE_ID: ExtensionId = ExtensionId::new("example.notes");

struct NoteScanner;
impl SyntaxExtension for NoteScanner {
    fn id(&self) -> ExtensionId {
        NOTE_ID
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        // This sample grammar recognizes one literal token, not nested syntax.
        context
            .source()
            .find("{{note}}")
            .map(|start| {
                ExtensionCandidate::new("note", context.source_range(start, start + 8).unwrap())
                    .with_attribute("target", "설계 노트")
            })
            .into_iter()
            .collect()
    }
}

#[derive(Default)]
struct HostView {
    paint_commands: Vec<String>,
}

fn project(
    document: &DocumentSnapshot,
    extensions: &ExtensionSet,
    policy: RevealPolicy,
) -> Result<(Projection, ExtensionSnapshot), Box<dyn Error>> {
    let syntax = PulldownCmarkParser.parse(document, &MarkdownDialect::gfm());
    let metadata = extensions.scan(document, &syntax)?;
    let blocks = BlockCache::new().reconcile(document, &syntax)?;
    let delimiters = DelimiterResolver.resolve(document, &syntax)?;
    let projection = Projection::build_with_current_extensions(
        document,
        &syntax,
        &blocks,
        &delimiters,
        &metadata,
        extensions,
        policy,
        &RevealContext::new(),
    )?;
    Ok((projection, metadata))
}

fn main() -> Result<(), Box<dyn Error>> {
    let extensions = ExtensionSet::new().with_extension(NoteScanner);
    let source = "- [ ] task\n\n{{note}}\n";
    let document = Document::new(source)?.snapshot();
    let (projection, metadata) = project(&document, &extensions, RevealPolicy::ConcealInactive)?;
    let mut registry = HostRendererRegistry::<HostView, Option<WidgetAction>, Infallible>::new();
    registry.try_register(
        HostRenderKey::Widget(WidgetKind::TaskCheckbox),
        |view, input| {
            let HostRenderInput::Widget(widget) = input else {
                unreachable!("widget key always receives a widget");
            };
            view.paint_commands.push(format!(
                "checkbox at {:?}, checked={:?}",
                input.source_range(),
                widget.task_checked()
            ));
            // Available click action, not an edit applied during rendering.
            Ok(widget.primary_action())
        },
    )?;
    registry.try_register(
        HostRenderKey::Extension {
            extension: NOTE_ID,
            name: "note",
        },
        |view, input| {
            let HostRenderInput::Extension(item) = input else {
                unreachable!("extension key always receives extension metadata");
            };
            view.paint_commands.push(format!(
                "note decoration at {:?}: {}",
                input.source_range(),
                item.attribute("target").unwrap_or_default()
            ));
            Ok(None)
        },
    )?;

    // Preflight before layout/painting: concealed widgets need host support.
    registry.check_widget_support(&projection)?;
    let mut view = HostView::default();
    let mut available_actions = Vec::new();
    for widget in projection.widgets() {
        if let Some(action) = registry.render(&mut view, HostRenderInput::Widget(widget))? {
            available_actions.push(action);
        }
    }
    // Only dispatch metadata this host owns. Do not paint built-in extension
    // matches a second time if a projected widget already represents them.
    for item in metadata
        .matches()
        .iter()
        .filter(|item| item.kind().extension() == NOTE_ID)
    {
        registry.render(&mut view, HostRenderInput::Extension(item))?;
    }
    for command in &view.paint_commands {
        println!("{command}");
    }
    println!("available editor actions: {available_actions:?}");
    assert_eq!(
        available_actions.len(),
        1,
        "the sample must expose a checkbox action"
    );
    println!(
        "projected text (note source stays visible): {:?}",
        projection.text()
    );
    assert_eq!(document.text(), source);

    // A different frame requires a rule renderer that this view lacks. Do not
    // merely skip it: rebuild source-visible projection and reflow that frame.
    let unsupported = Document::new("---\n")?.snapshot();
    let (concealed, _) = project(&unsupported, &extensions, RevealPolicy::ConcealInactive)?;
    if let Err(error) = registry.check_widget_support(&concealed) {
        println!("preflight: {error}");
        let (fallback, _) = project(&unsupported, &extensions, RevealPolicy::SourceVisible)?;
        registry.check_widget_support(&fallback)?;
        view.paint_commands.clear();
        // A real host reshapes/reflows this text and discards old paint output.
        println!("fallback source: {:?}", fallback.text());
        assert_eq!(fallback.text(), "---\n");
    }
    Ok(())
}
