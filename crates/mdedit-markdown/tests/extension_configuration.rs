use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use mdedit_core::Document;
use mdedit_markdown::{
    ExtensionCandidate, ExtensionId, ExtensionScanContext, ExtensionSet, ExtensionSnapshot,
    MarkdownDialect, MarkdownParser, PulldownCmarkParser, SyntaxExtension,
};

struct Scanner {
    id: &'static str,
    enabled: Arc<AtomicBool>,
}

impl SyntaxExtension for Scanner {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        if self.enabled.load(Ordering::Relaxed) {
            vec![ExtensionCandidate::new(
                "marker",
                context.source_range(0, context.source().len()).unwrap(),
            )]
        } else {
            Vec::new()
        }
    }
}

fn scanner(id: &'static str) -> Scanner {
    Scanner {
        id,
        enabled: Arc::new(AtomicBool::new(true)),
    }
}

fn scan(source: &str, extensions: &ExtensionSet) -> ExtensionSnapshot {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    extensions.scan(&document, &syntax).unwrap()
}

#[test]
fn stable_configuration_is_preserved_by_scans_and_snapshot_clones() {
    static EXTENSIONS: ExtensionSet = ExtensionSet::new();
    let id = EXTENSIONS.configuration_id();
    let first = scan("한글", &EXTENSIONS);
    let other_document = scan("different source", &EXTENSIONS);

    assert_eq!(first.configuration_id(), id);
    assert_eq!(first.clone(), first);
    assert_eq!(first.clone().configuration_id(), id);
    assert_eq!(other_document.configuration_id(), id);
    assert_eq!(EXTENSIONS.configuration_id(), id);
}

#[test]
fn unrelated_sets_have_distinct_ids_even_with_equal_registrations() {
    let mut left = ExtensionSet::new();
    let mut right = ExtensionSet::default();
    let left_empty = scan("marker", &left);
    let right_empty = scan("marker", &right);
    assert_ne!(
        left_empty.configuration_id(),
        right_empty.configuration_id()
    );
    assert_ne!(left_empty, right_empty);

    left.register(scanner("example.same"));
    right.register(scanner("example.same"));
    let left_result = scan("marker", &left);
    let right_result = scan("marker", &right);
    assert_eq!(left_result.matches(), right_result.matches());
    assert_ne!(
        left_result.configuration_id(),
        right_result.configuration_id()
    );
}

#[test]
fn registration_invalidates_identity_but_duplicate_rejection_preserves_it() {
    let mut extensions = ExtensionSet::new();
    let empty_id = extensions.configuration_id();
    extensions.try_register(scanner("example.first")).unwrap();
    let accepted = scan("marker", &extensions);
    assert_ne!(accepted.configuration_id(), empty_id);

    assert!(extensions.try_register(scanner("example.first")).is_err());
    assert_eq!(extensions.configuration_id(), accepted.configuration_id());
    assert_eq!(scan("marker", &extensions), accepted);

    extensions.try_register(scanner("example.second")).unwrap();
    assert_ne!(extensions.configuration_id(), accepted.configuration_id());
}

#[test]
fn explicit_invalidation_tracks_mutable_scanner_settings() {
    let enabled = Arc::new(AtomicBool::new(true));
    let mut extensions = ExtensionSet::new().with_extension(Scanner {
        id: "example.mutable",
        enabled: Arc::clone(&enabled),
    });
    let cached = scan("marker", &extensions);
    assert_eq!(cached.matches().len(), 1);

    enabled.store(false, Ordering::Relaxed);
    extensions.invalidate_configuration();
    let refreshed = scan("marker", &extensions);
    assert_ne!(refreshed.configuration_id(), cached.configuration_id());
    assert_eq!(refreshed.configuration_id(), extensions.configuration_id());
    assert!(refreshed.matches().is_empty());
    assert_eq!(cached.matches().len(), 1);
}

#[test]
fn shared_set_has_one_identity_when_first_observed_concurrently() {
    let extensions = Arc::new(ExtensionSet::new());
    let workers = (0..8)
        .map(|_| {
            let extensions = Arc::clone(&extensions);
            std::thread::spawn(move || extensions.configuration_id())
        })
        .collect::<Vec<_>>();
    let ids = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.iter().all(|id| *id == extensions.configuration_id()));
}
