use std::ops::Range;

use crate::change::{Assoc, ChangeSet};
use crate::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    Insertion,
    Deletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub range: Range<Position>,
    pub author: String,
    pub time: i64,
}

impl Suggestion {
    #[must_use]
    pub fn map(&self, changes: &ChangeSet) -> Option<Self> {
        let start = changes.map_position(self.range.start, Assoc::After);
        let end = changes.map_position(self.range.end, Assoc::Before);
        (start < end).then(|| Self {
            range: start..end,
            ..self.clone()
        })
    }
}

pub(crate) fn map_all(suggestions: &[Suggestion], changes: &ChangeSet) -> Vec<Suggestion> {
    suggestions
        .iter()
        .filter_map(|suggestion| suggestion.map(changes))
        .collect()
}
