use super::{index::OverlapIndex, scan_reference::scan_reference, *};
use crate::{MarkdownDialect, MarkdownParser, PulldownCmarkParser, SyntaxExtension};
use mdedit_core::Document;
use std::sync::{Arc, Mutex};

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

#[test]
fn overlap_index_matches_all_small_subsets_activation_orders_and_predicates() {
    let ranges = (0..=4)
        .flat_map(|start| (start..=4).map(move |end| range(start, end)))
        .collect::<Vec<_>>();
    for mask in 0..1_u32 << ranges.len() {
        let mut index = OverlapIndex::new(ranges.iter().copied());
        let mut active = Vec::new();
        for (slot, source) in ranges.iter().enumerate().rev() {
            if mask & (1 << slot) != 0 {
                index.activate(slot, active.len());
                active.push(*source);
            }
        }
        for start in 0..=5 {
            for end in start..=5 {
                let query = range(start, end);
                for filtered in [false, true] {
                    let expected = active
                        .iter()
                        .enumerate()
                        .find(|(ordinal, source)| {
                            ranges_overlap(**source, query) && (!filtered || ordinal % 3 != 0)
                        })
                        .map(|(ordinal, _)| ordinal);
                    assert_eq!(
                        index.first_matching(query, |ordinal| !filtered || ordinal % 3 != 0),
                        expected,
                        "mask={mask}, query={query:?}"
                    );
                }
            }
        }
    }
    let ranges = [
        range(0, 0),
        range(0, u32::MAX),
        range(u32::MAX - 3, u32::MAX),
        range(u32::MAX, u32::MAX),
    ];
    let mut index = OverlapIndex::new(ranges);
    for slot in 0..ranges.len() {
        index.activate(slot, slot);
    }
    for query in [
        range(0, 0),
        range(0, 1),
        range(u32::MAX - 2, u32::MAX - 1),
        range(u32::MAX, u32::MAX),
    ] {
        assert_eq!(
            index.first_matching(query, |_| true),
            ranges
                .iter()
                .position(|source| ranges_overlap(*source, query))
        );
    }
}

#[test]
fn disjoint_queries_remain_bounded_after_reverse_activation() {
    let ranges = (0..4096)
        .map(|index| range(index * 4, index * 4 + 3))
        .collect::<Vec<_>>();
    let mut lookup = OverlapIndex::new(ranges.iter().copied());
    for slot in (0..ranges.len()).rev() {
        lookup.activate(slot, ranges.len() - 1 - slot);
    }
    for (slot, source) in ranges.iter().enumerate() {
        let (actual, visits) = lookup.first_matching_with_visits(*source, |_| true);
        assert_eq!(actual, Some(ranges.len() - 1 - slot));
        assert!(visits <= 80, "disjoint query visited {visits} nodes");
    }
}

struct Claims {
    id: &'static str,
    mode: ExtensionScanMode,
    claims: Vec<ExtensionCandidate>,
}
impl SyntaxExtension for Claims {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }
    fn scan_mode(&self) -> ExtensionScanMode {
        self.mode
    }
    fn scan(&self, _: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        self.claims.clone()
    }
}

fn candidate(
    name: &'static str,
    start: u32,
    end: u32,
    priority: u16,
    allow: bool,
) -> ExtensionCandidate {
    ExtensionCandidate::new(name, range(start, end))
        .with_priority(priority)
        .with_overlap_policy(if allow {
            ExtensionOverlapPolicy::AllowContained
        } else {
            ExtensionOverlapPolicy::Exclusive
        })
        .with_attribute("label", format!("한🙂:{name}:{start}:{end}"))
        .with_attribute("label", "duplicate-key-order")
}

#[test]
fn generated_full_snapshots_preserve_order_attributes_protection_and_invalid_ranges() {
    let source = "a한🙂\r\nb `raw`\n\n```md\nblocked\n```\n\n<div>x</div>\n tail";
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
    let mut seed = 0x5eed1234_u32;
    fn next(seed: &mut u32) -> u32 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 17;
        *seed ^= *seed << 5;
        *seed
    }
    for iteration in 0..512 {
        let mut set = ExtensionSet::new();
        for (scanner, id) in ["zeta", "alpha", "middle"].into_iter().enumerate() {
            let mut claims = Vec::new();
            for item in 0..next(&mut seed) % 19 {
                let a = next(&mut seed) % (source.len() as u32 + 3);
                let b = next(&mut seed) % (source.len() as u32 + 3);
                claims.push(candidate(
                    ["a", "b", "same"][item as usize % 3],
                    a.min(b),
                    a.max(b),
                    [0, 1, 100, u16::MAX][next(&mut seed) as usize % 4],
                    next(&mut seed) & 1 != 0,
                ));
            }
            set.register(Claims {
                id,
                mode: if (iteration + scanner) % 2 == 0 {
                    ExtensionScanMode::MarkdownText
                } else {
                    ExtensionScanMode::AllSource
                },
                claims,
            });
        }
        let expected = scan_reference(&set, &document, &syntax).unwrap();
        let actual = set.scan(&document, &syntax).unwrap();
        assert_eq!(actual, expected, "iteration={iteration}");
        assert_eq!(document.text(), source);
    }
}

#[test]
fn real_obsidian_and_deliberate_nested_crossing_claims_match_parent_snapshots() {
    let sources = [
        "",
        "한🙂 [[note#part|name]] ==mark== %%comment%%\r\n",
        "> [!note] title\r\n> **body**\r\n\r\n![[file#^id|name]]\r\n",
        "`[[hidden]]`\n\n```md\n[[fenced]]\n```\n\n<div>[[html]]</div>\n",
        "%% [[comment]] ==mark== %% [[visible]] ^block\n",
        "abcdefghijklmno",
    ];
    for source in sources {
        let document = Document::new(source).unwrap().snapshot();
        let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::obsidian());
        let set = crate::obsidian_extension_set().with_extension(Claims {
            id: "custom",
            mode: ExtensionScanMode::AllSource,
            claims: vec![
                candidate("outer", 0, 12, 400, true),
                candidate("inner", 2, 6, 300, true),
                candidate("equal", 2, 6, 200, true),
                candidate("crossing", 4, 14, 100, true),
                candidate("touch", 12, 15, 1, false),
            ],
        });
        assert_eq!(
            set.scan(&document, &syntax).unwrap(),
            scan_reference(&set, &document, &syntax).unwrap()
        );
    }
}

#[test]
fn issues_keep_first_acceptance_and_first_protected_syntax_precedence() {
    let document = Document::new("abcdefghijkl").unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    let set = ExtensionSet::new().with_extension(Claims {
        id: "claims",
        mode: ExtensionScanMode::AllSource,
        claims: vec![
            candidate("late-source-first-accepted", 6, 10, 300, false),
            candidate("early-source-second-accepted", 0, 4, 200, false),
            candidate("crossing", 2, 8, 100, false),
            candidate("rejected-tail", 4, 5, 50, false),
        ],
    });
    let actual = set.scan(&document, &syntax).unwrap();
    assert_eq!(actual, scan_reference(&set, &document, &syntax).unwrap());
    assert!(
        matches!(actual.issues()[0].reason(), ExtensionIssueReason::Conflict { kept } if kept.name() == "late-source-first-accepted")
    );
    assert_eq!(actual.issues().len(), 1);
    assert!(
        actual
            .matches()
            .iter()
            .any(|item| item.kind().name() == "rejected-tail")
    );
    let source = "`code` and <b>x</b>";
    let document = Document::new(source).unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    let set = ExtensionSet::new().with_extension(Claims {
        id: "claims",
        mode: ExtensionScanMode::MarkdownText,
        claims: vec![
            candidate("invalid", 0, source.len() as u32 + 1, 100, false),
            candidate("whole", 0, source.len() as u32, 100, false),
        ],
    });
    let actual = set.scan(&document, &syntax).unwrap();
    assert_eq!(actual, scan_reference(&set, &document, &syntax).unwrap());
    assert_eq!(
        actual.issues()[0].reason(),
        ExtensionIssueReason::InvalidRange
    );
    assert_eq!(
        actual.issues()[1].reason(),
        ExtensionIssueReason::SuppressedByCoreSyntax(SyntaxKind::InlineCode)
    );
}

struct Recording {
    id: &'static str,
    events: Arc<Mutex<Vec<String>>>,
    fail: bool,
    panic: bool,
}
impl crate::FallibleSyntaxExtension for Recording {
    fn id(&self) -> ExtensionId {
        ExtensionId::new(self.id)
    }
    fn scan_mode(&self) -> ExtensionScanMode {
        self.events
            .lock()
            .unwrap()
            .push(format!("mode:{}", self.id));
        ExtensionScanMode::MarkdownText
    }
    fn try_scan(
        &self,
        _: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, crate::ExtensionScanError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("scan:{}", self.id));
        assert!(!self.panic, "scanner panic");
        if self.fail {
            Err(crate::ExtensionScanError::new("declared failure"))
        } else {
            Ok(vec![candidate("claim", 0, 1, 100, false)])
        }
    }
}

#[test]
fn prevalidation_failure_and_panic_keep_observable_callback_traces() {
    let document = Document::new("abc").unwrap().snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    for panic in [false, true] {
        let events = Arc::new(Mutex::new(Vec::new()));
        let set = ExtensionSet::new()
            .with_fallible_extension(Recording {
                id: "before",
                events: events.clone(),
                fail: false,
                panic: false,
            })
            .with_fallible_extension(Recording {
                id: "stop",
                events: events.clone(),
                fail: !panic,
                panic,
            })
            .with_fallible_extension(Recording {
                id: "after",
                events: events.clone(),
                fail: false,
                panic: false,
            });
        let expected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scan_reference(&set, &document, &syntax)
        }));
        let expected_events = std::mem::take(&mut *events.lock().unwrap());
        let actual = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            set.scan(&document, &syntax)
        }));
        assert_eq!(*events.lock().unwrap(), expected_events);
        assert_eq!(
            expected_events,
            vec!["mode:before", "scan:before", "mode:stop", "scan:stop"]
        );
        if panic {
            assert!(actual.is_err() && expected.is_err());
        } else {
            assert_eq!(actual.unwrap(), expected.unwrap());
        }
        events.lock().unwrap().clear();
        let other = Document::new("different length").unwrap().snapshot();
        let wrong = PulldownCmarkParser.parse(&other, &MarkdownDialect::gfm());
        assert_eq!(
            set.scan(&document, &wrong),
            scan_reference(&set, &document, &wrong)
        );
        assert!(events.lock().unwrap().is_empty());
    }
}
