use smallvec::SmallVec;
use thiserror::Error;

use crate::TextSize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Affinity {
    Before,
    #[default]
    After,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub offset: TextSize,
    pub affinity: Affinity,
}

impl Anchor {
    #[must_use]
    pub const fn new(offset: TextSize, affinity: Affinity) -> Self {
        Self { offset, affinity }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelectionRange {
    pub anchor: Anchor,
    pub head: Anchor,
}

impl SelectionRange {
    #[must_use]
    pub const fn caret(at: Anchor) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    #[must_use]
    pub const fn is_caret(self) -> bool {
        self.anchor.offset.get() == self.head.offset.get()
    }

    #[must_use]
    pub const fn is_reversed(self) -> bool {
        self.anchor.offset.get() > self.head.offset.get()
    }

    #[must_use]
    pub fn ordered_offsets(self) -> (TextSize, TextSize) {
        if self.anchor.offset <= self.head.offset {
            (self.anchor.offset, self.head.offset)
        } else {
            (self.head.offset, self.anchor.offset)
        }
    }

    fn from_ordered(start: TextSize, end: TextSize, reversed: bool) -> Self {
        if start == end {
            return Self::caret(Anchor::new(start, Affinity::After));
        }

        let start_anchor = Anchor::new(start, Affinity::Before);
        let end_anchor = Anchor::new(end, Affinity::After);
        if reversed {
            Self {
                anchor: end_anchor,
                head: start_anchor,
            }
        } else {
            Self {
                anchor: start_anchor,
                head: end_anchor,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionSet {
    primary: usize,
    ranges: SmallVec<[SelectionRange; 1]>,
}

impl SelectionSet {
    pub fn new(ranges: Vec<SelectionRange>, primary: usize) -> Result<Self, SelectionError> {
        if ranges.is_empty() {
            return Err(SelectionError::Empty);
        }
        if primary >= ranges.len() {
            return Err(SelectionError::PrimaryOutOfBounds {
                primary,
                len: ranges.len(),
            });
        }

        #[derive(Clone, Copy)]
        struct Entry {
            range: SelectionRange,
            primary: bool,
        }

        let mut entries: Vec<Entry> = ranges
            .into_iter()
            .enumerate()
            .map(|(index, range)| Entry {
                range,
                primary: index == primary,
            })
            .collect();

        entries.sort_by_key(|entry| {
            let (start, end) = entry.range.ordered_offsets();
            (start, end)
        });

        let mut normalized: Vec<Entry> = Vec::with_capacity(entries.len());

        for entry in entries {
            let Some(last) = normalized.last_mut() else {
                normalized.push(entry);
                continue;
            };

            let (last_start, last_end) = last.range.ordered_offsets();
            let (start, end) = entry.range.ordered_offsets();

            if start <= last_end {
                let merged_start = last_start.min(start);
                let merged_end = last_end.max(end);

                let reversed = if last.primary {
                    last.range.is_reversed()
                } else if entry.primary {
                    entry.range.is_reversed()
                } else {
                    false
                };

                last.range = if merged_start == merged_end {
                    if last.primary {
                        last.range
                    } else if entry.primary {
                        entry.range
                    } else {
                        SelectionRange::caret(Anchor::new(merged_start, Affinity::After))
                    }
                } else {
                    SelectionRange::from_ordered(merged_start, merged_end, reversed)
                };
                last.primary |= entry.primary;
            } else {
                normalized.push(entry);
            }
        }

        let primary = normalized
            .iter()
            .position(|entry| entry.primary)
            .expect("one normalized selection must contain the primary range");

        Ok(Self {
            primary,
            ranges: normalized.into_iter().map(|entry| entry.range).collect(),
        })
    }

    #[must_use]
    pub fn caret(at: Anchor) -> Self {
        Self {
            primary: 0,
            ranges: SmallVec::from_buf([SelectionRange::caret(at)]),
        }
    }

    #[must_use]
    pub fn primary(&self) -> SelectionRange {
        self.ranges[self.primary]
    }

    #[must_use]
    pub const fn primary_index(&self) -> usize {
        self.primary
    }

    #[must_use]
    pub fn ranges(&self) -> &[SelectionRange] {
        &self.ranges
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::caret(Anchor::default())
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SelectionError {
    #[error("a selection set must contain at least one range")]
    Empty,

    #[error("primary selection index {primary} is outside selection count {len}")]
    PrimaryOutOfBounds { primary: usize, len: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(offset: u32) -> Anchor {
        Anchor::new(TextSize::new(offset), Affinity::After)
    }

    #[test]
    fn normalizes_overlapping_ranges_and_preserves_primary_direction() {
        let first = SelectionRange {
            anchor: anchor(2),
            head: anchor(6),
        };
        let primary = SelectionRange {
            anchor: anchor(8),
            head: anchor(4),
        };

        let set = SelectionSet::new(vec![first, primary], 1).unwrap();

        assert_eq!(set.len(), 1);
        assert_eq!(
            set.primary().ordered_offsets(),
            (TextSize::new(2), TextSize::new(8))
        );
        assert!(set.primary().is_reversed());
    }

    #[test]
    fn sorts_disjoint_ranges_and_tracks_primary() {
        let later = SelectionRange::caret(anchor(8));
        let earlier = SelectionRange::caret(anchor(2));

        let set = SelectionSet::new(vec![later, earlier], 0).unwrap();

        assert_eq!(set.ranges()[0].head.offset, TextSize::new(2));
        assert_eq!(set.ranges()[1].head.offset, TextSize::new(8));
        assert_eq!(set.primary_index(), 1);
    }
}
