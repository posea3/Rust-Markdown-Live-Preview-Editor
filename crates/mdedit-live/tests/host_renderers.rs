use std::{cell::Cell, convert::Infallible, error::Error, fmt, rc::Rc};

use mdedit_core::{Document, DocumentSnapshot, TextRange, TextSize};
use mdedit_live::{
    HostRenderError, HostRenderInput, HostRenderKey, HostRendererRegistrationError,
    HostRendererRegistry, Projection, RevealContext, RevealPolicy, WidgetAction, WidgetKind,
};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionCandidate, ExtensionId, ExtensionScanContext,
    ExtensionSet, ExtensionSnapshot, MarkdownDialect, MarkdownParser, ObsidianSyntaxExtension,
    PulldownCmarkParser, SyntaxExtension,
};

const ALPHA: ExtensionId = ExtensionId::new("application.alpha");
const BETA: ExtensionId = ExtensionId::new("application.beta");

struct TokenScanner {
    id: ExtensionId,
    token: &'static str,
    kind: &'static str,
}

impl SyntaxExtension for TokenScanner {
    fn id(&self) -> ExtensionId {
        self.id
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        context
            .source()
            .find(self.token)
            .map(|start| {
                ExtensionCandidate::new(
                    self.kind,
                    context
                        .source_range(start, start + self.token.len())
                        .unwrap(),
                )
                .with_attribute("label", self.id.as_str())
            })
            .into_iter()
            .collect()
    }
}

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

fn project(
    document: &DocumentSnapshot,
    extensions: &ExtensionSet,
    policy: RevealPolicy,
) -> (Projection, ExtensionSnapshot) {
    let syntax = PulldownCmarkParser.parse(document, &MarkdownDialect::obsidian());
    let snapshot = extensions.scan(document, &syntax).unwrap();
    let blocks = BlockCache::new().reconcile(document, &syntax).unwrap();
    let delimiters = DelimiterResolver.resolve(document, &syntax).unwrap();
    let projection = Projection::build_with_current_extensions(
        document,
        &syntax,
        &blocks,
        &delimiters,
        &snapshot,
        extensions,
        policy,
        &RevealContext::new(),
    )
    .unwrap();
    (projection, snapshot)
}

#[test]
fn checkbox_handler_receives_payload_and_returns_an_action_without_editing_source() {
    let document = Document::new("- [ ] task\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let mut registry = HostRendererRegistry::<Vec<TextRange>, WidgetAction, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            |view, input| {
                view.push(input.source_range());
                let HostRenderInput::Widget(widget) = input else {
                    panic!("expected widget input");
                };
                assert_eq!(widget.task_checked(), Some(false));
                Ok(widget.primary_action().unwrap())
            },
        )
        .unwrap();
    let mut view = Vec::new();
    let action = registry
        .render(&mut view, HostRenderInput::Widget(&projection.widgets()[0]))
        .unwrap();
    assert_eq!(view, vec![range(2, 5)]);
    assert_eq!(
        action,
        WidgetAction::SetTaskChecked {
            marker_range: range(2, 5),
            checked: true
        }
    );
    assert_eq!(action.replacement_text(), "[x]");
    assert_eq!(document.text(), "- [ ] task\n");
}

#[test]
fn equal_local_names_dispatch_only_to_their_qualified_extension() {
    let document = Document::new("{{a}} {{b}}\n").unwrap().snapshot();
    let extensions = ExtensionSet::new()
        .with_extension(TokenScanner {
            id: ALPHA,
            token: "{{a}}",
            kind: "embed",
        })
        .with_extension(TokenScanner {
            id: BETA,
            token: "{{b}}",
            kind: "embed",
        });
    let (_, snapshot) = project(&document, &extensions, RevealPolicy::ConcealInactive);
    let mut registry = HostRendererRegistry::<(), &'static str, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: ALPHA,
                name: "embed",
            },
            |_, _| Ok("alpha"),
        )
        .unwrap();
    let alpha = &snapshot.matches()[0];
    let beta = &snapshot.matches()[1];
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Extension(alpha))
            .unwrap(),
        "alpha"
    );
    assert_eq!(
        registry.render(&mut (), HostRenderInput::Extension(beta)),
        Err(HostRenderError::Unregistered {
            key: HostRenderKey::Extension {
                extension: BETA,
                name: "embed"
            },
            source_range: range(6, 11),
        })
    );
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: BETA,
                name: "embed",
            },
            |_, _| Ok("beta"),
        )
        .unwrap();
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Extension(beta))
            .unwrap(),
        "beta"
    );
}

#[test]
fn extension_callout_handler_cannot_handle_a_builtin_callout() {
    let document = Document::new("{{a}}\n\n> [!warning] Notice\n> body\n")
        .unwrap()
        .snapshot();
    let extensions = ExtensionSet::new()
        .with_extension(ObsidianSyntaxExtension)
        .with_extension(TokenScanner {
            id: ALPHA,
            token: "{{a}}",
            kind: "callout",
        });
    let (projection, snapshot) = project(&document, &extensions, RevealPolicy::ConcealInactive);
    let foreign = snapshot
        .matches()
        .iter()
        .find(|item| item.kind().extension() == ALPHA)
        .unwrap();
    let widget = &projection.widgets()[0];
    assert_eq!(widget.kind(), WidgetKind::Callout);
    let mut registry = HostRendererRegistry::<(), &'static str, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: ALPHA,
                name: "callout",
            },
            |_, _| Ok("foreign"),
        )
        .unwrap();
    assert!(matches!(
        registry.render(&mut (), HostRenderInput::Widget(widget)),
        Err(HostRenderError::Unregistered {
            key: HostRenderKey::Widget(WidgetKind::Callout),
            ..
        })
    ));
    registry
        .try_register(HostRenderKey::Widget(WidgetKind::Callout), |_, input| {
            let HostRenderInput::Widget(widget) = input else {
                panic!("expected widget");
            };
            assert_eq!(widget.callout().unwrap().callout_type(), "warning");
            Ok("builtin")
        })
        .unwrap();
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Widget(widget))
            .unwrap(),
        "builtin"
    );
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Extension(foreign))
            .unwrap(),
        "foreign"
    );
}

#[test]
fn duplicate_registration_preserves_the_original_handler() {
    let document = Document::new("---\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let key = HostRenderKey::Widget(WidgetKind::HorizontalRule);
    let mut registry = HostRendererRegistry::<(), &'static str, Infallible>::default();
    registry.try_register(key, |_, _| Ok("original")).unwrap();
    assert_eq!(
        registry.try_register(key, |_, _| Ok("replacement")),
        Err(HostRendererRegistrationError::DuplicateKey(key))
    );
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Widget(&projection.widgets()[0]))
            .unwrap(),
        "original"
    );
}

#[test]
fn preflight_reports_missing_concealed_widget_before_any_callback_and_allows_source_fallback() {
    let source = "- [ ] task\n\n---\n";
    let document = Document::new(source).unwrap().snapshot();
    let extensions = ExtensionSet::new();
    let (concealed, _) = project(&document, &extensions, RevealPolicy::ConcealInactive);
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::clone(&calls);
    let mut registry = HostRendererRegistry::<(), (), Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::HorizontalRule),
            move |_, _| {
                seen.set(seen.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(
        registry.check_widget_support(&concealed),
        Err(HostRenderError::Unregistered {
            key: HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            source_range: range(2, 5),
        })
    );
    assert_eq!(calls.get(), 0);
    let (fallback, _) = project(&document, &extensions, RevealPolicy::SourceVisible);
    registry.check_widget_support(&fallback).unwrap();
    assert_eq!(fallback.text(), source);
    assert_eq!(calls.get(), 0);
}

#[test]
fn preflight_checks_an_unsupported_widget_after_a_supported_widget_without_callbacks() {
    let document = Document::new("- [ ] task\n\n---\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::clone(&calls);
    let mut registry = HostRendererRegistry::<(), (), Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            move |_, _| {
                seen.set(seen.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(
        registry.check_widget_support(&projection),
        Err(HostRenderError::Unregistered {
            key: HostRenderKey::Widget(WidgetKind::HorizontalRule),
            source_range: range(12, 16),
        })
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn complete_preflight_does_not_invoke_stateful_view_thread_callbacks() {
    let document = Document::new("- [ ] task\n\n---\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let calls = Rc::new(Cell::new(0));
    let mut registry = HostRendererRegistry::<Vec<WidgetKind>, usize, Infallible>::new();
    for kind in [WidgetKind::TaskCheckbox, WidgetKind::HorizontalRule] {
        let seen = Rc::clone(&calls);
        let mut local_calls = 0;
        registry
            .try_register(HostRenderKey::Widget(kind), move |view, _| {
                local_calls += 1;
                seen.set(seen.get() + 1);
                view.push(kind);
                Ok(local_calls)
            })
            .unwrap();
    }
    registry.check_widget_support(&projection).unwrap();
    assert_eq!(calls.get(), 0);
    let mut view = Vec::new();
    for widget in projection.widgets() {
        assert_eq!(
            registry
                .render(&mut view, HostRenderInput::Widget(widget))
                .unwrap(),
            1
        );
        assert_eq!(
            registry
                .render(&mut view, HostRenderInput::Widget(widget))
                .unwrap(),
            2
        );
    }
    assert_eq!(calls.get(), 4);
    assert_eq!(
        view,
        vec![
            WidgetKind::TaskCheckbox,
            WidgetKind::TaskCheckbox,
            WidgetKind::HorizontalRule,
            WidgetKind::HorizontalRule
        ]
    );
}

#[derive(Debug, PartialEq, Eq)]
struct ResourceError(&'static str);
impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
impl Error for ResourceError {}

#[test]
fn failed_callback_preserves_typed_cause_key_range_and_prior_host_side_effects() {
    let document = Document::new("- [ ] task\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let mut registry = HostRendererRegistry::<usize, (), ResourceError>::new();
    registry
        .try_register(
            HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            |view, _| {
                *view += 1;
                Err(ResourceError("resource unavailable"))
            },
        )
        .unwrap();
    let mut view = 0;
    let error = registry
        .render(&mut view, HostRenderInput::Widget(&projection.widgets()[0]))
        .unwrap_err();
    assert_eq!(
        error,
        HostRenderError::RendererFailed {
            key: HostRenderKey::Widget(WidgetKind::TaskCheckbox),
            source_range: range(2, 5),
            error: ResourceError("resource unavailable"),
        }
    );
    assert_eq!(
        error.source().unwrap().downcast_ref::<ResourceError>(),
        Some(&ResourceError("resource unavailable"))
    );
    assert_eq!(view, 1);
    assert_eq!(document.text(), "- [ ] task\n");
}

#[test]
#[should_panic(expected = "host renderer panic")]
fn callback_panics_propagate_to_the_host() {
    let document = Document::new("---\n").unwrap().snapshot();
    let (projection, _) = project(
        &document,
        &ExtensionSet::new(),
        RevealPolicy::ConcealInactive,
    );
    let mut registry = HostRendererRegistry::<(), (), Infallible>::new();
    registry
        .try_register(HostRenderKey::Widget(WidgetKind::HorizontalRule), |_, _| {
            panic!("host renderer panic")
        })
        .unwrap();
    let _ = registry.render(&mut (), HostRenderInput::Widget(&projection.widgets()[0]));
}

#[test]
fn foreign_metadata_dispatch_does_not_conceal_source_or_create_native_widgets() {
    let source = "{{a}} **text**\n";
    let document = Document::new(source).unwrap().snapshot();
    let extensions = ExtensionSet::new().with_extension(TokenScanner {
        id: ALPHA,
        token: "{{a}}",
        kind: "embed",
    });
    let (projection, snapshot) = project(&document, &extensions, RevealPolicy::ConcealInactive);
    let mut registry = HostRendererRegistry::<(), String, Infallible>::new();
    registry
        .try_register(
            HostRenderKey::Extension {
                extension: ALPHA,
                name: "embed",
            },
            |_, input| {
                let HostRenderInput::Extension(item) = input else {
                    panic!("expected extension");
                };
                assert_eq!(input.source_range(), range(0, 5));
                Ok(item.attribute("label").unwrap().to_owned())
            },
        )
        .unwrap();
    registry.check_widget_support(&projection).unwrap();
    assert_eq!(
        registry
            .render(&mut (), HostRenderInput::Extension(&snapshot.matches()[0]))
            .unwrap(),
        "application.alpha"
    );
    assert_eq!(projection.text(), "{{a}} text\n");
    assert!(projection.widgets().is_empty());
    assert_eq!(document.text(), source);
}
