use smallvec::SmallVec;

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
    pub fn ordered_offsets(self) -> (TextSize, TextSize) {
        if self.anchor.offset <= self.head.offset {
            (self.anchor.offset, self.head.offset)
        } else {
            (self.head.offset, self.anchor.offset)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionSet {
    primary: usize,
    ranges: SmallVec<[SelectionRange; 1]>,
}

impl SelectionSet {
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
    pub fn ranges(&self) -> &[SelectionRange] {
        &self.ranges
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::caret(Anchor::default())
    }
}
