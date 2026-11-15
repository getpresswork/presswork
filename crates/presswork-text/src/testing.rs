use proptest::prelude::*;

use crate::Position;

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
