#[path = "../examples/support/scanners.rs"]
mod scanners;

use mdedit_core::{Change, ChangeSet, Document, TextRange, TextSize, Transaction, TransactionKind};
use mdedit_markdown::{
    ExtensionCandidate, ExtensionError, ExtensionId, ExtensionIssueReason, ExtensionOverlapPolicy,
    ExtensionScanContext, ExtensionScanMode, ExtensionSet, ExtensionSnapshot, MarkdownDialect,
    MarkdownParser, PulldownCmarkParser, SyntaxExtension, SyntaxKind,
};
use scanners::{IssueScanner, ReferenceScanner};

fn scan(source: &str, extensions: &ExtensionSet) -> ExtensionSnapshot {
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    extensions.scan(&document, &syntax).unwrap()
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        TextSize::try_from_usize(start).unwrap(),
        TextSize::try_from_usize(end).unwrap(),
    )
    .unwrap()
}

struct LiteralClaim {
    id: &'static str,
    token: &'static str,
    priority: u16,
    overlap: ExtensionOverlapPolicy,
}

impl SyntaxExtension for LiteralClaim {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        context
            .source()
            .match_indices(self.token)
            .filter_map(|(start, token)| {
                context
                    .source_range(start, start + token.len())
                    .map(|range| {
                        ExtensionCandidate::new("claim", range)
                            .with_priority(self.priority)
                            .with_overlap_policy(self.overlap)
                    })
            })
            .collect()
    }
}

#[test]
fn sample_scanners_preserve_unicode_source_and_attributes() {
    let source = "한글 {{ref:설계 노트}} and #42";
    let snapshot = scan(
        source,
        &ExtensionSet::new()
            .with_extension(ReferenceScanner)
            .with_extension(IssueScanner),
    );

    assert!(snapshot.issues().is_empty());
    assert_eq!(snapshot.matches().len(), 2);
    let reference = &snapshot.matches()[0];
    assert_eq!(reference.kind().extension().as_str(), "example.references");
    assert_eq!(
        &source[reference.range().as_usize_range()],
        "{{ref:설계 노트}}"
    );
    assert_eq!(reference.attribute("target"), Some("설계 노트"));
    let issue = &snapshot.matches()[1];
    assert_eq!(&source[issue.range().as_usize_range()], "#42");
    assert_eq!(issue.attribute("number"), Some("42"));
}

#[test]
fn markdown_text_scanners_respect_code_and_html_protection() {
    let source = concat!(
        "{{ref:visible}}\n\n",
        "`{{ref:inline}}`\n\n",
        "```md\n{{ref:fenced}}\n```\n\n",
        "<div>\n{{ref:html}}\n</div>\n",
    );
    let snapshot = scan(
        source,
        &ExtensionSet::new().with_extension(ReferenceScanner),
    );

    assert_eq!(snapshot.matches().len(), 1);
    assert_eq!(snapshot.matches()[0].attribute("target"), Some("visible"));
    assert_eq!(snapshot.issues().len(), 3);
    for kind in [
        SyntaxKind::InlineCode,
        SyntaxKind::CodeBlock,
        SyntaxKind::HtmlBlock,
    ] {
        assert!(
            snapshot.issues().iter().any(|issue| {
                issue.reason() == ExtensionIssueReason::SuppressedByCoreSyntax(kind)
            })
        );
    }
}

struct AllSourceReferences;

impl SyntaxExtension for AllSourceReferences {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.inspect-source")
    }

    fn scan_mode(&self) -> ExtensionScanMode {
        ExtensionScanMode::AllSource
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        ReferenceScanner.scan(context)
    }
}

#[test]
fn all_source_is_an_explicit_opt_in_for_protected_text() {
    let source = "`{{ref:inside code}}`";
    let snapshot = scan(
        source,
        &ExtensionSet::new().with_extension(AllSourceReferences),
    );

    assert!(snapshot.issues().is_empty());
    assert_eq!(snapshot.matches().len(), 1);
    assert_eq!(
        &source[snapshot.matches()[0].range().as_usize_range()],
        "{{ref:inside code}}"
    );
}

#[test]
fn overlap_winner_uses_priority_then_qualified_identity_not_registration_order() {
    for (alpha_priority, omega_priority, winner) in [(100, 200, "omega"), (100, 100, "alpha")] {
        for reverse in [false, true] {
            let alpha = LiteralClaim {
                id: "alpha",
                token: "#42",
                priority: alpha_priority,
                overlap: ExtensionOverlapPolicy::Exclusive,
            };
            let omega = LiteralClaim {
                id: "omega",
                token: "#42",
                priority: omega_priority,
                overlap: ExtensionOverlapPolicy::Exclusive,
            };
            let extensions = if reverse {
                ExtensionSet::new()
                    .with_extension(omega)
                    .with_extension(alpha)
            } else {
                ExtensionSet::new()
                    .with_extension(alpha)
                    .with_extension(omega)
            };
            let snapshot = scan("#42", &extensions);

            assert_eq!(snapshot.matches().len(), 1);
            assert_eq!(snapshot.matches()[0].kind().extension().as_str(), winner);
            assert_eq!(snapshot.issues().len(), 1);
            match snapshot.issues()[0].reason() {
                ExtensionIssueReason::Conflict { kept } => {
                    assert_eq!(kept.extension().as_str(), winner);
                }
                other => panic!("expected a conflict diagnostic, got {other:?}"),
            }
        }
    }
}

#[test]
fn contained_claims_require_both_scanners_to_allow_containment() {
    for (inner_policy, accepted) in [
        (ExtensionOverlapPolicy::AllowContained, 2),
        (ExtensionOverlapPolicy::Exclusive, 1),
    ] {
        let extensions = ExtensionSet::new()
            .with_extension(LiteralClaim {
                id: "outer",
                token: "{{ref:#42}}",
                priority: 100,
                overlap: ExtensionOverlapPolicy::AllowContained,
            })
            .with_extension(LiteralClaim {
                id: "inner",
                token: "#42",
                priority: 100,
                overlap: inner_policy,
            });
        let snapshot = scan("{{ref:#42}}", &extensions);

        assert_eq!(snapshot.matches().len(), accepted);
        assert_eq!(snapshot.matches()[0].kind().extension().as_str(), "outer");
        assert_eq!(snapshot.issues().len(), 2 - accepted);
    }
}

#[test]
fn allow_contained_does_not_allow_crossing_claims() {
    let extensions = ExtensionSet::new()
        .with_extension(LiteralClaim {
            id: "left",
            token: "abcd",
            priority: 100,
            overlap: ExtensionOverlapPolicy::AllowContained,
        })
        .with_extension(LiteralClaim {
            id: "right",
            token: "cdef",
            priority: 100,
            overlap: ExtensionOverlapPolicy::AllowContained,
        });
    let snapshot = scan("abcdef", &extensions);

    assert_eq!(snapshot.matches().len(), 1);
    assert_eq!(snapshot.matches()[0].range(), range(0, 4));
    assert_eq!(snapshot.issues().len(), 1);
}

struct InvalidRanges;

impl SyntaxExtension for InvalidRanges {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.invalid-ranges")
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        vec![
            ExtensionCandidate::new("invalid-utf8", range(1, 2)),
            ExtensionCandidate::new("empty", range(0, 0)),
            ExtensionCandidate::new("past-end", range(0, context.source().len() + 1)),
            ExtensionCandidate::new("valid", range(3, 6)),
        ]
    }
}

#[test]
fn invalid_scanner_ranges_are_diagnosed_without_discarding_valid_matches() {
    let snapshot = scan("한글", &ExtensionSet::new().with_extension(InvalidRanges));

    assert_eq!(snapshot.matches().len(), 1);
    assert_eq!(snapshot.matches()[0].range(), range(3, 6));
    assert_eq!(snapshot.issues().len(), 3);
    assert!(
        snapshot
            .issues()
            .iter()
            .all(|issue| issue.reason() == ExtensionIssueReason::InvalidRange)
    );
}

#[test]
fn scanning_rejects_stale_syntax_even_when_source_length_is_unchanged() {
    let mut document = Document::new("#42").unwrap();
    let syntax = PulldownCmarkParser.parse(&document.snapshot(), &MarkdownDialect::gfm());
    document
        .apply(Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range(2, 3), "3")),
            TransactionKind::Programmatic,
        ))
        .unwrap();

    let result = ExtensionSet::new().scan(&document.snapshot(), &syntax);
    assert!(
        matches!(result, Err(ExtensionError::RevisionMismatch { document, syntax }) if document.get() == 1 && syntax.get() == 0)
    );
}

#[test]
fn scanning_rejects_same_revision_syntax_with_a_different_source_length() {
    let original = Document::new("#42").unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&original, &MarkdownDialect::gfm());
    let other = Document::new("#420").unwrap().snapshot();

    let result = ExtensionSet::new().scan(&other, &syntax);
    assert!(
        matches!(result, Err(ExtensionError::SourceLengthMismatch { document, syntax }) if document.get() == 4 && syntax.get() == 3)
    );
}
