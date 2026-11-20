use std::ops::Range;

use presswork_text::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Replace {
        range: Range<Position>,
        text: String,
        select: Range<usize>,
    },
    ReplaceAll(Vec<(Range<Position>, String)>),
    AddCursor(Position),
    AddCursorLine {
        down: bool,
    },
    SelectNext,
    SelectEvery,
    SingleCursor,
    MoveEach {
        heads: Vec<Position>,
        extend: bool,
    },
    Paste(String),
    Fold(Range<Position>),
    Unfold(Range<Position>),
    Move {
        motion: Motion,
        extend: bool,
    },
    MoveTo {
        position: Position,
        extend: bool,
        goal: Option<f32>,
    },
    SelectAll,
    SelectWord(Position),
    SelectLine(Position),
    Insert(String),
    Newline,
    Delete(Deletion),
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deletion {
    Backward,
    Forward,
    WordBackward,
    WordForward,
}
