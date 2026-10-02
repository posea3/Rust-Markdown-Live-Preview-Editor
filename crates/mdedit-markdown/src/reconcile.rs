use std::{error::Error, fmt};

use mdedit_core::{DocumentSnapshot, Revision, TextSize};

use crate::SyntaxSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ParseRequestId(u64);

impl ParseRequestId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ParseConfigEpoch(u64);

impl ParseConfigEpoch {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseTicket {
    request_id: ParseRequestId,
    revision: Revision,
    source_len: TextSize,
    config_epoch: ParseConfigEpoch,
}

impl ParseTicket {
    #[must_use]
    pub const fn request_id(self) -> ParseRequestId {
        self.request_id
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn source_len(self) -> TextSize {
        self.source_len
    }

    #[must_use]
    pub const fn config_epoch(self) -> ParseConfigEpoch {
        self.config_epoch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseDiscardReason {
    DocumentChanged {
        requested: Revision,
        current: Revision,
    },
    ConfigurationChanged {
        requested: ParseConfigEpoch,
        current: ParseConfigEpoch,
    },
    Superseded {
        requested: ParseRequestId,
        latest: ParseRequestId,
    },
    AlreadyAccepted {
        requested: ParseRequestId,
        accepted: ParseRequestId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseReconcileOutcome {
    Accepted {
        request_id: ParseRequestId,
        revision: Revision,
    },
    Discarded(ParseDiscardReason),
}

#[derive(Clone, Debug)]
pub struct ParseReconciler {
    next_request_id: u64,
    config_epoch: ParseConfigEpoch,
    latest_requested: Option<ParseRequestId>,
    accepted_request: Option<ParseRequestId>,
    current: Option<SyntaxSnapshot>,
}

impl Default for ParseReconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseReconciler {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_request_id: 1,
            config_epoch: ParseConfigEpoch(0),
            latest_requested: None,
            accepted_request: None,
            current: None,
        }
    }

    #[must_use]
    pub const fn config_epoch(&self) -> ParseConfigEpoch {
        self.config_epoch
    }

    #[must_use]
    pub fn current(&self) -> Option<&SyntaxSnapshot> {
        self.current.as_ref()
    }

    pub fn request(
        &mut self,
        document: &DocumentSnapshot,
    ) -> Result<ParseTicket, ParseReconcileError> {
        let source_len = document
            .len()
            .map_err(|_| ParseReconcileError::DocumentTooLarge)?;
        let request_id = ParseRequestId(self.next_request_id);
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(ParseReconcileError::RequestIdExhausted)?;
        self.latest_requested = Some(request_id);

        Ok(ParseTicket {
            request_id,
            revision: document.revision(),
            source_len,
            config_epoch: self.config_epoch,
        })
    }

    pub fn invalidate_configuration(
        &mut self,
    ) -> Result<ParseConfigEpoch, ParseReconcileError> {
        let next = self
            .config_epoch
            .0
            .checked_add(1)
            .ok_or(ParseReconcileError::ConfigEpochExhausted)?;
        self.config_epoch = ParseConfigEpoch(next);
        self.latest_requested = None;
        self.accepted_request = None;
        self.current = None;
        Ok(self.config_epoch)
    }

    pub fn reconcile(
        &mut self,
        document: &DocumentSnapshot,
        ticket: ParseTicket,
        syntax: SyntaxSnapshot,
    ) -> Result<ParseReconcileOutcome, ParseReconcileError> {
        validate_ticket_result(ticket, &syntax)?;

        if ticket.config_epoch != self.config_epoch {
            return Ok(ParseReconcileOutcome::Discarded(
                ParseDiscardReason::ConfigurationChanged {
                    requested: ticket.config_epoch,
                    current: self.config_epoch,
                },
            ));
        }

        if document.revision() != ticket.revision {
            return Ok(ParseReconcileOutcome::Discarded(
                ParseDiscardReason::DocumentChanged {
                    requested: ticket.revision,
                    current: document.revision(),
                },
            ));
        }

        let current_len = document
            .len()
            .map_err(|_| ParseReconcileError::DocumentTooLarge)?;
        if current_len != ticket.source_len {
            return Err(ParseReconcileError::CurrentDocumentLengthMismatch {
                ticket: ticket.source_len,
                current: current_len,
            });
        }

        if let Some(latest) = self.latest_requested
            && ticket.request_id != latest
        {
            return Ok(ParseReconcileOutcome::Discarded(
                ParseDiscardReason::Superseded {
                    requested: ticket.request_id,
                    latest,
                },
            ));
        }

        if let Some(accepted) = self.accepted_request
            && ticket.request_id <= accepted
        {
            return Ok(ParseReconcileOutcome::Discarded(
                ParseDiscardReason::AlreadyAccepted {
                    requested: ticket.request_id,
                    accepted,
                },
            ));
        }

        self.accepted_request = Some(ticket.request_id);
        self.current = Some(syntax);

        Ok(ParseReconcileOutcome::Accepted {
            request_id: ticket.request_id,
            revision: ticket.revision,
        })
    }
}

fn validate_ticket_result(
    ticket: ParseTicket,
    syntax: &SyntaxSnapshot,
) -> Result<(), ParseReconcileError> {
    if syntax.revision() != ticket.revision {
        return Err(ParseReconcileError::ResultRevisionMismatch {
            ticket: ticket.revision,
            syntax: syntax.revision(),
        });
    }
    if syntax.source_len() != ticket.source_len {
        return Err(ParseReconcileError::ResultLengthMismatch {
            ticket: ticket.source_len,
            syntax: syntax.source_len(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseReconcileError {
    ResultRevisionMismatch {
        ticket: Revision,
        syntax: Revision,
    },
    ResultLengthMismatch {
        ticket: TextSize,
        syntax: TextSize,
    },
    CurrentDocumentLengthMismatch {
        ticket: TextSize,
        current: TextSize,
    },
    DocumentTooLarge,
    RequestIdExhausted,
    ConfigEpochExhausted,
}

impl fmt::Display for ParseReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultRevisionMismatch { ticket, syntax } => write!(
                formatter,
                "parse ticket revision {} does not match syntax revision {}",
                ticket.get(),
                syntax.get()
            ),
            Self::ResultLengthMismatch { ticket, syntax } => write!(
                formatter,
                "parse ticket source length {} does not match syntax length {}",
                ticket.get(),
                syntax.get()
            ),
            Self::CurrentDocumentLengthMismatch { ticket, current } => write!(
                formatter,
                "parse ticket source length {} does not match current document length {}",
                ticket.get(),
                current.get()
            ),
            Self::DocumentTooLarge => {
                write!(formatter, "document exceeds the supported source size")
            }
            Self::RequestIdExhausted => write!(formatter, "parse request id space is exhausted"),
            Self::ConfigEpochExhausted => {
                write!(formatter, "parse configuration epoch space is exhausted")
            }
        }
    }
}

impl Error for ParseReconcileError {}

#[cfg(test)]
mod tests {
    use mdedit_core::{
        Change, ChangeSet, Document, TextRange, TextSize, Transaction, TransactionKind,
    };

    use super::*;
    use crate::{
        MarkdownDialect, MarkdownParser, ParseStatus, PulldownCmarkParser,
        RawFallbackReason, SyntaxKind, SyntaxNode,
    };

    fn parse(document: &Document) -> SyntaxSnapshot {
        PulldownCmarkParser.parse(&document.snapshot(), &MarkdownDialect::commonmark())
    }

    fn replace(document: &mut Document, start: u32, end: u32, text: &str) {
        let range = TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap();
        let transaction = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range, text)),
            TransactionKind::Programmatic,
        );
        document.apply(transaction).unwrap();
    }

    #[test]
    fn accepts_latest_result_for_current_document() {
        let document = Document::new("# A\n").unwrap();
        let mut reconciler = ParseReconciler::new();
        let ticket = reconciler.request(&document.snapshot()).unwrap();
        let syntax = parse(&document);

        let outcome = reconciler
            .reconcile(&document.snapshot(), ticket, syntax)
            .unwrap();

        assert_eq!(
            outcome,
            ParseReconcileOutcome::Accepted {
                request_id: ticket.request_id(),
                revision: document.revision(),
            }
        );
        assert_eq!(
            reconciler.current().map(SyntaxSnapshot::revision),
            Some(document.revision())
        );
    }

    #[test]
    fn discards_result_when_document_advanced() {
        let mut document = Document::new("old\n").unwrap();
        let old_snapshot = document.snapshot();
        let mut reconciler = ParseReconciler::new();
        let ticket = reconciler.request(&old_snapshot).unwrap();
        let syntax = PulldownCmarkParser.parse(&old_snapshot, &MarkdownDialect::commonmark());

        replace(&mut document, 0, 3, "new");
        let outcome = reconciler
            .reconcile(&document.snapshot(), ticket, syntax)
            .unwrap();

        assert!(matches!(
            outcome,
            ParseReconcileOutcome::Discarded(ParseDiscardReason::DocumentChanged { .. })
        ));
        assert!(reconciler.current().is_none());
    }

    #[test]
    fn later_request_supersedes_earlier_result_even_for_same_revision() {
        let document = Document::new("same\n").unwrap();
        let snapshot = document.snapshot();
        let mut reconciler = ParseReconciler::new();
        let first = reconciler.request(&snapshot).unwrap();
        let second = reconciler.request(&snapshot).unwrap();

        let first_result = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::commonmark());
        let outcome = reconciler
            .reconcile(&snapshot, first, first_result)
            .unwrap();

        assert_eq!(
            outcome,
            ParseReconcileOutcome::Discarded(ParseDiscardReason::Superseded {
                requested: first.request_id(),
                latest: second.request_id(),
            })
        );

        let second_result = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::commonmark());
        assert!(matches!(
            reconciler
                .reconcile(&snapshot, second, second_result)
                .unwrap(),
            ParseReconcileOutcome::Accepted { .. }
        ));
    }

    #[test]
    fn configuration_invalidation_rejects_old_results_and_clears_current() {
        let document = Document::new("text\n").unwrap();
        let snapshot = document.snapshot();
        let mut reconciler = ParseReconciler::new();

        let accepted_ticket = reconciler.request(&snapshot).unwrap();
        let accepted = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::commonmark());
        reconciler
            .reconcile(&snapshot, accepted_ticket, accepted)
            .unwrap();
        assert!(reconciler.current().is_some());

        let old_ticket = reconciler.request(&snapshot).unwrap();
        let old_result = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::commonmark());
        let new_epoch = reconciler.invalidate_configuration().unwrap();

        let outcome = reconciler
            .reconcile(&snapshot, old_ticket, old_result)
            .unwrap();

        assert_eq!(
            outcome,
            ParseReconcileOutcome::Discarded(
                ParseDiscardReason::ConfigurationChanged {
                    requested: old_ticket.config_epoch(),
                    current: new_epoch,
                }
            )
        );
        assert!(reconciler.current().is_none());
    }

    #[test]
    fn mismatched_ticket_result_is_an_error_not_a_stale_discard() {
        let first = Document::new("a\n").unwrap();
        let second = Document::new("longer\n").unwrap();
        let mut reconciler = ParseReconciler::new();
        let ticket = reconciler.request(&first.snapshot()).unwrap();
        let foreign = parse(&second);

        let error = reconciler
            .reconcile(&first.snapshot(), ticket, foreign)
            .unwrap_err();

        assert!(matches!(
            error,
            ParseReconcileError::ResultLengthMismatch { .. }
        ));
    }

    #[test]
    fn accepts_current_raw_fallback_as_safe_parse_result() {
        let document = Document::new("raw\n").unwrap();
        let snapshot = document.snapshot();
        let mut reconciler = ParseReconciler::new();
        let ticket = reconciler.request(&snapshot).unwrap();
        let len = snapshot.len().unwrap();
        let raw = SyntaxSnapshot::raw_fallback(
            snapshot.revision(),
            len,
            RawFallbackReason::UnbalancedEvents,
        );

        let outcome = reconciler.reconcile(&snapshot, ticket, raw).unwrap();

        assert!(matches!(outcome, ParseReconcileOutcome::Accepted { .. }));
        assert_eq!(
            reconciler.current().map(SyntaxSnapshot::status),
            Some(ParseStatus::RawFallback(
                RawFallbackReason::UnbalancedEvents
            ))
        );
    }

    #[test]
    fn accepted_snapshot_remains_project_owned_and_source_mapped() {
        let document = Document::new("**bold**\n").unwrap();
        let snapshot = document.snapshot();
        let mut reconciler = ParseReconciler::new();
        let ticket = reconciler.request(&snapshot).unwrap();
        let syntax = PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::commonmark());

        reconciler.reconcile(&snapshot, ticket, syntax).unwrap();
        let root = reconciler.current().unwrap().root();

        assert_eq!(root.kind(), SyntaxKind::Document);
        assert_eq!(root.range().as_usize_range(), 0..snapshot.text().len());
        assert!(
            root.children()
                .iter()
                .flat_map(SyntaxNode::children)
                .any(|node| node.kind() == SyntaxKind::Strong)
        );
    }
}
