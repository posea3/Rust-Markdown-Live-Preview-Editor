use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use mdedit_core::Document;
use mdedit_markdown::{
    ExtensionCandidate, ExtensionId, ExtensionRegistrationError, ExtensionScanContext,
    ExtensionSet, ExtensionSnapshot, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    SyntaxExtension,
};

struct Claim {
    id: &'static str,
    token: &'static str,
}

impl SyntaxExtension for Claim {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        context
            .source()
            .match_indices(self.token)
            .map(|(start, token)| {
                ExtensionCandidate::new(
                    "claim",
                    context.source_range(start, start + token.len()).unwrap(),
                )
            })
            .collect()
    }
}

fn scan(source: &str, extensions: &ExtensionSet) -> ExtensionSnapshot {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    extensions.scan(&document, &syntax).unwrap()
}

#[test]
#[should_panic(expected = "example.shared")]
fn register_rejects_duplicate_namespaces() {
    let mut extensions = ExtensionSet::new();
    extensions.register(Claim {
        id: "example.shared",
        token: "first",
    });
    extensions.register(Claim {
        id: "example.shared",
        token: "second",
    });
}

#[test]
#[should_panic(expected = "example.shared")]
fn builder_rejects_duplicate_namespaces() {
    let _ = ExtensionSet::new()
        .with_extension(Claim {
            id: "example.shared",
            token: "first",
        })
        .with_extension(Claim {
            id: "example.shared",
            token: "second",
        });
}

struct MutableIdentity(Arc<AtomicBool>);

impl SyntaxExtension for MutableIdentity {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(if self.0.load(Ordering::Relaxed) {
            "example.changed"
        } else {
            "example.original"
        })
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        vec![ExtensionCandidate::new(
            "claim",
            context.source_range(0, 3).unwrap(),
        )]
    }
}

#[test]
fn scanner_matches_keep_the_registration_time_namespace() {
    let changed = Arc::new(AtomicBool::new(false));
    let extensions = ExtensionSet::new().with_extension(MutableIdentity(Arc::clone(&changed)));
    changed.store(true, Ordering::Relaxed);

    let snapshot = scan("한글", &extensions);
    assert_eq!(snapshot.matches().len(), 1);
    assert_eq!(
        snapshot.matches()[0].kind().extension().as_str(),
        "example.original"
    );
    assert_eq!(snapshot.matches()[0].range().as_usize_range(), 0..3);
}

#[test]
fn duplicate_error_preserves_registered_scanners_and_allows_distinct_ids() {
    let mut extensions = ExtensionSet::new();
    extensions
        .try_register(Claim {
            id: "example.shared",
            token: "한글",
        })
        .unwrap();
    let error = extensions
        .try_register(Claim {
            id: "example.shared",
            token: "second",
        })
        .unwrap_err();
    assert_eq!(
        error,
        ExtensionRegistrationError::DuplicateId(ExtensionId::new("example.shared"))
    );
    let public_error: &dyn std::error::Error = &error;
    assert!(public_error.to_string().contains("example.shared"));

    extensions
        .try_register(Claim {
            id: "example.shared.child",
            token: "third",
        })
        .unwrap();
    let snapshot = scan("한글 second third", &extensions);
    assert!(snapshot.issues().is_empty());
    assert_eq!(snapshot.matches().len(), 2);
    assert_eq!(snapshot.matches()[0].range().as_usize_range(), 0..6);
    assert_eq!(snapshot.matches()[1].range().as_usize_range(), 14..19);
    assert_eq!(
        snapshot.matches()[1].kind().extension().as_str(),
        "example.shared.child"
    );
}

#[test]
fn changing_scanner_identity_cannot_release_its_registered_namespace() {
    let changed = Arc::new(AtomicBool::new(false));
    let mut extensions = ExtensionSet::new();
    extensions
        .try_register(MutableIdentity(Arc::clone(&changed)))
        .unwrap();
    changed.store(true, Ordering::Relaxed);

    assert_eq!(
        extensions.try_register(Claim {
            id: "example.original",
            token: "글",
        }),
        Err(ExtensionRegistrationError::DuplicateId(ExtensionId::new(
            "example.original"
        )))
    );
    extensions
        .try_register(Claim {
            id: "example.changed",
            token: "#42",
        })
        .unwrap();
    let snapshot = scan("한글 #42", &extensions);
    assert!(snapshot.issues().is_empty());
    assert_eq!(snapshot.matches().len(), 2);
    assert_eq!(
        snapshot.matches()[0].kind().extension().as_str(),
        "example.original"
    );
    assert_eq!(
        snapshot.matches()[1].kind().extension().as_str(),
        "example.changed"
    );
}
