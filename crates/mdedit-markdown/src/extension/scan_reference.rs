// Frozen independent scan implementation from parent36a6dff; test-only.
use super::*;

pub(super) fn scan_reference(
    set: &ExtensionSet,
    document: &DocumentSnapshot,
    syntax: &SyntaxSnapshot,
) -> Result<ExtensionSnapshot, ExtensionError> {
    if syntax.status() != ParseStatus::Parsed {
        return Err(ExtensionError::SyntaxUnavailable(syntax.status()));
    }
    if document.revision() != syntax.revision() {
        return Err(ExtensionError::RevisionMismatch {
            document: document.revision(),
            syntax: syntax.revision(),
        });
    }

    let source_len = document
        .len()
        .map_err(|_| ExtensionError::DocumentTooLarge)?;
    if source_len != syntax.source_len() {
        return Err(ExtensionError::SourceLengthMismatch {
            document: source_len,
            syntax: syntax.source_len(),
        });
    }

    let configuration_id = set.configuration_id();
    let source = document.text();
    let protected = collect_protected_ranges(syntax.root());
    let context = ExtensionScanContext {
        source: &source,
        syntax,
    };

    let mut pending = Vec::new();
    let mut issues = Vec::new();

    for registered in &set.extensions {
        let id = registered.id;
        let extension = &registered.scanner;
        let mode = extension.scan_mode();

        let candidates =
            extension
                .scan(&context)
                .map_err(|error| ExtensionError::ScannerFailed {
                    extension: id,
                    error,
                })?;
        for candidate in candidates {
            let range = candidate.range.as_usize_range();
            if candidate.range.is_empty()
                || range.end > source.len()
                || !source.is_char_boundary(range.start)
                || !source.is_char_boundary(range.end)
            {
                issues.push(ExtensionIssue {
                    kind: ExtensionKind {
                        extension: id,
                        name: candidate.name,
                    },
                    range: candidate.range,
                    reason: ExtensionIssueReason::InvalidRange,
                });
                continue;
            }

            if mode == ExtensionScanMode::MarkdownText
                && let Some((_, core_kind)) = protected
                    .iter()
                    .find(|(protected_range, _)| ranges_overlap(*protected_range, candidate.range))
            {
                issues.push(ExtensionIssue {
                    kind: ExtensionKind {
                        extension: id,
                        name: candidate.name,
                    },
                    range: candidate.range,
                    reason: ExtensionIssueReason::SuppressedByCoreSyntax(*core_kind),
                });
                continue;
            }

            pending.push(ExtensionMatch {
                kind: ExtensionKind {
                    extension: id,
                    name: candidate.name,
                },
                range: candidate.range,
                priority: candidate.priority,
                overlap: candidate.overlap,
                attributes: candidate.attributes,
            });
        }
    }

    pending.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.range.start().cmp(&right.range.start()))
            .then_with(|| right.range.end().cmp(&left.range.end()))
            .then_with(|| left.kind.cmp(&right.kind))
    });

    let mut accepted: Vec<ExtensionMatch> = Vec::new();
    for candidate in pending {
        let conflict = accepted.iter().find(|existing| {
            ranges_overlap(existing.range, candidate.range) && !can_coexist(existing, &candidate)
        });

        if let Some(existing) = conflict {
            issues.push(ExtensionIssue {
                kind: candidate.kind,
                range: candidate.range,
                reason: ExtensionIssueReason::Conflict {
                    kept: existing.kind,
                },
            });
        } else {
            accepted.push(candidate);
        }
    }

    accepted.sort_by(|left, right| {
        left.range
            .start()
            .cmp(&right.range.start())
            .then_with(|| right.range.end().cmp(&left.range.end()))
            .then_with(|| left.kind.cmp(&right.kind))
    });

    Ok(ExtensionSnapshot {
        revision: syntax.revision(),
        source_len: syntax.source_len(),
        configuration_id,
        matches: accepted,
        issues,
    })
}
