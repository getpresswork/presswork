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

#[must_use]
pub fn group_at(suggestions: &[Suggestion], position: Position) -> Option<Range<Position>> {
    let mut sorted: Vec<&Suggestion> = suggestions.iter().collect();
    sorted.sort_by_key(|suggestion| (suggestion.range.start, suggestion.range.end));
    let found = sorted.iter().position(|suggestion| {
        suggestion.range.contains(&position) || suggestion.range.end == position
    })?;
    let joined = |before: &Suggestion, after: &Suggestion| {
        before.range.end == after.range.start && before.author == after.author
    };
    let mut first = found;
    while first > 0 && joined(sorted[first - 1], sorted[first]) {
        first -= 1;
    }
    let mut last = found;
    while last + 1 < sorted.len() && joined(sorted[last], sorted[last + 1]) {
        last += 1;
    }
    Some(sorted[first].range.start..sorted[last].range.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestion(kind: SuggestionKind, range: Range<usize>, author: &str) -> Suggestion {
        Suggestion {
            kind,
            range: Position::new(range.start)..Position::new(range.end),
            author: author.to_owned(),
            time: 0,
        }
    }

    fn group(suggestions: &[Suggestion], at: usize) -> Option<Range<usize>> {
        group_at(suggestions, Position::new(at))
            .map(|range| range.start.offset()..range.end.offset())
    }

    #[test]
    fn a_group_has_the_side_by_side_suggestions_of_one_author() {
        let suggestions = [
            suggestion(SuggestionKind::Insertion, 7..10, "Ann"),
            suggestion(SuggestionKind::Deletion, 4..7, "Ann"),
            suggestion(SuggestionKind::Deletion, 10..12, "Bob"),
            suggestion(SuggestionKind::Deletion, 15..18, "Ann"),
        ];

        assert_eq!(group(&suggestions, 5), Some(4..10));
        assert_eq!(group(&suggestions, 4), Some(4..10));
        assert_eq!(group(&suggestions, 11), Some(10..12));
        assert_eq!(group(&suggestions, 16), Some(15..18));
        assert_eq!(group(&suggestions, 13), None);
    }
}
