use std::ops::Range;

use crate::error::TextError;
use crate::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<Position>,
    pub text: String,
}

impl Edit {
    #[must_use]
    pub fn insert(at: Position, text: impl Into<String>) -> Self {
        Self::replace(at..at, text)
    }

    #[must_use]
    pub fn delete(range: Range<Position>) -> Self {
        Self::replace(range, "")
    }

    #[must_use]
    pub fn replace(range: Range<Position>, text: impl Into<String>) -> Self {
        Self {
            range,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Run {
    Keep(usize),
    Replace { delete: usize, insert: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    runs: Vec<Run>,
    len_before: usize,
    len_after: usize,
}

impl ChangeSet {
    #[must_use]
    pub fn empty(len: usize) -> Self {
        let mut builder = Builder::default();
        builder.keep(len);
        builder.finish()
    }

    pub fn new(len: usize, edits: impl IntoIterator<Item = Edit>) -> Result<Self, TextError> {
        let mut builder = Builder::default();
        let mut previous_end = 0;
        for Edit { range, text } in edits {
            let (start, end) = (range.start.offset(), range.end.offset());
            if start > end || end > len {
                return Err(TextError::InvalidRange { start, end, len });
            }
            if start < previous_end {
                return Err(TextError::Unsorted {
                    start,
                    previous_end,
                });
            }
            builder.keep(start - previous_end);
            builder.replace(end - start, &text);
            previous_end = end;
        }
        builder.keep(len - previous_end);
        Ok(builder.finish())
    }

    #[must_use]
    pub fn len_before(&self) -> usize {
        self.len_before
    }

    #[must_use]
    pub fn len_after(&self) -> usize {
        self.len_after
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self
            .runs
            .iter()
            .any(|run| matches!(run, Run::Replace { .. }))
    }

    pub fn edits(&self) -> impl Iterator<Item = Edit> + '_ {
        self.replacements().map(|(range, text)| {
            Edit::replace(Position::new(range.start)..Position::new(range.end), text)
        })
    }

    pub(crate) fn replacements(&self) -> impl Iterator<Item = (Range<usize>, &str)> {
        let mut offset = 0;
        self.runs.iter().filter_map(move |run| match run {
            Run::Keep(len) => {
                offset += len;
                None
            }
            Run::Replace { delete, insert } => {
                let start = offset;
                offset += delete;
                Some((start..offset, insert.as_str()))
            }
        })
    }

    #[must_use]
    pub fn map_position(&self, position: Position, assoc: Assoc) -> Position {
        let position = position.offset();
        let (mut old, mut new) = (0, 0);
        for run in &self.runs {
            match run {
                Run::Keep(len) => {
                    if old + len > position {
                        return Position::new(new + position - old);
                    }
                    old += len;
                    new += len;
                }
                Run::Replace { delete, insert } => {
                    let end = old + delete;
                    let at_insertion = end == position && *delete == 0 && assoc == Assoc::Before;
                    if end > position || at_insertion {
                        let after = position != old && assoc == Assoc::After;
                        return Position::new(if after { new + insert.len() } else { new });
                    }
                    old = end;
                    new += insert.len();
                }
            }
        }
        Position::new(new)
    }
}

#[derive(Default)]
struct Builder {
    runs: Vec<Run>,
    len_before: usize,
    len_after: usize,
}

impl Builder {
    fn keep(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.len_before += len;
        self.len_after += len;
        if let Some(Run::Keep(last)) = self.runs.last_mut() {
            *last += len;
        } else {
            self.runs.push(Run::Keep(len));
        }
    }

    fn replace(&mut self, delete: usize, insert: &str) {
        if delete == 0 && insert.is_empty() {
            return;
        }
        self.len_before += delete;
        self.len_after += insert.len();
        if let Some(Run::Replace {
            delete: last_delete,
            insert: last_insert,
        }) = self.runs.last_mut()
        {
            *last_delete += delete;
            last_insert.push_str(insert);
        } else {
            self.runs.push(Run::Replace {
                delete,
                insert: insert.to_owned(),
            });
        }
    }

    fn finish(self) -> ChangeSet {
        ChangeSet {
            runs: self.runs,
            len_before: self.len_before,
            len_after: self.len_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::testing::{changes, cursor_positions};

    fn at(offset: usize) -> Position {
        Position::new(offset)
    }

    fn change_set(len: usize, edits: Vec<Edit>) -> ChangeSet {
        ChangeSet::new(len, edits).unwrap()
    }

    #[test]
    fn an_empty_change_set_keeps_the_length() {
        let changes = ChangeSet::empty(5);

        assert!(changes.is_empty());
        assert_eq!((changes.len_before(), changes.len_after()), (5, 5));
        assert_eq!(changes.edits().count(), 0);
    }

    #[test]
    fn a_change_set_gives_back_its_edits() {
        let edits = vec![
            Edit::insert(at(1), "x"),
            Edit::replace(at(3)..at(5), "yz"),
            Edit::delete(at(7)..at(8)),
        ];

        let changes = change_set(10, edits.clone());

        assert!(!changes.is_empty());
        assert_eq!(changes.edits().collect::<Vec<_>>(), edits);
        assert_eq!(changes.len_after(), 10);
    }

    #[test]
    fn edits_that_touch_become_one_edit() {
        let changes = change_set(
            4,
            vec![
                Edit::replace(at(0)..at(1), "a"),
                Edit::replace(at(1)..at(2), "b"),
            ],
        );

        assert_eq!(
            changes.edits().collect::<Vec<_>>(),
            [Edit::replace(at(0)..at(2), "ab")]
        );
    }

    #[test]
    fn edits_must_be_in_sequence() {
        let error =
            ChangeSet::new(10, [Edit::insert(at(5), "a"), Edit::delete(at(2)..at(6))]).unwrap_err();

        assert_eq!(
            error,
            TextError::Unsorted {
                start: 2,
                previous_end: 5
            }
        );
    }

    #[test]
    fn edits_must_be_in_the_text() {
        let error = ChangeSet::new(3, [Edit::delete(at(2)..at(4))]).unwrap_err();

        assert_eq!(
            error,
            TextError::InvalidRange {
                start: 2,
                end: 4,
                len: 3
            }
        );
    }

    #[test]
    fn a_position_before_an_edit_stays() {
        let changes = change_set(6, vec![Edit::replace(at(3)..at(4), "XY")]);

        assert_eq!(changes.map_position(at(2), Assoc::After), at(2));
    }

    #[test]
    fn a_position_after_an_edit_moves_with_the_text() {
        let changes = change_set(6, vec![Edit::replace(at(3)..at(4), "XY")]);

        assert_eq!(changes.map_position(at(5), Assoc::Before), at(6));
        assert_eq!(changes.map_position(at(6), Assoc::Before), at(7));
    }

    #[test]
    fn assoc_selects_the_side_of_an_insertion() {
        let changes = change_set(4, vec![Edit::insert(at(2), "abc")]);

        assert_eq!(changes.map_position(at(2), Assoc::Before), at(2));
        assert_eq!(changes.map_position(at(2), Assoc::After), at(5));
    }

    #[test]
    fn assoc_selects_the_side_of_a_replacement_for_a_position_in_it() {
        let changes = change_set(6, vec![Edit::replace(at(1)..at(4), "xy")]);

        assert_eq!(changes.map_position(at(2), Assoc::Before), at(1));
        assert_eq!(changes.map_position(at(2), Assoc::After), at(3));
        assert_eq!(changes.map_position(at(1), Assoc::After), at(1));
        assert_eq!(changes.map_position(at(4), Assoc::Before), at(3));
    }

    proptest! {
        #[test]
        fn mapped_positions_keep_their_order((text, edits) in changes()) {
            let changes = change_set(text.len(), edits);
            let positions = cursor_positions(&text);

            for pair in positions.windows(2) {
                for assoc in [Assoc::Before, Assoc::After] {
                    let first = changes.map_position(pair[0], assoc);
                    let second = changes.map_position(pair[1], assoc);
                    prop_assert!(first <= second);
                    prop_assert!(second.offset() <= changes.len_after());
                }
            }
        }

        #[test]
        fn a_position_outside_the_edits_keeps_its_character((text, edits) in changes()) {
            let changes = change_set(text.len(), edits.clone());
            let new_text = crate::testing::apply(&text, &edits);

            for position in cursor_positions(&text) {
                let touched = edits.iter().any(|edit| {
                    edit.range.start <= position && position <= edit.range.end
                });
                if touched {
                    continue;
                }
                let mapped = changes.map_position(position, Assoc::Before);
                prop_assert_eq!(
                    text[position.offset()..].chars().next(),
                    new_text[mapped.offset()..].chars().next()
                );
            }
        }
    }
}
