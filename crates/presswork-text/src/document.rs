use std::time::Instant;

use crate::buffer::Buffer;
use crate::change::ChangeSet;
use crate::error::TextError;
use crate::history::History;
use crate::position::Position;
use crate::selection::Selection;
use crate::suggestion::{Suggestion, map_all};
use crate::transaction::Transaction;

#[derive(Debug, Clone)]
pub struct Document {
    buffer: Buffer,
    selection: Selection,
    history: History,
    suggestions: Vec<Suggestion>,
}

impl Document {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            buffer: Buffer::from(text),
            selection: Selection::cursor(Position::default()),
            history: History::default(),
            suggestions: Vec::new(),
        }
    }

    #[must_use]
    pub fn suggestions(&self) -> &[Suggestion] {
        &self.suggestions
    }

    #[must_use]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    #[must_use]
    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn apply(&mut self, transaction: Transaction, at: Instant) -> Result<(), TextError> {
        let Transaction {
            changes,
            selection,
            suggestions,
        } = transaction;
        let inverse = changes.invert(&self.buffer)?;
        self.buffer.apply(&changes)?;
        let checked = match &selection {
            Some(selection) => self.buffer.check_selection(selection),
            None => Ok(()),
        }
        .and_then(|()| {
            suggestions
                .iter()
                .flatten()
                .try_for_each(|suggestion| self.buffer.check_range(&suggestion.range))
        });
        if let Err(error) = checked {
            self.buffer.apply(&inverse)?;
            return Err(error);
        }
        let selection = selection.unwrap_or_else(|| self.selection.map(&changes));
        let mapped = map_all(&self.suggestions, &changes);
        let suggested = suggestions.as_ref().is_some_and(|next| *next != mapped);
        let earlier = std::mem::replace(&mut self.suggestions, suggestions.unwrap_or(mapped));
        let before = std::mem::replace(&mut self.selection, selection);
        if !changes.is_empty() || suggested {
            self.history
                .record(inverse, &changes, before, earlier, at)?;
        } else if self.selection != before {
            self.history.break_group();
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<Option<ChangeSet>, TextError> {
        let transaction = self
            .history
            .undo(&self.buffer, &self.selection, &self.suggestions)?;
        self.restore(transaction)
    }

    pub fn redo(&mut self) -> Result<Option<ChangeSet>, TextError> {
        let transaction = self
            .history
            .redo(&self.buffer, &self.selection, &self.suggestions)?;
        self.restore(transaction)
    }

    pub fn apply_remote(&mut self, changes: &ChangeSet) -> Result<(), TextError> {
        self.buffer.check(changes)?;
        self.history.rebase(changes)?;
        self.buffer.apply(changes)?;
        self.selection = self.selection.map(changes);
        self.suggestions = map_all(&self.suggestions, changes);
        Ok(())
    }

    fn restore(
        &mut self,
        transaction: Option<Transaction>,
    ) -> Result<Option<ChangeSet>, TextError> {
        let Some(Transaction {
            changes,
            selection,
            suggestions,
        }) = transaction
        else {
            return Ok(None);
        };
        self.buffer.apply(&changes)?;
        self.selection = selection.unwrap_or_else(|| self.selection.map(&changes));
        self.suggestions = suggestions.unwrap_or_else(|| map_all(&self.suggestions, &changes));
        Ok(Some(changes))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use proptest::prelude::*;

    use super::*;
    use crate::change::Edit;
    use crate::selection::SelectionRange;
    use crate::suggestion::SuggestionKind;
    use crate::testing::{RawEdits, edits_for, raw_edits, text};

    fn at(offset: usize) -> Position {
        Position::new(offset)
    }

    fn text_of(document: &Document) -> String {
        document.buffer().to_string()
    }

    fn edit(document: &Document, edits: Vec<Edit>) -> Transaction {
        Transaction::new(ChangeSet::new(document.buffer().len(), edits).unwrap())
    }

    fn type_at(document: &mut Document, offset: usize, text: &str, time: Instant) {
        let transaction = edit(document, vec![Edit::insert(at(offset), text)])
            .with_selection(Selection::cursor(at(offset + text.len())));
        document.apply(transaction, time).unwrap();
    }

    #[test]
    fn apply_changes_the_text_and_the_selection() {
        let mut document = Document::new("Typst");

        type_at(&mut document, 0, "Hi ", Instant::now());

        assert_eq!(text_of(&document), "Hi Typst");
        assert_eq!(document.selection(), &Selection::cursor(at(3)));
    }

    #[test]
    fn without_a_new_selection_the_selection_moves_with_the_text() {
        let mut document = Document::new("Typst");
        let start = Instant::now();
        let select = edit(&document, vec![])
            .with_selection(Selection::single(SelectionRange::new(at(1), at(4))));
        document.apply(select, start).unwrap();

        document
            .apply(edit(&document, vec![Edit::insert(at(0), "ab")]), start)
            .unwrap();

        assert_eq!(
            document.selection(),
            &Selection::single(SelectionRange::new(at(3), at(6)))
        );
    }

    #[test]
    fn a_selection_must_be_in_the_new_text() {
        let mut document = Document::new("abc");
        let transaction = edit(&document, vec![Edit::delete(at(0)..at(2))])
            .with_selection(Selection::cursor(at(3)));

        let error = document.apply(transaction, Instant::now()).unwrap_err();

        assert_eq!(
            error,
            TextError::InvalidRange {
                start: 3,
                end: 3,
                len: 1
            }
        );
        assert_eq!(text_of(&document), "abc");
    }

    #[test]
    fn undo_and_redo_restore_the_text_and_the_selection() {
        let mut document = Document::new("Typst");
        type_at(&mut document, 5, "!", Instant::now());

        assert!(document.undo().unwrap().is_some());
        assert_eq!(text_of(&document), "Typst");
        assert_eq!(document.selection(), &Selection::cursor(at(0)));

        assert!(document.redo().unwrap().is_some());
        assert_eq!(text_of(&document), "Typst!");
        assert_eq!(document.selection(), &Selection::cursor(at(6)));
    }

    #[test]
    fn undo_with_an_empty_history_changes_nothing() {
        let mut document = Document::new("Typst");

        assert!(document.undo().unwrap().is_none());
        assert!(document.redo().unwrap().is_none());
        assert_eq!(text_of(&document), "Typst");
    }

    fn applied(text: &str, changes: &ChangeSet) -> String {
        let mut buffer = Buffer::from(text);
        buffer.apply(changes).unwrap();
        buffer.to_string()
    }

    #[test]
    fn undo_and_redo_give_the_changes_that_they_apply() {
        let mut document = Document::new("Typst");
        type_at(&mut document, 5, "!", Instant::now());

        let undone = document.undo().unwrap().unwrap();
        let redone = document.redo().unwrap().unwrap();

        assert_eq!(applied("Typst!", &undone), "Typst");
        assert_eq!(applied("Typst", &redone), "Typst!");
    }

    #[test]
    fn edits_close_in_time_and_place_undo_together() {
        let mut document = Document::new("");
        let mut time = Instant::now();
        for (offset, letter) in ["a", "b", "c"].into_iter().enumerate() {
            type_at(&mut document, offset, letter, time);
            time += Duration::from_millis(100);
        }

        document.undo().unwrap();

        assert_eq!(text_of(&document), "");
        assert!(!document.history().can_undo());
    }

    #[test]
    fn a_pause_starts_a_new_undo_group() {
        let mut document = Document::new("");
        let start = Instant::now();
        type_at(&mut document, 0, "a", start);
        type_at(&mut document, 1, "b", start + Duration::from_millis(600));

        document.undo().unwrap();

        assert_eq!(text_of(&document), "a");
    }

    #[test]
    fn an_edit_at_another_place_starts_a_new_undo_group() {
        let mut document = Document::new("0123456789");
        let start = Instant::now();
        type_at(&mut document, 0, "a", start);
        type_at(&mut document, 9, "b", start + Duration::from_millis(50));

        document.undo().unwrap();

        assert_eq!(text_of(&document), "a0123456789");
    }

    #[test]
    fn a_new_edit_removes_the_redo() {
        let mut document = Document::new("");
        let start = Instant::now();
        type_at(&mut document, 0, "a", start);
        document.undo().unwrap();

        type_at(&mut document, 0, "b", start + Duration::from_secs(1));

        assert!(!document.history().can_redo());
    }

    #[test]
    fn remote_changes_move_the_selection() {
        let mut document = Document::new("Typst");
        type_at(&mut document, 5, "!", Instant::now());

        document
            .apply_remote(&ChangeSet::new(6, [Edit::insert(at(0), "Hi ")]).unwrap())
            .unwrap();

        assert_eq!(text_of(&document), "Hi Typst!");
        assert_eq!(document.selection(), &Selection::cursor(at(9)));
    }

    #[test]
    fn undo_keeps_the_text_of_remote_changes() {
        let mut document = Document::new("abc");
        type_at(&mut document, 3, "X", Instant::now());
        document
            .apply_remote(&ChangeSet::new(4, [Edit::insert(at(0), "R")]).unwrap())
            .unwrap();

        document.undo().unwrap();
        assert_eq!(text_of(&document), "Rabc");

        document.redo().unwrap();
        assert_eq!(text_of(&document), "RabcX");
    }

    #[test]
    fn undo_skips_an_edit_that_a_remote_change_removed() {
        let mut document = Document::new("abc");
        let start = Instant::now();
        type_at(&mut document, 3, "X", start);
        type_at(&mut document, 0, "Y", start + Duration::from_secs(1));
        document
            .apply_remote(&ChangeSet::new(5, [Edit::delete(at(4)..at(5))]).unwrap())
            .unwrap();

        assert!(document.undo().unwrap().is_some());
        assert_eq!(text_of(&document), "abc");
        assert!(document.undo().unwrap().is_none());
    }

    fn suggestion(kind: SuggestionKind, range: std::ops::Range<usize>) -> Suggestion {
        Suggestion {
            kind,
            range: at(range.start)..at(range.end),
            author: "Ann".to_owned(),
            time: 0,
        }
    }

    fn insertion(range: std::ops::Range<usize>) -> Suggestion {
        suggestion(SuggestionKind::Insertion, range)
    }

    fn deletion(range: std::ops::Range<usize>) -> Suggestion {
        suggestion(SuggestionKind::Deletion, range)
    }

    fn suggest(document: &mut Document, suggestions: Vec<Suggestion>) {
        let transaction = edit(document, vec![]).with_suggestions(suggestions);
        document.apply(transaction, Instant::now()).unwrap();
    }

    #[test]
    fn suggestions_move_with_the_text_around_them() {
        let mut document = Document::new("abcdef");
        suggest(&mut document, vec![insertion(2..4)]);

        let edits = vec![
            Edit::insert(at(0), "xy"),
            Edit::insert(at(2), "1"),
            Edit::insert(at(4), "2"),
        ];
        document
            .apply(edit(&document, edits), Instant::now())
            .unwrap();

        assert_eq!(text_of(&document), "xyab1cd2ef");
        assert_eq!(document.suggestions(), [insertion(5..7)]);
    }

    #[test]
    fn a_suggestion_without_text_goes_away() {
        let mut document = Document::new("abcdef");
        suggest(&mut document, vec![deletion(2..4)]);

        document
            .apply(
                edit(&document, vec![Edit::delete(at(1)..at(5))]),
                Instant::now(),
            )
            .unwrap();

        assert_eq!(document.suggestions(), []);
    }

    #[test]
    fn a_change_of_the_suggestions_alone_is_one_undo_step() {
        let mut document = Document::new("abcdef");
        suggest(&mut document, vec![deletion(1..3)]);
        assert_eq!(document.suggestions(), [deletion(1..3)]);

        assert!(document.undo().unwrap().is_some());
        assert_eq!(document.suggestions(), []);
        assert!(document.redo().unwrap().is_some());
        assert_eq!(document.suggestions(), [deletion(1..3)]);
        assert_eq!(text_of(&document), "abcdef");
    }

    #[test]
    fn undo_of_a_suggested_insertion_removes_its_text_and_its_mark() {
        let mut document = Document::new("abc");
        let transaction = edit(&document, vec![Edit::insert(at(3), "XY")])
            .with_selection(Selection::cursor(at(5)))
            .with_suggestions(vec![insertion(3..5)]);
        document.apply(transaction, Instant::now()).unwrap();

        document.undo().unwrap();

        assert_eq!(text_of(&document), "abc");
        assert_eq!(document.suggestions(), []);
    }

    #[test]
    fn remote_changes_move_the_suggestions_and_keep_their_undo() {
        let mut document = Document::new("abc");
        suggest(&mut document, vec![deletion(1..2)]);

        document
            .apply_remote(&ChangeSet::new(3, [Edit::insert(at(0), "R")]).unwrap())
            .unwrap();

        assert_eq!(document.suggestions(), [deletion(2..3)]);
        assert!(document.undo().unwrap().is_some());
        assert_eq!(document.suggestions(), []);
    }

    fn apply_local(document: &mut Document, raw: RawEdits, time: Instant) {
        let edits = edits_for(&text_of(document), raw);
        document.apply(edit(document, edits), time).unwrap();
    }

    proptest! {
        #[test]
        fn undo_of_all_edits_restores_the_text(
            text in text(),
            steps in prop::collection::vec(raw_edits(), 1..6),
        ) {
            let mut document = Document::new(&text);
            let mut time = Instant::now();
            for raw in steps {
                apply_local(&mut document, raw, time);
                time += Duration::from_secs(1);
            }

            while document.undo().unwrap().is_some() {}

            prop_assert_eq!(text_of(&document), text);
        }

        #[test]
        fn undo_never_removes_remote_text(
            text in text(),
            steps in prop::collection::vec(raw_edits(), 1..6),
            remote in raw_edits(),
        ) {
            let mut document = Document::new(&text);
            let mut time = Instant::now();
            for raw in steps {
                apply_local(&mut document, raw, time);
                time += Duration::from_millis(300);
            }
            let remote = edits_for(&text_of(&document), remote);
            let inserted: Vec<String> = remote.iter().map(|edit| edit.text.clone()).collect();
            document
                .apply_remote(&ChangeSet::new(document.buffer().len(), remote).unwrap())
                .unwrap();
            let before_undo = text_of(&document);

            while document.undo().unwrap().is_some() {}
            let undone = text_of(&document);
            while document.redo().unwrap().is_some() {}

            for text in inserted {
                prop_assert!(undone.contains(&text));
            }
            prop_assert_eq!(text_of(&document), before_undo);
        }
    }
}
