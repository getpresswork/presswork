use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use ropey::Rope;

use crate::change::ChangeSet;
use crate::error::TextError;
use crate::position::{LineColumn, Position, Unit};

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    #[must_use]
    pub fn len(&self) -> usize {
        self.rope.len_bytes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn apply(&mut self, changes: &ChangeSet) -> Result<(), TextError> {
        self.check(changes)?;
        let replacements: Vec<_> = changes.replacements().collect();
        for (range, text) in replacements.into_iter().rev() {
            let start = self.rope.byte_to_char(range.start);
            let end = self.rope.byte_to_char(range.end);
            self.rope.remove(start..end);
            self.rope.insert(start, text);
        }
        Ok(())
    }

    pub(crate) fn check(&self, changes: &ChangeSet) -> Result<(), TextError> {
        if changes.len_before() != self.len() {
            return Err(TextError::LengthMismatch {
                expected: changes.len_before(),
                actual: self.len(),
            });
        }
        for (range, _) in changes.replacements() {
            for offset in [range.start, range.end] {
                if !self.is_char_boundary(offset) {
                    return Err(TextError::NotCharBoundary(offset));
                }
            }
        }
        Ok(())
    }

    fn is_char_boundary(&self, offset: usize) -> bool {
        offset <= self.len() && self.rope.char_to_byte(self.rope.byte_to_char(offset)) == offset
    }

    #[must_use]
    pub fn line(&self, index: usize) -> Option<Cow<'_, str>> {
        (index < self.line_count()).then(|| {
            text_of(
                self.rope
                    .byte_slice(self.rope.line_to_byte(index)..self.line_end(index)),
            )
        })
    }

    #[must_use]
    pub fn slice(&self, range: Range<Position>) -> Cow<'_, str> {
        text_of(
            self.rope
                .byte_slice(range.start.offset()..range.end.offset()),
        )
    }

    #[must_use]
    pub fn line_column(&self, position: Position, unit: Unit) -> LineColumn {
        let offset = self.floor(position.offset());
        let line = self.rope.byte_to_line(offset);
        let start = self.rope.line_to_byte(line);
        let column = match unit {
            Unit::Byte => offset - start,
            Unit::Char => self.rope.byte_to_char(offset) - self.rope.byte_to_char(start),
            Unit::Utf16 => self.utf16(offset) - self.utf16(start),
        };
        LineColumn { line, column }
    }

    #[must_use]
    pub fn position(&self, line_column: LineColumn, unit: Unit) -> Position {
        let LineColumn { line, column } = line_column;
        if line >= self.line_count() {
            return Position::new(self.len());
        }
        let start = self.rope.line_to_byte(line);
        let end = self.line_end(line);
        let offset = match unit {
            Unit::Byte => self.floor((start + column).min(end)),
            Unit::Char => {
                let first = self.rope.byte_to_char(start);
                let last = self.rope.byte_to_char(end);
                self.rope.char_to_byte((first + column).min(last))
            }
            Unit::Utf16 => {
                let first = self.utf16(start);
                let last = self.utf16(end);
                let char = self.rope.utf16_cu_to_char((first + column).min(last));
                self.rope.char_to_byte(char)
            }
        };
        Position::new(offset)
    }

    fn floor(&self, offset: usize) -> usize {
        let offset = offset.min(self.len());
        self.rope.char_to_byte(self.rope.byte_to_char(offset))
    }

    fn utf16(&self, offset: usize) -> usize {
        self.rope.char_to_utf16_cu(self.rope.byte_to_char(offset))
    }

    fn line_end(&self, line: usize) -> usize {
        let slice = self.rope.line(line);
        let mut end = self.rope.line_to_byte(line) + slice.len_bytes();
        let mut chars = slice.chars_at(slice.len_chars()).reversed();
        if let Some(last) = chars.next().filter(|&char| is_line_break(char)) {
            end -= last.len_utf8();
            if last == '\n' && chars.next() == Some('\r') {
                end -= 1;
            }
        }
        end
    }
}

fn is_line_break(char: char) -> bool {
    matches!(
        char,
        '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

fn text_of(slice: ropey::RopeSlice<'_>) -> Cow<'_, str> {
    slice
        .as_str()
        .map_or_else(|| Cow::Owned(slice.to_string()), Cow::Borrowed)
}

impl From<&str> for Buffer {
    fn from(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.rope, formatter)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::change::Edit;
    use crate::testing::{changes, cursor_positions, text};

    fn lines(buffer: &Buffer) -> Vec<String> {
        (0..buffer.line_count())
            .map(|index| buffer.line(index).unwrap().into_owned())
            .collect()
    }

    #[test]
    fn an_empty_buffer_has_one_empty_line() {
        let buffer = Buffer::default();

        assert!(buffer.is_empty());
        assert_eq!(lines(&buffer), [""]);
    }

    #[test]
    fn a_buffer_keeps_its_text() {
        let buffer = Buffer::from("Presswork\n");

        assert_eq!(buffer.to_string(), "Presswork\n");
        assert_eq!(buffer.len(), 10);
    }

    #[test]
    fn lines_break_at_the_typst_line_breaks() {
        let buffer = Buffer::from("a\nb\r\nc\rd\u{0B}e\u{0C}f\u{85}g\u{2028}h\u{2029}i");

        assert_eq!(
            lines(&buffer),
            ["a", "b", "c", "d", "e", "f", "g", "h", "i"]
        );
    }

    #[test]
    fn a_line_break_at_the_end_starts_an_empty_line() {
        let buffer = Buffer::from("a\r\n");

        assert_eq!(lines(&buffer), ["a", ""]);
        assert_eq!(buffer.line(2), None);
    }

    #[test]
    fn slice_gives_the_text_of_a_range() {
        let buffer = Buffer::from("Typst and Presswork");

        assert_eq!(buffer.slice(Position::new(6)..Position::new(9)), "and");
    }

    #[test]
    fn columns_count_bytes_characters_or_utf16_units() {
        let buffer = Buffer::from("x\naé😀b");
        let before_b = Position::new(9);

        for (unit, column) in [(Unit::Byte, 7), (Unit::Char, 3), (Unit::Utf16, 4)] {
            assert_eq!(
                buffer.line_column(before_b, unit),
                LineColumn { line: 1, column }
            );
            assert_eq!(
                buffer.position(LineColumn { line: 1, column }, unit),
                before_b
            );
        }
    }

    #[test]
    fn a_position_in_a_character_moves_to_its_start() {
        let buffer = Buffer::from("é");

        assert_eq!(
            buffer.line_column(Position::new(1), Unit::Byte),
            LineColumn { line: 0, column: 0 }
        );
    }

    #[test]
    fn a_column_after_the_line_end_stops_at_the_line_end() {
        let buffer = Buffer::from("ab\r\ncd");

        assert_eq!(
            buffer.position(LineColumn { line: 0, column: 9 }, Unit::Char),
            Position::new(2)
        );
    }

    #[test]
    fn a_line_after_the_last_line_stops_at_the_end_of_the_text() {
        let buffer = Buffer::from("ab\ncd");

        assert_eq!(
            buffer.position(LineColumn { line: 7, column: 0 }, Unit::Char),
            Position::new(5)
        );
    }

    #[test]
    fn apply_replaces_the_ranges_of_the_edits() {
        let mut buffer = Buffer::from("Typst is fast");
        let changes = ChangeSet::new(
            buffer.len(),
            [
                Edit::insert(Position::new(0), "The "),
                Edit::replace(Position::new(9)..Position::new(13), "very fast"),
            ],
        )
        .unwrap();

        buffer.apply(&changes).unwrap();

        assert_eq!(buffer.to_string(), "The Typst is very fast");
    }

    #[test]
    fn apply_needs_changes_for_the_same_length() {
        let mut buffer = Buffer::from("abc");

        let error = buffer.apply(&ChangeSet::empty(4)).unwrap_err();

        assert_eq!(
            error,
            TextError::LengthMismatch {
                expected: 4,
                actual: 3
            }
        );
    }

    #[test]
    fn apply_keeps_the_text_if_an_edit_splits_a_character() {
        let mut buffer = Buffer::from("aé");
        let changes = ChangeSet::new(
            buffer.len(),
            [
                Edit::insert(Position::new(0), "x"),
                Edit::insert(Position::new(2), "y"),
            ],
        )
        .unwrap();

        let error = buffer.apply(&changes).unwrap_err();

        assert_eq!(error, TextError::NotCharBoundary(2));
        assert_eq!(buffer.to_string(), "aé");
    }

    proptest! {
        #[test]
        fn apply_agrees_with_a_string((text, edits) in changes()) {
            let mut buffer = Buffer::from(text.as_str());
            let changes = ChangeSet::new(text.len(), edits.clone()).unwrap();

            buffer.apply(&changes).unwrap();

            prop_assert_eq!(buffer.to_string(), crate::testing::apply(&text, &edits));
        }

        #[test]
        fn line_column_and_position_agree(text in text(), pick: prop::sample::Index) {
            let buffer = Buffer::from(text.as_str());
            let positions = cursor_positions(&text);
            let position = positions[pick.index(positions.len())];

            for unit in [Unit::Byte, Unit::Char, Unit::Utf16] {
                let line_column = buffer.line_column(position, unit);
                prop_assert_eq!(buffer.position(line_column, unit), position);
            }
        }
    }
}
