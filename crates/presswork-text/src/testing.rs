use proptest::prelude::*;

use crate::{Edit, Position};

const PIECES: &[&str] = &[
    "a", "b", "ab", " ", "é", "中", "😀", "\n", "\r\n", "\r", "\u{2028}",
];

pub(crate) fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(PIECES), 0..24).prop_map(|pieces| pieces.concat())
}

pub(crate) fn cursor_positions(text: &str) -> Vec<Position> {
    let bytes = text.as_bytes();
    (0..=text.len())
        .filter(|&offset| text.is_char_boundary(offset))
        .filter(|&offset| {
            !(offset > 0 && bytes[offset - 1] == b'\r' && bytes.get(offset) == Some(&b'\n'))
        })
        .map(Position::new)
        .collect()
}

fn inserted_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(PIECES), 0..3).prop_map(|pieces| pieces.concat())
}

pub(crate) type RawEdits = Vec<(prop::sample::Index, prop::sample::Index, String)>;

pub(crate) fn raw_edits() -> impl Strategy<Value = RawEdits> {
    prop::collection::vec(
        (
            any::<prop::sample::Index>(),
            any::<prop::sample::Index>(),
            inserted_text(),
        ),
        0..4,
    )
}

pub(crate) fn changes() -> impl Strategy<Value = (String, Vec<Edit>)> {
    (text(), raw_edits()).prop_map(|(text, raw)| {
        let edits = edits_for(&text, raw);
        (text, edits)
    })
}

pub(crate) fn edits_for(text: &str, raw: RawEdits) -> Vec<Edit> {
    let boundaries: Vec<usize> = (0..=text.len())
        .filter(|&offset| text.is_char_boundary(offset))
        .collect();
    let mut ranges: Vec<(usize, usize, String)> = raw
        .into_iter()
        .map(|(first, second, inserted)| {
            let first = boundaries[first.index(boundaries.len())];
            let second = boundaries[second.index(boundaries.len())];
            (first.min(second), first.max(second), inserted)
        })
        .collect();
    ranges.sort_by_key(|(start, end, _)| (*start, *end));
    let mut edits = Vec::new();
    let mut previous_end = 0;
    for (start, end, inserted) in ranges {
        if start >= previous_end {
            edits.push(Edit::replace(
                Position::new(start)..Position::new(end),
                inserted,
            ));
            previous_end = end;
        }
    }
    edits
}

pub(crate) fn apply(text: &str, edits: &[Edit]) -> String {
    let mut result = text.to_owned();
    for edit in edits.iter().rev() {
        result.replace_range(
            edit.range.start.offset()..edit.range.end.offset(),
            &edit.text,
        );
    }
    result
}
