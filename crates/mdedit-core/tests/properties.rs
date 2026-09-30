use mdedit_core::{
    Affinity, Change, ChangeSet, Document, TextRange, TextSize, Transaction, TransactionKind,
};
use proptest::prelude::*;

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        TextSize::try_from_usize(start).unwrap(),
        TextSize::try_from_usize(end).unwrap(),
    )
    .unwrap()
}

proptest! {
    #[test]
    fn single_edit_inverse_round_trips(
        source in "[a-zA-Z0-9 ]{0,128}",
        replacement in "[a-zA-Z0-9 ]{0,64}",
        a in 0usize..128,
        b in 0usize..128,
    ) {
        let len = source.len();
        let start = a.min(b).min(len);
        let end = a.max(b).min(len);

        let mut document = Document::new(&source).unwrap();
        let tx = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range(start, end), replacement)),
            TransactionKind::Programmatic,
        );

        let applied = document.apply(tx).unwrap();
        document.apply(applied.inverse).unwrap();

        prop_assert_eq!(document.text(), source);
    }

    #[test]
    fn unchanged_positions_after_replacement_shift_by_delta(
        source in "[a-z]{1,128}",
        replacement in "[A-Z]{0,32}",
        a in 0usize..128,
        b in 0usize..128,
    ) {
        let len = source.len();
        let start = a.min(b).min(len);
        let end = a.max(b).min(len);

        let mut document = Document::new(&source).unwrap();
        let tx = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(range(start, end), replacement.clone())),
            TransactionKind::Programmatic,
        );

        let applied = document.apply(tx).unwrap();

        if end < len {
            let old = TextSize::try_from_usize(end + 1).unwrap();
            let mapped = applied
                .change_map
                .map_old_to_new(old, Affinity::After)
                .unwrap()
                .offset;

            let removed = end - start;
            let expected = (end + 1) + replacement.len() - removed;
            prop_assert_eq!(mapped, TextSize::try_from_usize(expected).unwrap());
        }
    }
}
