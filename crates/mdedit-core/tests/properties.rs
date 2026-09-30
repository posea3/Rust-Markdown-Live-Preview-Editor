use mdedit_core::{
    Affinity, Change, ChangeMap, ChangeSet, Document, TextRange, TextSize, Transaction,
    TransactionKind,
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
    fn two_non_overlapping_edits_inverse_round_trip(
        source in "[a-zA-Z0-9 ]{8,128}",
        left in "[A-Z]{0,8}",
        right in "[0-9]{0,8}",
        positions in prop::collection::vec(0usize..128, 4),
    ) {
        let len = source.len();
        let mut points: Vec<usize> = positions.into_iter().map(|p| p.min(len)).collect();
        points.sort_unstable();

        let changes = ChangeSet::new(vec![
            Change::new(range(points[0], points[1]), left),
            Change::new(range(points[2], points[3]), right),
        ]).unwrap();

        let mut document = Document::new(&source).unwrap();
        let applied = document.apply(Transaction::new(
            document.revision(),
            changes,
            TransactionKind::Programmatic,
        )).unwrap();

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

    #[test]
    fn change_map_is_monotonic_for_both_affinities(
        source in "[a-z]{1,128}",
        replacement in "[A-Z]{0,32}",
        a in 0usize..128,
        b in 0usize..128,
    ) {
        let len = source.len();
        let start = a.min(b).min(len);
        let end = a.max(b).min(len);
        let changes = ChangeSet::single(Change::new(range(start, end), replacement));
        let map = ChangeMap::from_change_set(&changes).unwrap();

        for affinity in [Affinity::Before, Affinity::After] {
            let mut previous = TextSize::ZERO;
            for offset in 0..=len {
                let mapped = map
                    .map_old_to_new(TextSize::try_from_usize(offset).unwrap(), affinity)
                    .unwrap()
                    .offset;
                prop_assert!(mapped >= previous);
                previous = mapped;
            }
        }
    }
}
