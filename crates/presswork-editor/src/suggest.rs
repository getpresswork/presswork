use std::ops::Range;

use presswork_text::{
    Buffer, ChangeSet, Edit, Position, Selection, SelectionRange, Suggestion, SuggestionKind,
    TextError,
};

pub(crate) struct Suggested {
    pub(crate) changes: ChangeSet,
    pub(crate) suggestions: Vec<Suggestion>,
    pub(crate) selection: Selection,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Remove,
    Keep,
    Strike,
}

struct Plan {
    removals: Vec<Range<usize>>,
    strikes: Vec<Range<usize>>,
    insertions: Vec<(usize, String)>,
}

impl Plan {
    fn shift(&self, old: usize, after_insertions: bool) -> usize {
        let removed: usize = self
            .removals
            .iter()
            .map(|range| range.end.min(old).saturating_sub(range.start))
            .sum();
        let inserted: usize = self
            .insertions
            .iter()
            .filter(|(at, _)| *at < old || (after_insertions && *at == old))
            .map(|(_, text)| text.len())
            .sum();
        old - removed + inserted
    }
}

pub(crate) fn suggest(
    buffer: &Buffer,
    existing: &[Suggestion],
    selection: &Selection,
    edits: &[Edit],
    author: &str,
    time: i64,
) -> Result<Suggested, TextError> {
    let mut plan = Plan {
        removals: Vec::new(),
        strikes: Vec::new(),
        insertions: Vec::new(),
    };
    for edit in edits {
        let (start, end) = (edit.range.start.offset(), edit.range.end.offset());
        for (piece, range) in pieces(start, end, existing) {
            match piece {
                Piece::Remove => plan.removals.push(range),
                Piece::Strike => plan.strikes.push(range),
                Piece::Keep => {}
            }
        }
        if !edit.text.is_empty() {
            plan.insertions.push((end, edit.text.clone()));
        }
    }
    let mut operations: Vec<Edit> = plan
        .removals
        .iter()
        .map(|range| Edit::delete(Position::new(range.start)..Position::new(range.end)))
        .chain(
            plan.insertions
                .iter()
                .map(|(at, text)| Edit::insert(Position::new(*at), text.clone())),
        )
        .collect();
    operations.sort_by_key(|edit| (edit.range.start, edit.range.end));
    let changes = ChangeSet::new(buffer.len(), operations)?;
    let mut suggestions: Vec<Suggestion> = existing
        .iter()
        .filter_map(|suggestion| suggestion.map(&changes))
        .collect();
    let made = |kind, range: Range<usize>| Suggestion {
        kind,
        range: Position::new(range.start)..Position::new(range.end),
        author: author.to_owned(),
        time,
    };
    for strike in &plan.strikes {
        let start = plan.shift(strike.start, true);
        let end = plan.shift(strike.end, false);
        if start < end {
            suggestions.push(made(SuggestionKind::Deletion, start..end));
        }
    }
    for (at, text) in &plan.insertions {
        if !inside_insertion(*at, existing) {
            let start = plan.shift(*at, false);
            suggestions.push(made(SuggestionKind::Insertion, start..start + text.len()));
        }
    }
    let cursors = selection
        .ranges()
        .iter()
        .map(|range| SelectionRange::cursor(Position::new(new_head(*range, edits, &plan))))
        .collect();
    Ok(Suggested {
        changes,
        suggestions: normalize(suggestions),
        selection: Selection::new(cursors, selection.primary_index()),
    })
}

fn pieces(start: usize, end: usize, existing: &[Suggestion]) -> Vec<(Piece, Range<usize>)> {
    let mut marks: Vec<(usize, usize, SuggestionKind)> = existing
        .iter()
        .map(|suggestion| {
            (
                suggestion.range.start.offset(),
                suggestion.range.end.offset(),
                suggestion.kind,
            )
        })
        .filter(|(from, to, _)| *from < end && *to > start)
        .collect();
    marks.sort_by_key(|(from, to, _)| (*from, *to));
    let mut result = Vec::new();
    let mut at = start;
    for (from, to, kind) in marks {
        let (from, to) = (from.max(at), to.min(end));
        if from >= to {
            continue;
        }
        if at < from {
            result.push((Piece::Strike, at..from));
        }
        let piece = match kind {
            SuggestionKind::Insertion => Piece::Remove,
            SuggestionKind::Deletion => Piece::Keep,
        };
        result.push((piece, from..to));
        at = to;
    }
    if at < end {
        result.push((Piece::Strike, at..end));
    }
    result
}

fn inside_insertion(at: usize, existing: &[Suggestion]) -> bool {
    existing.iter().any(|suggestion| {
        suggestion.kind == SuggestionKind::Insertion
            && suggestion.range.start.offset() < at
            && at < suggestion.range.end.offset()
    })
}

fn new_head(range: SelectionRange, edits: &[Edit], plan: &Plan) -> usize {
    let (start, end, head) = (
        range.start().offset(),
        range.end().offset(),
        range.head.offset(),
    );
    for edit in edits {
        let (from, to) = (edit.range.start.offset(), edit.range.end.offset());
        if !edit.text.is_empty() && from <= head && head <= to {
            return plan.shift(to, false) + edit.text.len();
        }
        if edit.text.is_empty() {
            if !range.is_empty() && from == start && to == end {
                return plan.shift(to, false);
            }
            if range.is_empty() && head == to && from < to {
                return plan.shift(from, false);
            }
            if range.is_empty() && head == from && from < to {
                return plan.shift(to, false);
            }
        }
    }
    plan.shift(head, false)
}

fn normalize(suggestions: Vec<Suggestion>) -> Vec<Suggestion> {
    let mut insertions: Vec<Range<Position>> = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == SuggestionKind::Insertion)
        .map(|suggestion| suggestion.range.clone())
        .collect();
    insertions.sort_by_key(|range| range.start);
    let mut pieces = Vec::new();
    for suggestion in suggestions {
        if suggestion.kind == SuggestionKind::Insertion {
            pieces.push(suggestion);
            continue;
        }
        let mut start = suggestion.range.start;
        for insertion in &insertions {
            if insertion.start >= suggestion.range.end || insertion.end <= start {
                continue;
            }
            if insertion.start > start {
                pieces.push(Suggestion {
                    range: start..insertion.start,
                    ..suggestion.clone()
                });
            }
            start = start.max(insertion.end);
        }
        if start < suggestion.range.end {
            pieces.push(Suggestion {
                range: start..suggestion.range.end,
                ..suggestion
            });
        }
    }
    pieces.sort_by_key(|suggestion| (suggestion.range.start, suggestion.range.end));
    let mut merged: Vec<Suggestion> = Vec::new();
    for suggestion in pieces {
        match merged.last_mut() {
            Some(last)
                if last.kind == suggestion.kind
                    && last.author == suggestion.author
                    && suggestion.range.start <= last.range.end =>
            {
                last.range.end = last.range.end.max(suggestion.range.end);
                last.time = last.time.max(suggestion.time);
            }
            _ => merged.push(suggestion),
        }
    }
    merged
}
