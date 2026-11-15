#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    #[error("the range {start}..{end} is not in a text of {len} bytes")]
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("the edit at {start} starts before the end of the previous edit at {previous_end}")]
    Unsorted { start: usize, previous_end: usize },
    #[error("the changes are for a text of {expected} bytes, but the text has {actual} bytes")]
    LengthMismatch { expected: usize, actual: usize },
    #[error("the position {0} is not at the start of a character")]
    NotCharBoundary(usize),
}
