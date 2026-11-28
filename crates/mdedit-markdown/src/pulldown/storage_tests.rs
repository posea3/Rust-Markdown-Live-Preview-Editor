use super::reference::ReferenceParser;
use super::*;
use mdedit_core::Document;
use std::{cell::Cell, rc::Rc};

#[test]
fn full_syntax_matches_parent_across_dialects_metadata_unicode_and_generated_markup() {
    let fragments = [
        "plain 한🙂\r\n",
        "# title **bold**\r\n\r\n",
        "***a\u{301}*** and ~~strike~~\n\n",
        "[link](a\\(b\\) \"title\")\n\n",
        "![alt *x*](img.png)\n\n",
        "`code  한` $x^2$\n\n",
        "```rust\r\nlet x=1;\r\n```\r\n\r\n",
        "| a | b |\n| :- | -: |\n| 1 | 2 |\n\n",
        "> [!note] title\n> body **x**\n\n",
        "- [ ] task\n- item\n\n",
        "---\n\n",
        "<div>raw &amp;</div>\n\n",
        "[^a]: note\n\nref [^a]\n\n",
    ];
    for dialect in [
        MarkdownDialect::commonmark(),
        MarkdownDialect::gfm(),
        MarkdownDialect::obsidian(),
    ] {
        for sample in 0..256 {
            let mut source = String::new();
            for offset in 0..1 + sample % 7 {
                source.push_str(fragments[(sample * 5 + offset * 3) % fragments.len()]);
            }
            let snapshot = Document::new(&source).unwrap().snapshot();
            assert_eq!(
                PulldownCmarkParser.parse(&snapshot, &dialect),
                ReferenceParser.parse(&snapshot, &dialect),
                "sample={sample}, dialect={dialect:?}"
            );
        }
    }
}

#[test]
fn malformed_events_preserve_first_fallback_and_stop_consuming() {
    let cases = vec![
        vec![(Event::Text("x".into()), 1..2)],
        vec![(Event::End(TagEnd::Paragraph), 0..1)],
        vec![
            (Event::Start(Tag::Paragraph), 0..3),
            (Event::End(TagEnd::Strong), 0..3),
        ],
        vec![(Event::Start(Tag::Paragraph), 0..3)],
    ];
    for events in cases {
        let seen = Rc::new(Cell::new(0));
        let iter = events
            .clone()
            .into_iter()
            .inspect(|_| seen.set(seen.get() + 1));
        let actual = build_snapshot(Revision::initial(), "한", iter);
        let count = seen.get();
        seen.set(0);
        let expected = reference::build_snapshot(
            Revision::initial(),
            "한",
            events.into_iter().inspect(|_| seen.set(seen.get() + 1)),
        );
        assert_eq!(actual, expected);
        assert_eq!(count, seen.get());
        assert!(matches!(
            actual.status(),
            crate::ParseStatus::RawFallback(_)
        ));
    }
    let events = vec![
        (Event::End(TagEnd::Paragraph), 0..1),
        (Event::Text("later".into()), 0..1),
    ];
    let seen = Cell::new(0);
    let snapshot = build_snapshot(
        Revision::initial(),
        "a",
        events.into_iter().inspect(|_| seen.set(seen.get() + 1)),
    );
    assert_eq!(
        snapshot.status(),
        crate::ParseStatus::RawFallback(RawFallbackReason::UnbalancedEvents)
    );
    assert_eq!(seen.get(), 1);
}
