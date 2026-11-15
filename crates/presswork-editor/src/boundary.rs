use std::ops::Range;

use presswork_text::{Buffer, Position};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Space,
    Word,
    Punctuation,
}

fn kind(segment: &str) -> Kind {
    if segment.chars().all(char::is_whitespace) {
        Kind::Space
    } else if segment.chars().any(char::is_alphanumeric) {
        Kind::Word
    } else {
        Kind::Punctuation
    }
}

fn offset(position: Position, by: usize) -> Position {
    Position::new(position.offset() + by)
}

pub(crate) fn previous_grapheme(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let start = buffer.line_start(line);
    if position <= start {
        return if line == 0 {
            start
        } else {
            buffer.line_end(line - 1)
        };
    }
    let text = buffer.slice(start..position);
    let index = text
        .grapheme_indices(true)
        .next_back()
        .map_or(0, |(index, _)| index);
    offset(start, index)
}

pub(crate) fn next_grapheme(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let end = buffer.line_end(line);
    if position >= end {
        return if line + 1 < buffer.line_count() {
            buffer.line_start(line + 1)
        } else {
            end
        };
    }
    let text = buffer.slice(position..end);
    offset(position, text.graphemes(true).next().map_or(0, str::len))
}

pub(crate) fn previous_word(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let start = buffer.line_start(line);
    if position <= start {
        return previous_grapheme(buffer, position);
    }
    let text = buffer.slice(start..position);
    let mut segments = text
        .split_word_bound_indices()
        .rev()
        .skip_while(|(_, segment)| kind(segment) == Kind::Space);
    let Some((mut index, first)) = segments.next() else {
        return start;
    };
    if kind(first) == Kind::Punctuation {
        for (segment_index, segment) in segments {
            if kind(segment) != Kind::Punctuation {
                break;
            }
            index = segment_index;
        }
    }
    offset(start, index)
}

pub(crate) fn next_word(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let end = buffer.line_end(line);
    if position >= end {
        return next_grapheme(buffer, position);
    }
    let text = buffer.slice(position..end);
    let mut segments = text
        .split_word_bound_indices()
        .skip_while(|(_, segment)| kind(segment) == Kind::Space);
    let Some((index, first)) = segments.next() else {
        return end;
    };
    let mut stop = index + first.len();
    if kind(first) == Kind::Punctuation {
        for (segment_index, segment) in segments {
            if kind(segment) != Kind::Punctuation {
                break;
            }
            stop = segment_index + segment.len();
        }
    }
    offset(position, stop)
}

pub(crate) fn smart_line_start(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let start = buffer.line_start(line);
    let text = buffer.line(line).unwrap_or_default();
    let first = offset(start, text.len() - text.trim_start().len());
    if position == first { start } else { first }
}

pub(crate) fn word_at(buffer: &Buffer, position: Position) -> Range<Position> {
    let line = buffer.line_of(position);
    let start = buffer.line_start(line);
    let text = buffer.line(line).unwrap_or_default();
    let column = (position.offset() - start.offset()).min(text.len());
    let segments: Vec<(usize, &str)> = text.split_word_bound_indices().collect();
    let Some(last) = segments.len().checked_sub(1) else {
        return position..position;
    };
    let containing = segments
        .iter()
        .position(|(index, segment)| *index <= column && column < index + segment.len())
        .unwrap_or(last);
    let chosen = match containing.checked_sub(1) {
        Some(previous)
            if kind(segments[containing].1) != Kind::Word
                && kind(segments[previous].1) == Kind::Word
                && segments[containing].0 == column =>
        {
            previous
        }
        _ => containing,
    };
    let (index, segment) = segments[chosen];
    offset(start, index)..offset(start, index + segment.len())
}

pub(crate) fn line_at(buffer: &Buffer, position: Position) -> Range<Position> {
    let line = buffer.line_of(position);
    buffer.line_start(line)..buffer.line_start(line + 1)
}

pub(crate) fn indentation(buffer: &Buffer, position: Position) -> String {
    let start = buffer.line_start(buffer.line_of(position));
    buffer
        .slice(start..position)
        .chars()
        .take_while(|char| char.is_whitespace())
        .collect()
}

pub(crate) fn line_break(buffer: &Buffer) -> &'static str {
    let first_break = buffer.line_end(0)..buffer.line_start(1);
    if buffer.line_count() > 1 && buffer.slice(first_break) == "\r\n" {
        "\r\n"
    } else {
        "\n"
    }
}
