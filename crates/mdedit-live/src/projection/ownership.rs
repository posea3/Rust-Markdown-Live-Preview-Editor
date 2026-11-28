//! Build-local containment lookups. No document/configuration state is cached.
use mdedit_core::{TextRange, TextSize};
use mdedit_markdown::{SyntaxKind, SyntaxNode};

/// Any containing range can be found with a start upper-bound and prefix max end.
pub(super) struct ContainmentSet {
    starts_and_max_ends: Vec<(TextSize, TextSize)>,
}

impl ContainmentSet {
    pub(super) fn new(ranges: impl IntoIterator<Item = TextRange>) -> Self {
        let mut entries = ranges
            .into_iter()
            .map(|range| (range.start(), range.end()))
            .collect::<Vec<_>>();
        entries.sort_unstable();
        let mut max_end = TextSize::ZERO;
        for (_, end) in &mut entries {
            max_end = max_end.max(*end);
            *end = max_end;
        }
        Self {
            starts_and_max_ends: entries,
        }
    }

    pub(super) fn contains(&self, range: TextRange) -> bool {
        let upper = self
            .starts_and_max_ends
            .partition_point(|(start, _)| *start <= range.start());
        upper > 0 && self.starts_and_max_ends[upper - 1].1 >= range.end()
    }
}

struct Entry<'a> {
    node: &'a SyntaxNode,
    bounds: TextRange,
    preorder: usize,
    subtree_end: TextSize,
}

/// A balanced interval tree over effective ancestor-intersection bounds.
/// Original node ranges determine ownership; preorder breaks equal-size ties.
pub(super) struct OwnerLookup<'a> {
    entries: Vec<Entry<'a>>,
}

impl<'a> OwnerLookup<'a> {
    pub(super) fn new(root: &'a SyntaxNode) -> Self {
        fn collect<'a>(
            node: &'a SyntaxNode,
            parent_bounds: TextRange,
            out: &mut Vec<(&'a SyntaxNode, TextRange)>,
        ) {
            let Ok(bounds) = TextRange::new(
                parent_bounds.start().max(node.range().start()),
                parent_bounds.end().min(node.range().end()),
            ) else {
                return;
            };
            out.push((node, bounds));
            for child in node.children() {
                collect(child, bounds, out);
            }
        }
        let mut entries = Vec::new();
        collect(root, root.range(), &mut entries);
        Self::from_entries(entries)
    }

    pub(super) fn from_entries(entries: Vec<(&'a SyntaxNode, TextRange)>) -> Self {
        let mut entries = entries
            .into_iter()
            .enumerate()
            .map(|(preorder, (node, bounds))| Entry {
                node,
                bounds,
                preorder,
                subtree_end: bounds.end(),
            })
            .collect::<Vec<_>>();
        entries.sort_unstable_by_key(|entry| (entry.bounds.start(), entry.preorder));
        fn augment(entries: &mut [Entry<'_>]) -> TextSize {
            if entries.is_empty() {
                return TextSize::ZERO;
            }
            let middle = entries.len() / 2;
            let (left, tail) = entries.split_at_mut(middle);
            let (root, right) = tail.split_first_mut().expect("nonempty tree");
            root.subtree_end = root.bounds.end().max(augment(left)).max(augment(right));
            root.subtree_end
        }
        augment(&mut entries);
        Self { entries }
    }

    pub(super) fn smallest_containing(
        &self,
        kind: SyntaxKind,
        range: TextRange,
    ) -> Option<&'a SyntaxNode> {
        self.query(kind, range, &mut || {})
    }

    fn query(
        &self,
        kind: SyntaxKind,
        range: TextRange,
        on_visit: &mut impl FnMut(),
    ) -> Option<&'a SyntaxNode> {
        let mut best = None;
        self.search(0, self.entries.len(), kind, range, &mut best, on_visit);
        best.map(|index| self.entries[index].node)
    }

    fn search(
        &self,
        start: usize,
        end: usize,
        kind: SyntaxKind,
        range: TextRange,
        best: &mut Option<usize>,
        on_visit: &mut impl FnMut(),
    ) {
        if start == end {
            return;
        }
        on_visit();
        let middle = start + (end - start) / 2;
        let entry = &self.entries[middle];
        if self.entries[start].bounds.start() > range.start() || entry.subtree_end < range.end() {
            return;
        }
        self.search(start, middle, kind, range, best, on_visit);
        if entry.node.kind() == kind
            && entry.bounds.start() <= range.start()
            && entry.bounds.end() >= range.end()
            && best.is_none_or(|previous| {
                (entry.node.range().len(), entry.preorder)
                    < (
                        self.entries[previous].node.range().len(),
                        self.entries[previous].preorder,
                    )
            })
        {
            *best = Some(middle);
        }
        // All entries in the right subtree start at or after this node's start.
        if entry.bounds.start() <= range.start() {
            self.search(middle + 1, end, kind, range, best, on_visit);
        }
    }

    #[cfg(test)]
    pub(super) fn query_with_visits(
        &self,
        kind: SyntaxKind,
        range: TextRange,
    ) -> (Option<&'a SyntaxNode>, usize) {
        let mut visits = 0;
        let result = self.query(kind, range, &mut || visits += 1);
        (result, visits)
    }
}
