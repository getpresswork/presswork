use std::ops::Range;

use crate::buffer::Buffer;
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
        self.replacements().map(|(old, _, text)| {
            Edit::replace(Position::new(old.start)..Position::new(old.end), text)
        })
    }

    pub(crate) fn replacements(&self) -> impl Iterator<Item = (Range<usize>, Range<usize>, &str)> {
        let (mut old, mut new) = (0, 0);
        self.runs.iter().filter_map(move |run| match run {
            Run::Keep(len) => {
                old += len;
                new += len;
                None
            }
            Run::Replace { delete, insert } => {
                let ranges = (old..old + delete, new..new + insert.len());
                old += delete;
                new += insert.len();
                Some((ranges.0, ranges.1, insert.as_str()))
            }
        })
    }

    pub fn invert(&self, before: &Buffer) -> Result<Self, TextError> {
        before.check(self)?;
        let mut builder = Builder::default();
        let mut offset = 0;
        for run in &self.runs {
            match run {
                Run::Keep(len) => {
                    builder.keep(*len);
                    offset += len;
                }
                Run::Replace { delete, insert } => {
                    let deleted =
                        before.slice(Position::new(offset)..Position::new(offset + delete));
                    builder.replace(insert.len(), &deleted);
                    offset += delete;
                }
            }
        }
        Ok(builder.finish())
    }

    pub fn compose(&self, next: &Self) -> Result<Self, TextError> {
        let mismatch = TextError::LengthMismatch {
            expected: self.len_after,
            actual: next.len_before,
        };
        if self.len_after != next.len_before {
            return Err(mismatch);
        }
        let (mut firsts, mut seconds) = (self.pieces(), next.pieces());
        let (mut first, mut second) = (firsts.next(), seconds.next());
        let mut builder = Builder::default();
        let mut offset = 0;
        loop {
            match (first, second) {
                (None, None) => return Ok(builder.finish()),
                (Some(Piece::Delete(len)), _) => {
                    builder.replace(len, "");
                    first = firsts.next();
                }
                (_, Some(Piece::Insert(text))) => {
                    builder.replace(0, text);
                    second = seconds.next();
                }
                (Some(a), Some(b)) => {
                    let len = a.len().min(b.len());
                    if let Piece::Insert(text) = a
                        && !text.is_char_boundary(len)
                    {
                        return Err(TextError::NotCharBoundary(offset + len));
                    }
                    match (a, b) {
                        (Piece::Keep(_), Piece::Keep(_)) => builder.keep(len),
                        (Piece::Keep(_), Piece::Delete(_)) => builder.replace(len, ""),
                        (Piece::Insert(text), Piece::Keep(_)) => builder.replace(0, &text[..len]),
                        _ => {}
                    }
                    offset += len;
                    first = a.rest(len).or_else(|| firsts.next());
                    second = b.rest(len).or_else(|| seconds.next());
                }
                _ => return Err(mismatch),
            }
        }
    }

    pub fn map(&self, other: &Self, before: bool) -> Result<Self, TextError> {
        let mismatch = TextError::LengthMismatch {
            expected: self.len_before,
            actual: other.len_before,
        };
        if self.len_before != other.len_before {
            return Err(mismatch);
        }
        let (mut ours_pieces, mut theirs_pieces) = (self.pieces(), other.pieces());
        let (mut ours, mut theirs) = (ours_pieces.next(), theirs_pieces.next());
        let mut builder = Builder::default();
        loop {
            match (ours, theirs) {
                (None, None) => return Ok(builder.finish()),
                (Some(Piece::Insert(text)), _)
                    if before || !matches!(theirs, Some(Piece::Insert(_))) =>
                {
                    builder.replace(0, text);
                    ours = ours_pieces.next();
                }
                (_, Some(Piece::Insert(text))) => {
                    builder.keep(text.len());
                    theirs = theirs_pieces.next();
                }
                (Some(a), Some(b)) => {
                    let len = a.len().min(b.len());
                    match (a, b) {
                        (Piece::Keep(_), Piece::Keep(_)) => builder.keep(len),
                        (Piece::Delete(_), Piece::Keep(_)) => builder.replace(len, ""),
                        _ => {}
                    }
                    ours = a.rest(len).or_else(|| ours_pieces.next());
                    theirs = b.rest(len).or_else(|| theirs_pieces.next());
                }
                _ => return Err(mismatch),
            }
        }
    }

    fn pieces(&self) -> impl Iterator<Item = Piece<'_>> {
        self.runs
            .iter()
            .flat_map(|run| match run {
                Run::Keep(len) => [Some(Piece::Keep(*len)), None],
                Run::Replace { delete, insert } => [
                    (*delete > 0).then_some(Piece::Delete(*delete)),
                    (!insert.is_empty()).then_some(Piece::Insert(insert)),
                ],
            })
            .flatten()
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

#[derive(Debug, Clone, Copy)]
enum Piece<'a> {
    Keep(usize),
    Delete(usize),
    Insert(&'a str),
}

impl Piece<'_> {
    fn len(self) -> usize {
        match self {
            Self::Keep(len) | Self::Delete(len) => len,
            Self::Insert(text) => text.len(),
        }
    }

    fn rest(self, used: usize) -> Option<Self> {
        (used < self.len()).then(|| match self {
            Self::Keep(len) => Self::Keep(len - used),
            Self::Delete(len) => Self::Delete(len - used),
            Self::Insert(text) => Self::Insert(&text[used..]),
        })
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
    use crate::testing::{changes, cursor_positions, edits_for, raw_edits, text};

    fn at(offset: usize) -> Position {
        Position::new(offset)
    }

    fn change_set(len: usize, edits: Vec<Edit>) -> ChangeSet {
        ChangeSet::new(len, edits).unwrap()
    }

    fn applied(text: &str, changes: &ChangeSet) -> String {
        let mut buffer = Buffer::from(text);
        buffer.apply(changes).unwrap();
        buffer.to_string()
    }

    #[test]
    fn invert_gives_the_changes_that_undo_the_changes() {
        let before = Buffer::from("abcd");
        let changes = change_set(4, vec![Edit::replace(at(1)..at(3), "XYZ")]);

        let inverse = changes.invert(&before).unwrap();

        assert_eq!(
            inverse.edits().collect::<Vec<_>>(),
            [Edit::replace(at(1)..at(4), "bc")]
        );
        assert_eq!(applied("aXYZd", &inverse), "abcd");
    }

    #[test]
    fn invert_needs_the_text_before_the_changes() {
        let changes = ChangeSet::empty(4);

        let error = changes.invert(&Buffer::from("abc")).unwrap_err();

        assert_eq!(
            error,
            TextError::LengthMismatch {
                expected: 4,
                actual: 3
            }
        );
    }

    #[test]
    fn compose_gives_one_change_set_for_two() {
        let first = change_set(3, vec![Edit::insert(at(0), "x")]);
        let second = change_set(4, vec![Edit::delete(at(1)..at(2))]);

        let both = first.compose(&second).unwrap();

        assert_eq!(
            both.edits().collect::<Vec<_>>(),
            [Edit::replace(at(0)..at(1), "x")]
        );
    }

    #[test]
    fn compose_rejects_changes_that_split_a_character() {
        let first = change_set(0, vec![Edit::insert(at(0), "é")]);
        let second = change_set(2, vec![Edit::delete(at(0)..at(1))]);

        let error = first.compose(&second).unwrap_err();

        assert_eq!(error, TextError::NotCharBoundary(1));
    }

    #[test]
    fn compose_needs_changes_in_sequence() {
        let first = change_set(3, vec![Edit::insert(at(0), "x")]);

        let error = first.compose(&ChangeSet::empty(3)).unwrap_err();

        assert_eq!(
            error,
            TextError::LengthMismatch {
                expected: 4,
                actual: 3
            }
        );
    }

    #[test]
    fn map_moves_changes_over_other_changes() {
        let ours = change_set(3, vec![Edit::insert(at(1), "X")]);
        let theirs = change_set(3, vec![Edit::insert(at(2), "Y")]);

        let mapped = ours.map(&theirs, false).unwrap();

        assert_eq!(applied("abYc", &mapped), "aXbYc");
    }

    #[test]
    fn before_selects_the_order_of_two_insertions_at_one_position() {
        let ours = change_set(3, vec![Edit::insert(at(1), "X")]);
        let theirs = change_set(3, vec![Edit::insert(at(1), "Y")]);

        assert_eq!(applied("aYbc", &ours.map(&theirs, false).unwrap()), "aYXbc");
        assert_eq!(applied("aYbc", &ours.map(&theirs, true).unwrap()), "aXYbc");
    }

    #[test]
    fn map_does_not_delete_text_twice() {
        let ours = change_set(4, vec![Edit::delete(at(1)..at(3))]);
        let theirs = change_set(4, vec![Edit::delete(at(2)..at(4))]);

        let mapped = ours.map(&theirs, false).unwrap();

        assert_eq!(applied("ab", &mapped), "a");
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
        fn the_inverse_restores_the_text((text, edits) in changes()) {
            let changes = change_set(text.len(), edits);

            let inverse = changes.invert(&Buffer::from(text.as_str())).unwrap();

            prop_assert_eq!(applied(&applied(&text, &changes), &inverse), text);
        }

        #[test]
        fn composed_changes_agree_with_changes_in_sequence(
            (text, edits) in changes(),
            raw in raw_edits(),
        ) {
            let first = change_set(text.len(), edits);
            let middle = applied(&text, &first);
            let second = change_set(middle.len(), edits_for(&middle, raw));

            let both = first.compose(&second).unwrap();

            prop_assert_eq!(applied(&text, &both), applied(&middle, &second));
        }

        #[test]
        fn mapped_changes_give_the_same_text_in_each_order(
            text in text(),
            ours in raw_edits(),
            theirs in raw_edits(),
        ) {
            let ours = change_set(text.len(), edits_for(&text, ours));
            let theirs = change_set(text.len(), edits_for(&text, theirs));

            let ours_last = applied(&applied(&text, &theirs), &ours.map(&theirs, false).unwrap());
            let theirs_last = applied(&applied(&text, &ours), &theirs.map(&ours, true).unwrap());

            prop_assert_eq!(ours_last, theirs_last);
        }

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
