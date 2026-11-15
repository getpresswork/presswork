use crate::change::{Assoc, ChangeSet};
use crate::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelectionRange {
    pub anchor: Position,
    pub head: Position,
}

impl SelectionRange {
    #[must_use]
    pub const fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    #[must_use]
    pub const fn cursor(at: Position) -> Self {
        Self::new(at, at)
    }

    #[must_use]
    pub fn start(self) -> Position {
        self.anchor.min(self.head)
    }

    #[must_use]
    pub fn end(self) -> Position {
        self.anchor.max(self.head)
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }

    #[must_use]
    pub fn map(self, changes: &ChangeSet) -> Self {
        if self.is_empty() {
            return Self::cursor(changes.map_position(self.head, Assoc::Before));
        }
        let first = changes.map_position(self.start(), Assoc::After);
        let second = changes.map_position(self.end(), Assoc::Before);
        self.directed(first.min(second), first.max(second))
    }

    fn directed(self, start: Position, end: Position) -> Self {
        if self.anchor > self.head {
            Self::new(end, start)
        } else {
            Self::new(start, end)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: Vec<SelectionRange>,
    primary: usize,
}

impl Selection {
    #[must_use]
    pub fn new(ranges: Vec<SelectionRange>, primary: usize) -> Self {
        if ranges.is_empty() {
            return Self::cursor(Position::default());
        }
        let primary = primary.min(ranges.len() - 1);
        let mut indexed: Vec<_> = ranges.into_iter().enumerate().collect();
        indexed.sort_by_key(|(_, range)| range.start());
        let mut primary = indexed
            .iter()
            .position(|(index, _)| *index == primary)
            .unwrap_or_default();
        let mut merged: Vec<SelectionRange> = Vec::with_capacity(indexed.len());
        for (_, range) in indexed {
            if let Some(previous) = merged.last_mut() {
                let touch_joins = range.is_empty() || previous.is_empty();
                let joins = if touch_joins {
                    range.start() <= previous.end()
                } else {
                    range.start() < previous.end()
                };
                if joins {
                    let end = previous.end().max(range.end());
                    let direction = if range.is_empty() { *previous } else { range };
                    *previous = direction.directed(previous.start(), end);
                    if merged.len() <= primary {
                        primary -= 1;
                    }
                    continue;
                }
            }
            merged.push(range);
        }
        Self {
            ranges: merged,
            primary,
        }
    }

    #[must_use]
    pub fn single(range: SelectionRange) -> Self {
        Self::new(vec![range], 0)
    }

    #[must_use]
    pub fn cursor(at: Position) -> Self {
        Self::single(SelectionRange::cursor(at))
    }

    #[must_use]
    pub fn ranges(&self) -> &[SelectionRange] {
        &self.ranges
    }

    #[must_use]
    pub fn primary(&self) -> SelectionRange {
        self.ranges[self.primary]
    }

    #[must_use]
    pub fn primary_index(&self) -> usize {
        self.primary
    }

    #[must_use]
    pub fn map(&self, changes: &ChangeSet) -> Self {
        let ranges = self.ranges.iter().map(|range| range.map(changes)).collect();
        Self::new(ranges, self.primary)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::change::Edit;
    use crate::testing::{changes, cursor_positions};

    fn at(offset: usize) -> Position {
        Position::new(offset)
    }

    fn range(anchor: usize, head: usize) -> SelectionRange {
        SelectionRange::new(at(anchor), at(head))
    }

    fn change_set(len: usize, edits: Vec<Edit>) -> ChangeSet {
        ChangeSet::new(len, edits).unwrap()
    }

    #[test]
    fn a_selection_sorts_its_ranges_and_keeps_the_primary_range() {
        let selection = Selection::new(vec![range(8, 9), range(1, 2), range(4, 4)], 0);

        assert_eq!(selection.ranges(), [range(1, 2), range(4, 4), range(8, 9)]);
        assert_eq!(selection.primary(), range(8, 9));
        assert_eq!(selection.primary_index(), 2);
    }

    #[test]
    fn overlapping_ranges_become_one_range() {
        let selection = Selection::new(vec![range(1, 5), range(7, 3)], 1);

        assert_eq!(selection.ranges(), [range(7, 1)]);
        assert_eq!(selection.primary(), range(7, 1));
    }

    #[test]
    fn a_cursor_in_a_range_joins_the_range() {
        let selection = Selection::new(vec![range(2, 6), range(6, 6)], 0);

        assert_eq!(selection.ranges(), [range(2, 6)]);
    }

    #[test]
    fn a_cursor_at_the_start_of_a_range_joins_the_range() {
        let selection = Selection::new(vec![range(2, 2), range(6, 2)], 0);

        assert_eq!(selection.ranges(), [range(6, 2)]);
        assert_eq!(selection.primary(), range(6, 2));
    }

    #[test]
    fn a_cursor_that_joins_a_range_keeps_the_direction_of_the_range() {
        let selection = Selection::new(vec![range(6, 2), range(6, 6)], 1);

        assert_eq!(selection.ranges(), [range(6, 2)]);
    }

    #[test]
    fn ranges_that_only_touch_stay_apart() {
        let selection = Selection::new(vec![range(2, 4), range(4, 6)], 0);

        assert_eq!(selection.ranges(), [range(2, 4), range(4, 6)]);
    }

    #[test]
    fn a_cursor_moves_with_the_text_in_front_of_it() {
        let changes = change_set(10, vec![Edit::insert(at(0), "abc")]);

        assert_eq!(
            Selection::cursor(at(4)).map(&changes),
            Selection::cursor(at(7))
        );
    }

    #[test]
    fn a_cursor_stays_in_front_of_text_inserted_at_it() {
        let changes = change_set(10, vec![Edit::insert(at(4), "abc")]);

        assert_eq!(
            Selection::cursor(at(4)).map(&changes),
            Selection::cursor(at(4))
        );
    }

    #[test]
    fn a_range_does_not_grow_at_its_edges() {
        let changes = change_set(
            10,
            vec![Edit::insert(at(2), "ab"), Edit::insert(at(5), "cd")],
        );

        assert_eq!(range(2, 5).map(&changes), range(4, 7));
        assert_eq!(range(5, 2).map(&changes), range(7, 4));
    }

    #[test]
    fn a_range_in_replaced_text_selects_the_new_text() {
        let changes = change_set(10, vec![Edit::replace(at(1)..at(8), "xyz")]);

        assert_eq!(range(3, 5).map(&changes), range(1, 4));
    }

    #[test]
    fn cursors_that_meet_become_one_cursor() {
        let changes = change_set(10, vec![Edit::delete(at(2)..at(6))]);
        let selection = Selection::new(vec![range(2, 2), range(6, 6)], 1);

        assert_eq!(selection.map(&changes), Selection::cursor(at(2)));
    }

    proptest! {
        #[test]
        fn a_mapped_selection_stays_sorted_and_in_the_text(
            (text, edits) in changes(),
            picks in prop::collection::vec(
                (any::<prop::sample::Index>(), any::<prop::sample::Index>()),
                1..5,
            ),
            primary: prop::sample::Index,
        ) {
            let positions = cursor_positions(&text);
            let ranges: Vec<_> = picks
                .iter()
                .map(|(anchor, head)| SelectionRange::new(
                    positions[anchor.index(positions.len())],
                    positions[head.index(positions.len())],
                ))
                .collect();
            let selection = Selection::new(ranges.clone(), primary.index(ranges.len()));
            let changes = change_set(text.len(), edits);

            let mapped = selection.map(&changes);

            prop_assert!(mapped.ranges().iter().any(|range| *range == mapped.primary()));
            for pair in mapped.ranges().windows(2) {
                prop_assert!(pair[0].end() <= pair[1].start());
            }
            for range in mapped.ranges() {
                prop_assert!(range.end().offset() <= changes.len_after());
            }
        }
    }
}
