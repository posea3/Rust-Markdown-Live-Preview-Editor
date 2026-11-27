use mdedit_core::{TextRange, TextSize};

use super::{ProjectedSize, ProjectionBias, ProjectionMap};

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(TextSize::new(start), TextSize::new(end)).unwrap()
}

#[test]
fn every_small_concealment_mask_matches_prefix_counts_and_inverse_extrema() {
    for len in 0..=10_u32 {
        for mask in 0..1_u32 << len {
            // Independent oracle: count retained bytes at every source boundary.
            let mut prefix = vec![0_u32];
            let mut claims = vec![range(0, 0)];
            for byte in 0..len {
                let hidden = mask & (1 << byte) != 0;
                prefix.push(prefix.last().unwrap() + u32::from(!hidden));
                if hidden {
                    claims.push(range(byte, byte + 1));
                    if byte % 2 == 0 {
                        claims.push(range(byte, byte + 1));
                    }
                }
            }
            claims.reverse();
            let map = ProjectionMap::from_concealed(TextSize::new(len), claims).unwrap();
            for (source, expected) in prefix.iter().enumerate() {
                assert_eq!(
                    map.source_to_projected(TextSize::new(source as u32)),
                    Some(ProjectedSize::new(*expected)),
                    "len={len}, mask={mask}, source={source}"
                );
            }
            assert_eq!(map.source_to_projected(TextSize::new(len + 1)), None);
            let projected_len = *prefix.last().unwrap();
            let expected = (0..=projected_len)
                .map(|projected| {
                    let before =
                        prefix.iter().position(|value| *value == projected).unwrap() as u32;
                    let after = prefix
                        .iter()
                        .rposition(|value| *value == projected)
                        .unwrap() as u32;
                    (
                        ProjectedSize::new(projected),
                        TextSize::new(before),
                        TextSize::new(after),
                    )
                })
                .collect::<Vec<_>>();
            for &(projected, before, after) in &expected {
                assert_eq!(
                    map.projected_to_source(projected, ProjectionBias::Before),
                    Some(before),
                    "len={len}, mask={mask}, projected={projected:?}"
                );
                assert_eq!(
                    map.projected_to_source(projected, ProjectionBias::After),
                    Some(after),
                    "len={len}, mask={mask}, projected={projected:?}"
                );
            }
            for bias in [ProjectionBias::Before, ProjectionBias::After] {
                assert_eq!(
                    map.projected_to_source(ProjectedSize::new(projected_len + 1), bias),
                    None
                );
            }
            let repeated = expected.iter().flat_map(|value| [value.0, value.0]);
            let repeated_expected = expected
                .iter()
                .flat_map(|value| [*value, *value])
                .collect::<Vec<_>>();
            assert_eq!(
                map.projected_source_boundaries(repeated)
                    .collect::<Vec<_>>(),
                repeated_expected,
                "bulk len={len}, mask={mask}"
            );
            let sparse = [ProjectedSize::ZERO, ProjectedSize::new(projected_len)];
            assert_eq!(
                map.projected_source_boundaries(sparse.into_iter())
                    .collect::<Vec<_>>(),
                vec![expected[0], *expected.last().unwrap()],
                "sparse len={len}, mask={mask}"
            );
        }
    }
}

#[test]
fn mapping_near_u32_limit_keeps_collapsed_edges_without_overflow() {
    let max = u32::MAX;
    let map =
        ProjectionMap::from_concealed(TextSize::new(max), vec![range(0, 2), range(max - 2, max)])
            .unwrap();
    for (source, projected) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 1),
        (max - 2, max - 4),
        (max - 1, max - 4),
        (max, max - 4),
    ] {
        assert_eq!(
            map.source_to_projected(TextSize::new(source)),
            Some(ProjectedSize::new(projected))
        );
    }
    let expected = [
        (0, 0, 2),
        (1, 3, 3),
        (max - 5, max - 3, max - 3),
        (max - 4, max - 2, max),
    ];
    for (projected, before, after) in expected {
        assert_eq!(
            map.projected_to_source(ProjectedSize::new(projected), ProjectionBias::Before),
            Some(TextSize::new(before))
        );
        assert_eq!(
            map.projected_to_source(ProjectedSize::new(projected), ProjectionBias::After),
            Some(TextSize::new(after))
        );
    }
    assert_eq!(
        map.projected_source_boundaries(
            expected
                .into_iter()
                .map(|value| ProjectedSize::new(value.0))
        )
        .collect::<Vec<_>>(),
        expected
            .into_iter()
            .map(|(p, b, a)| (ProjectedSize::new(p), TextSize::new(b), TextSize::new(a)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        map.projected_to_source(ProjectedSize::new(max), ProjectionBias::After),
        None
    );
}
