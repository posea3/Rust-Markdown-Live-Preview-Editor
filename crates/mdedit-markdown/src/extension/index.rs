//! Build-local active interval tree. Payload ordinals retain original scan order.
use mdedit_core::{TextRange, TextSize};

struct Node {
    range: TextRange,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    active: Option<usize>,
    first: usize,
    min_start: TextSize,
    max_end: TextSize,
}

pub(super) struct OverlapIndex {
    nodes: Vec<Node>,
    slots: Vec<usize>,
    root: Option<usize>,
}

impl OverlapIndex {
    pub(super) fn new(ranges: impl IntoIterator<Item = TextRange>) -> Self {
        let mut ordered = ranges.into_iter().enumerate().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|(slot, range)| (range.start(), *slot));
        let mut slots = vec![0; ordered.len()];
        let mut nodes = ordered
            .into_iter()
            .enumerate()
            .map(|(position, (slot, range))| {
                slots[slot] = position;
                Node {
                    range,
                    parent: None,
                    left: None,
                    right: None,
                    active: None,
                    first: usize::MAX,
                    min_start: TextSize::new(u32::MAX),
                    max_end: TextSize::ZERO,
                }
            })
            .collect::<Vec<_>>();
        fn link(
            nodes: &mut [Node],
            start: usize,
            end: usize,
            parent: Option<usize>,
        ) -> Option<usize> {
            if start == end {
                return None;
            }
            let middle = start + (end - start) / 2;
            nodes[middle].parent = parent;
            nodes[middle].left = link(nodes, start, middle, Some(middle));
            nodes[middle].right = link(nodes, middle + 1, end, Some(middle));
            Some(middle)
        }
        let count = nodes.len();
        let root = link(&mut nodes, 0, count, None);
        Self { nodes, slots, root }
    }

    pub(super) fn fully_active(ranges: impl IntoIterator<Item = TextRange>) -> Self {
        let mut index = Self::new(ranges);
        for slot in 0..index.slots.len() {
            index.activate(slot, slot);
        }
        index
    }

    pub(super) fn activate(&mut self, slot: usize, ordinal: usize) {
        debug_assert_ne!(ordinal, usize::MAX);
        let position = self.slots[slot];
        self.nodes[position].active = Some(ordinal);
        let mut current = Some(position);
        while let Some(position) = current {
            let node = &self.nodes[position];
            let mut first = node.active.unwrap_or(usize::MAX);
            let mut min_start = node
                .active
                .map_or(TextSize::new(u32::MAX), |_| node.range.start());
            let mut max_end = node.active.map_or(TextSize::ZERO, |_| node.range.end());
            for child in [node.left, node.right].into_iter().flatten() {
                first = first.min(self.nodes[child].first);
                min_start = min_start.min(self.nodes[child].min_start);
                max_end = max_end.max(self.nodes[child].max_end);
            }
            current = node.parent;
            self.nodes[position].first = first;
            self.nodes[position].min_start = min_start;
            self.nodes[position].max_end = max_end;
        }
    }

    pub(super) fn first_matching(
        &self,
        range: TextRange,
        mut predicate: impl FnMut(usize) -> bool,
    ) -> Option<usize> {
        let mut best = usize::MAX;
        self.search(self.root, range, &mut predicate, &mut best, &mut || {});
        (best != usize::MAX).then_some(best)
    }

    fn search(
        &self,
        position: Option<usize>,
        range: TextRange,
        predicate: &mut impl FnMut(usize) -> bool,
        best: &mut usize,
        on_visit: &mut impl FnMut(),
    ) {
        let Some(position) = position else {
            return;
        };
        on_visit();
        let node = &self.nodes[position];
        if node.first >= *best || node.max_end <= range.start() || node.min_start >= range.end() {
            return;
        }
        if let Some(ordinal) = node.active
            && ordinal < *best
            && super::ranges_overlap(node.range, range)
            && predicate(ordinal)
        {
            *best = ordinal;
        }
        let first =
            |child: Option<usize>| child.map_or(usize::MAX, |index| self.nodes[index].first);
        let (early, late) = if first(node.left) <= first(node.right) {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };
        self.search(early, range, predicate, best, on_visit);
        self.search(late, range, predicate, best, on_visit);
    }

    #[cfg(test)]
    pub(super) fn first_matching_with_visits(
        &self,
        range: TextRange,
        mut predicate: impl FnMut(usize) -> bool,
    ) -> (Option<usize>, usize) {
        let mut best = usize::MAX;
        let mut visits = 0;
        self.search(self.root, range, &mut predicate, &mut best, &mut || {
            visits += 1
        });
        ((best != usize::MAX).then_some(best), visits)
    }
}
