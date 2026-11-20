use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use std::ops::Range;

use presswork_text::{
    Assoc, Buffer, ChangeSet, Document, Edit, LineColumn, Position, Query, Search, Selection,
    SelectionRange, TextError, Transaction, Unit,
};

use crate::action::{Action, Deletion, Motion};
use crate::boundary;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Editor {
    id: u64,
    document: Document,
    revision: u64,
    goal: Option<f32>,
    word_search: bool,
    folds: Vec<Range<Position>>,
    fold_revision: u64,
}

impl Editor {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            document: Document::new(text),
            revision: 0,
            goal: None,
            word_search: false,
            folds: Vec::new(),
            fold_revision: 0,
        }
    }

    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn goal(&self) -> Option<f32> {
        self.goal
    }

    #[must_use]
    pub fn folds(&self) -> &[Range<Position>] {
        &self.folds
    }

    #[must_use]
    pub fn fold_revision(&self) -> u64 {
        self.fold_revision
    }

    #[must_use]
    pub fn selected_text(&self) -> String {
        let buffer = self.document.buffer();
        let parts: Vec<_> = self
            .document
            .selection()
            .ranges()
            .iter()
            .filter(|range| !range.is_empty())
            .map(|range| buffer.slice(range.start()..range.end()).into_owned())
            .collect();
        parts.join(boundary::line_break(buffer))
    }

    pub fn apply_remote(&mut self, changes: &ChangeSet) -> Result<(), TextError> {
        if changes.is_empty() {
            return Ok(());
        }
        self.document.apply_remote(changes)?;
        self.revision += 1;
        self.map_folds(changes);
        self.reveal();
        Ok(())
    }

    pub fn perform(
        &mut self,
        action: Action,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        self.goal = None;
        let word_search = std::mem::take(&mut self.word_search);
        let buffer = self.document.buffer();
        match action {
            Action::Move { motion, extend } => {
                let selection = self.document.selection();
                let ranges = selection
                    .ranges()
                    .iter()
                    .map(|range| self.over_folds(range.head, moved(buffer, *range, motion, extend)))
                    .collect();
                self.select(Selection::new(ranges, selection.primary_index()), now)
            }
            Action::MoveTo {
                position,
                extend,
                goal,
            } => {
                let anchor = if extend {
                    self.document.selection().primary().anchor
                } else {
                    position
                };
                self.select(
                    Selection::single(SelectionRange::new(anchor, position)),
                    now,
                )?;
                self.goal = goal;
                Ok(None)
            }
            Action::SelectAll => {
                let all = Position::default()..Position::new(buffer.len());
                self.select_range(all, now)
            }
            Action::SelectWord(position) => {
                let word = boundary::word_at(buffer, position);
                self.select_range(word, now)
            }
            Action::SelectLine(position) => {
                let line = boundary::line_at(buffer, position);
                self.select_range(line, now)
            }
            Action::Insert(text) => self.replace_selection(&text, now),
            Action::Replace {
                range,
                text,
                select,
            } => self.replace(range, &text, select, now),
            Action::ReplaceAll(replacements) => self.replace_all(replacements, now),
            Action::AddCursor(position) => self.add_cursor(position, now),
            Action::AddCursorLine { down } => self.add_cursor_line(down, now),
            Action::SelectNext => self.select_next(word_search, now),
            Action::SelectEvery => self.select_every(word_search, now),
            Action::SingleCursor => {
                let primary = self.document.selection().primary();
                self.select(Selection::single(primary), now)
            }
            Action::MoveEach { heads, extend } => self.move_each(&heads, extend, now),
            Action::Paste(text) => self.paste(&text, now),
            Action::Fold(hidden) => self.fold(hidden, now),
            Action::Unfold(hidden) => {
                self.unfold(&hidden);
                Ok(None)
            }
            Action::Newline => self.replace_each(
                |_, buffer, range| {
                    format!(
                        "{}{}",
                        boundary::line_break(buffer),
                        boundary::indentation(buffer, range.head)
                    )
                },
                now,
            ),
            Action::Delete(deletion) => self.delete(deletion, now),
            Action::Undo => {
                let changes = self.document.undo()?;
                self.count(changes.as_ref());
                Ok(changes)
            }
            Action::Redo => {
                let changes = self.document.redo()?;
                self.count(changes.as_ref());
                Ok(changes)
            }
        }
    }

    fn count(&mut self, changes: Option<&ChangeSet>) {
        if let Some(changes) = changes {
            self.revision += 1;
            self.map_folds(changes);
            self.reveal();
        }
    }

    fn select(
        &mut self,
        selection: Selection,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let len = self.document.buffer().len();
        let transaction = Transaction::new(ChangeSet::empty(len)).with_selection(selection);
        self.document.apply(transaction, now)?;
        self.reveal();
        Ok(None)
    }

    fn fold(
        &mut self,
        hidden: Range<Position>,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        if hidden.start >= hidden.end || self.folds.contains(&hidden) {
            return Ok(None);
        }
        let header_end = self.header_end(&hidden);
        let selection = self.document.selection();
        let ranges = selection
            .ranges()
            .iter()
            .map(|range| {
                if hidden.contains(&range.head) {
                    SelectionRange::cursor(header_end)
                } else {
                    *range
                }
            })
            .collect();
        let primary = selection.primary_index();
        self.folds.push(hidden);
        self.folds.sort_by_key(|fold| fold.start);
        self.fold_revision += 1;
        self.select(Selection::new(ranges, primary), now)
    }

    fn unfold(&mut self, hidden: &Range<Position>) {
        let before = self.folds.len();
        self.folds.retain(|fold| fold != hidden);
        if self.folds.len() != before {
            self.fold_revision += 1;
        }
    }

    fn header_end(&self, hidden: &Range<Position>) -> Position {
        let buffer = self.document.buffer();
        buffer.line_end(buffer.line_of(hidden.start).saturating_sub(1))
    }

    fn over_folds(&self, from: Position, range: SelectionRange) -> SelectionRange {
        let Some(fold) = self.folds.iter().find(|fold| fold.contains(&range.head)) else {
            return range;
        };
        let head = if range.head > from {
            fold.end
        } else {
            self.header_end(fold)
        };
        let anchor = if range.is_empty() { head } else { range.anchor };
        SelectionRange::new(anchor, head)
    }

    fn reveal(&mut self) {
        if self.folds.is_empty() {
            return;
        }
        let heads: Vec<Position> = self
            .document
            .selection()
            .ranges()
            .iter()
            .map(|range| range.head)
            .collect();
        let before = self.folds.len();
        self.folds
            .retain(|fold| !heads.iter().any(|head| fold.contains(head)));
        if self.folds.len() != before {
            self.fold_revision += 1;
        }
    }

    fn map_folds(&mut self, changes: &ChangeSet) {
        if self.folds.is_empty() {
            return;
        }
        let buffer = self.document.buffer();
        let mapped: Vec<Range<Position>> = self
            .folds
            .iter()
            .filter_map(|fold| {
                let start =
                    line_start_from(buffer, changes.map_position(fold.start, Assoc::Before));
                let end = line_start_from(buffer, changes.map_position(fold.end, Assoc::Before));
                (start.offset() > 0 && start < end).then_some(start..end)
            })
            .collect();
        if mapped != self.folds {
            self.folds = mapped;
            self.fold_revision += 1;
        }
    }

    fn select_range(
        &mut self,
        range: std::ops::Range<Position>,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        self.select(
            Selection::single(SelectionRange::new(range.start, range.end)),
            now,
        )
    }

    fn replace_selection(
        &mut self,
        text: &str,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        self.replace_each(|_, _, _| text.to_owned(), now)
    }

    fn replace_each(
        &mut self,
        text_for: impl Fn(usize, &Buffer, SelectionRange) -> String,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let selection = self.document.selection().clone();
        let buffer = self.document.buffer();
        let edits = selection.ranges().iter().enumerate().map(|(index, range)| {
            Edit::replace(range.start()..range.end(), text_for(index, buffer, *range))
        });
        let changes = ChangeSet::new(buffer.len(), edits)?;
        let cursors = selection
            .ranges()
            .iter()
            .map(|range| SelectionRange::cursor(changes.map_position(range.end(), Assoc::After)))
            .collect();
        let selection = Selection::new(cursors, selection.primary_index());
        self.change(Transaction::new(changes).with_selection(selection), now)
    }

    fn paste(&mut self, text: &str, now: Instant) -> Result<Option<ChangeSet>, TextError> {
        let count = self.document.selection().ranges().len();
        let lines: Vec<&str> = text.lines().collect();
        if count > 1 && lines.len() == count {
            return self.replace_each(|index, _, _| lines[index].to_owned(), now);
        }
        self.replace_selection(text, now)
    }

    fn add_cursor(
        &mut self,
        position: Position,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let selection = self.document.selection();
        let mut ranges = selection.ranges().to_vec();
        let primary = selection.primary_index();
        match ranges
            .iter()
            .position(|range| range.is_empty() && range.head == position)
        {
            Some(_) if ranges.len() == 1 => Ok(None),
            Some(index) => {
                ranges.remove(index);
                let primary = match index.cmp(&primary) {
                    std::cmp::Ordering::Less => primary - 1,
                    std::cmp::Ordering::Equal => ranges.len() - 1,
                    std::cmp::Ordering::Greater => primary,
                };
                self.select(Selection::new(ranges, primary), now)
            }
            None => {
                ranges.push(SelectionRange::cursor(position));
                let primary = ranges.len() - 1;
                self.select(Selection::new(ranges, primary), now)
            }
        }
    }

    fn add_cursor_line(
        &mut self,
        down: bool,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let buffer = self.document.buffer();
        let ranges = self.document.selection().ranges();
        let edge = if down { ranges.last() } else { ranges.first() };
        let Some(edge) = edge else {
            return Ok(None);
        };
        let place = buffer.line_column(edge.head, Unit::Char);
        let line = if down {
            place.line + 1
        } else if let Some(line) = place.line.checked_sub(1) {
            line
        } else {
            return Ok(None);
        };
        if line >= buffer.line_count() {
            return Ok(None);
        }
        let column = place.column;
        let position = buffer.position(LineColumn { line, column }, Unit::Char);
        self.add_cursor(position, now)
    }

    fn word_range(&self, at: Position) -> Option<SelectionRange> {
        let buffer = self.document.buffer();
        let word = boundary::word_at(buffer, at);
        buffer
            .slice(word.clone())
            .chars()
            .any(char::is_alphanumeric)
            .then(|| SelectionRange::new(word.start, word.end))
    }

    fn places_of(&self, range: SelectionRange, whole_word: bool) -> Vec<Range<usize>> {
        let text = self.document.buffer().to_string();
        let query = Query {
            text: text[range.start().offset()..range.end().offset()].to_owned(),
            match_case: true,
            whole_word,
            pattern: false,
        };
        match Search::new(&query) {
            Ok(Some(search)) => search.find(&text),
            _ => Vec::new(),
        }
    }

    fn select_next(
        &mut self,
        word_search: bool,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let selection = self.document.selection().clone();
        let primary = selection.primary();
        let mut ranges = selection.ranges().to_vec();
        if primary.is_empty() {
            let Some(word) = self.word_range(primary.head) else {
                return Ok(None);
            };
            ranges[selection.primary_index()] = word;
            self.word_search = true;
            return self.select(Selection::new(ranges, selection.primary_index()), now);
        }
        self.word_search = word_search;
        let places = self.places_of(primary, word_search);
        let free = |place: &&Range<usize>| {
            !ranges.iter().any(|taken| {
                place.start < taken.end().offset() && taken.start().offset() < place.end
            })
        };
        let after = primary.end().offset();
        let Some(next) = places
            .iter()
            .filter(|place| place.start >= after)
            .find(free)
            .or_else(|| places.iter().find(free))
            .cloned()
        else {
            return Ok(None);
        };
        ranges.push(SelectionRange::new(
            Position::new(next.start),
            Position::new(next.end),
        ));
        let primary = ranges.len() - 1;
        self.select(Selection::new(ranges, primary), now)
    }

    fn select_every(
        &mut self,
        word_search: bool,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let mut primary = self.document.selection().primary();
        let mut whole_word = word_search;
        if primary.is_empty() {
            let Some(word) = self.word_range(primary.head) else {
                return Ok(None);
            };
            primary = word;
            whole_word = true;
        }
        let ranges: Vec<SelectionRange> = self
            .places_of(primary, whole_word)
            .into_iter()
            .map(|place| SelectionRange::new(Position::new(place.start), Position::new(place.end)))
            .collect();
        let Some(index) = ranges
            .iter()
            .position(|range| range.start() == primary.start())
        else {
            return Ok(None);
        };
        self.word_search = whole_word;
        self.select(Selection::new(ranges, index), now)
    }

    fn move_each(
        &mut self,
        heads: &[Position],
        extend: bool,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let selection = self.document.selection();
        let ranges = selection
            .ranges()
            .iter()
            .zip(heads)
            .map(|(range, head)| {
                SelectionRange::new(if extend { range.anchor } else { *head }, *head)
            })
            .collect();
        self.select(Selection::new(ranges, selection.primary_index()), now)
    }

    fn replace(
        &mut self,
        range: std::ops::Range<Position>,
        text: &str,
        select: std::ops::Range<usize>,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let len = self.document.buffer().len();
        let start = range.start.offset().min(len);
        let end = range.end.offset().clamp(start, len);
        let changes = ChangeSet::new(
            len,
            [Edit::replace(
                Position::new(start)..Position::new(end),
                text,
            )],
        )?;
        let selection = SelectionRange::new(
            Position::new(start + select.start.min(text.len())),
            Position::new(start + select.end.min(text.len())),
        );
        self.change(
            Transaction::new(changes).with_selection(Selection::single(selection)),
            now,
        )
    }

    fn replace_all(
        &mut self,
        replacements: Vec<(std::ops::Range<Position>, String)>,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        if replacements.is_empty() {
            return Ok(None);
        }
        let edits = replacements
            .into_iter()
            .map(|(range, text)| Edit::replace(range, text));
        let changes = ChangeSet::new(self.document.buffer().len(), edits)?;
        self.change(Transaction::new(changes), now)
    }

    fn delete(&mut self, deletion: Deletion, now: Instant) -> Result<Option<ChangeSet>, TextError> {
        let buffer = self.document.buffer();
        let mut edits: Vec<Edit> = Vec::new();
        for range in self.document.selection().ranges() {
            let target = if range.is_empty() {
                deletion_range(buffer, range.head, deletion)
            } else {
                range.start()..range.end()
            };
            if target.is_empty() {
                continue;
            }
            match edits.last_mut() {
                Some(last) if target.start <= last.range.end => {
                    last.range.end = last.range.end.max(target.end);
                }
                _ => edits.push(Edit::delete(target)),
            }
        }
        if edits.is_empty() {
            return Ok(None);
        }
        let changes = ChangeSet::new(buffer.len(), edits)?;
        self.change(Transaction::new(changes), now)
    }

    fn change(
        &mut self,
        transaction: Transaction,
        now: Instant,
    ) -> Result<Option<ChangeSet>, TextError> {
        let changes = transaction.changes.clone();
        self.document.apply(transaction, now)?;
        self.revision += 1;
        self.map_folds(&changes);
        self.reveal();
        Ok(Some(changes))
    }
}

fn line_start_from(buffer: &Buffer, position: Position) -> Position {
    let line = buffer.line_of(position);
    let start = buffer.line_start(line);
    if start == position {
        start
    } else {
        buffer.line_start(line + 1)
    }
}

fn deletion_range(
    buffer: &Buffer,
    head: Position,
    deletion: Deletion,
) -> std::ops::Range<Position> {
    match deletion {
        Deletion::Backward => boundary::previous_grapheme(buffer, head)..head,
        Deletion::Forward => head..boundary::next_grapheme(buffer, head),
        Deletion::WordBackward => boundary::previous_word(buffer, head)..head,
        Deletion::WordForward => head..boundary::next_word(buffer, head),
    }
}

fn moved(buffer: &Buffer, range: SelectionRange, motion: Motion, extend: bool) -> SelectionRange {
    if !extend && !range.is_empty() {
        match motion {
            Motion::Left => return SelectionRange::cursor(range.start()),
            Motion::Right => return SelectionRange::cursor(range.end()),
            _ => {}
        }
    }
    let head = range.head;
    let target = match motion {
        Motion::Left => boundary::previous_grapheme(buffer, head),
        Motion::Right => boundary::next_grapheme(buffer, head),
        Motion::WordLeft => boundary::previous_word(buffer, head),
        Motion::WordRight => boundary::next_word(buffer, head),
        Motion::LineStart => boundary::smart_line_start(buffer, head),
        Motion::LineEnd => buffer.line_end(buffer.line_of(head)),
        Motion::DocumentStart => Position::default(),
        Motion::DocumentEnd => Position::new(buffer.len()),
    };
    if extend {
        SelectionRange::new(range.anchor, target)
    } else {
        SelectionRange::cursor(target)
    }
}

#[cfg(test)]
mod tests {
    use presswork_text::Position;

    use super::*;
    use crate::action::{Deletion, Motion};

    fn editor(marked: &str) -> Editor {
        let mut text = String::new();
        let (mut anchor, mut head) = (None, None);
        for char in marked.chars() {
            match char {
                '|' => head = Some(text.len()),
                '[' => anchor = Some(text.len()),
                _ => text.push(char),
            }
        }
        let head = Position::new(head.unwrap());
        let anchor = anchor.map_or(head, Position::new);
        let mut editor = Editor::new(&text);
        perform(&mut editor, move_to(anchor, false));
        perform(&mut editor, move_to(head, true));
        editor
    }

    fn marked(editor: &Editor) -> String {
        let mut text = editor.document().buffer().to_string();
        let range = editor.document().selection().primary();
        let mut markers = vec![(range.head.offset(), '|')];
        if !range.is_empty() {
            markers.push((range.anchor.offset(), '['));
        }
        markers.sort_by_key(|(offset, _)| std::cmp::Reverse(*offset));
        for (offset, marker) in markers {
            text.insert(offset, marker);
        }
        text
    }

    fn move_to(position: Position, extend: bool) -> Action {
        Action::MoveTo {
            position,
            extend,
            goal: None,
        }
    }

    fn perform(editor: &mut Editor, action: Action) {
        editor.perform(action, Instant::now()).unwrap();
    }

    fn after(marked_text: &str, actions: &[Action]) -> String {
        let mut editor = editor(marked_text);
        for action in actions {
            perform(&mut editor, action.clone());
        }
        marked(&editor)
    }

    fn step(motion: Motion) -> Action {
        Action::Move {
            motion,
            extend: false,
        }
    }

    fn extend(motion: Motion) -> Action {
        Action::Move {
            motion,
            extend: true,
        }
    }

    fn insert(text: &str) -> Action {
        Action::Insert(text.to_owned())
    }

    fn applied(text: &str, changes: &ChangeSet) -> String {
        let mut buffer = Buffer::from(text);
        buffer.apply(changes).unwrap();
        buffer.to_string()
    }

    #[test]
    fn an_action_gives_the_changes_of_its_text() {
        let mut editor = editor("ab|cd");

        let typed = editor.perform(insert("X"), Instant::now()).unwrap();
        let moved = editor.perform(step(Motion::Left), Instant::now()).unwrap();
        let undone = editor.perform(Action::Undo, Instant::now()).unwrap();

        assert_eq!(applied("abcd", &typed.unwrap()), "abXcd");
        assert!(moved.is_none());
        assert_eq!(applied("abXcd", &undone.unwrap()), "abcd");
    }

    #[test]
    fn a_remote_change_moves_the_text_and_keeps_the_cursor_at_its_place() {
        let mut editor = editor("ab|cd");
        let revision = editor.revision();

        editor
            .apply_remote(&ChangeSet::new(4, [Edit::insert(Position::new(0), "Hi ")]).unwrap())
            .unwrap();

        assert_eq!(marked(&editor), "Hi ab|cd");
        assert!(editor.revision() > revision);
    }

    #[test]
    fn typing_inserts_at_the_cursor() {
        assert_eq!(after("ab|cd", &[insert("X")]), "abX|cd");
    }

    #[test]
    fn typing_replaces_the_selection() {
        assert_eq!(after("a[bc|d", &[insert("X")]), "aX|d");
    }

    #[test]
    fn backspace_deletes_one_grapheme_cluster() {
        assert_eq!(
            after("ae\u{301}|b", &[Action::Delete(Deletion::Backward)]),
            "a|b"
        );
    }

    #[test]
    fn backspace_at_the_start_of_a_line_joins_the_lines() {
        assert_eq!(
            after("ab\r\n|cd", &[Action::Delete(Deletion::Backward)]),
            "ab|cd"
        );
    }

    #[test]
    fn backspace_deletes_the_selection() {
        assert_eq!(
            after("a[bc|d", &[Action::Delete(Deletion::Backward)]),
            "a|d"
        );
    }

    #[test]
    fn delete_removes_the_next_grapheme_cluster() {
        assert_eq!(after("a|👨‍👩‍👧b", &[Action::Delete(Deletion::Forward)]), "a|b");
    }

    #[test]
    fn word_deletion_removes_text_to_the_word_boundary() {
        assert_eq!(
            after("hello world|", &[Action::Delete(Deletion::WordBackward)]),
            "hello |"
        );
        assert_eq!(
            after("|hello world", &[Action::Delete(Deletion::WordForward)]),
            "| world"
        );
    }

    #[test]
    fn left_and_right_move_over_grapheme_clusters() {
        assert_eq!(after("|e\u{301}x", &[step(Motion::Right)]), "e\u{301}|x");
        assert_eq!(after("e\u{301}|x", &[step(Motion::Left)]), "|e\u{301}x");
    }

    #[test]
    fn right_at_the_end_of_a_line_moves_over_the_line_break() {
        assert_eq!(after("ab|\r\ncd", &[step(Motion::Right)]), "ab\r\n|cd");
        assert_eq!(after("ab\r\n|cd", &[step(Motion::Left)]), "ab|\r\ncd");
    }

    #[test]
    fn left_and_right_with_a_selection_go_to_its_edges() {
        assert_eq!(after("a[bc|d", &[step(Motion::Left)]), "a|bcd");
        assert_eq!(after("a|bc[d", &[step(Motion::Right)]), "abc|d");
    }

    #[test]
    fn shift_extends_the_selection() {
        assert_eq!(
            after("a|bcd", &[extend(Motion::Right), extend(Motion::Right)]),
            "a[bc|d"
        );
    }

    #[test]
    fn word_motions_stop_at_the_starts_and_the_ends_of_words() {
        let right = [step(Motion::WordRight)];
        assert_eq!(after("|let x = 10;", &right), "let| x = 10;");
        assert_eq!(after("let| x = 10;", &right), "let x| = 10;");
        assert_eq!(after("let x| = 10;", &right), "let x =| 10;");
        assert_eq!(after("let x =| 10;", &right), "let x = 10|;");

        let left = [step(Motion::WordLeft)];
        assert_eq!(after("let x = 10;|", &left), "let x = 10|;");
        assert_eq!(after("let x = 10|;", &left), "let x = |10;");
        assert_eq!(after("let x = |10;", &left), "let x |= 10;");
    }

    #[test]
    fn word_motions_move_over_line_breaks() {
        assert_eq!(after("ab|\ncd", &[step(Motion::WordRight)]), "ab\n|cd");
        assert_eq!(after("ab\n|cd", &[step(Motion::WordLeft)]), "ab|\ncd");
    }

    #[test]
    fn home_goes_to_the_first_text_and_then_to_the_line_start() {
        assert_eq!(after("x\n  ab|c", &[step(Motion::LineStart)]), "x\n  |abc");
        assert_eq!(after("x\n  |abc", &[step(Motion::LineStart)]), "x\n|  abc");
    }

    #[test]
    fn end_goes_to_the_end_of_the_line_text() {
        assert_eq!(after("a|b\r\nc", &[step(Motion::LineEnd)]), "ab|\r\nc");
    }

    #[test]
    fn document_motions_go_to_the_start_and_the_end() {
        assert_eq!(
            after("a\nb|\nc", &[step(Motion::DocumentStart)]),
            "|a\nb\nc"
        );
        assert_eq!(after("a\nb|\nc", &[step(Motion::DocumentEnd)]), "a\nb\nc|");
    }

    #[test]
    fn select_all_selects_the_whole_text() {
        assert_eq!(after("ab|c", &[Action::SelectAll]), "[abc|");
    }

    #[test]
    fn select_word_selects_the_word_at_a_position() {
        assert_eq!(
            after("|hello world", &[Action::SelectWord(Position::new(8))]),
            "hello [world|"
        );
    }

    #[test]
    fn select_line_selects_a_line_with_its_line_break() {
        assert_eq!(
            after("|a\nbc\nd", &[Action::SelectLine(Position::new(3))]),
            "a\n[bc\n|d"
        );
    }

    #[test]
    fn newline_keeps_the_indentation() {
        assert_eq!(after("  ab|c", &[Action::Newline]), "  ab\n  |c");
    }

    #[test]
    fn newline_uses_the_line_breaks_of_the_text() {
        assert_eq!(after("a\r\nb|", &[Action::Newline]), "a\r\nb\r\n|");
    }

    #[test]
    fn undo_and_redo_restore_the_text() {
        assert_eq!(after("ab|", &[insert("c"), Action::Undo]), "ab|");
        assert_eq!(
            after("ab|", &[insert("c"), Action::Undo, Action::Redo]),
            "abc|"
        );
    }

    #[test]
    fn the_revision_changes_only_with_the_text() {
        let mut editor = editor("ab|");
        let start = editor.revision();

        perform(&mut editor, step(Motion::Left));
        assert_eq!(editor.revision(), start);

        perform(&mut editor, insert("x"));
        assert_eq!(editor.revision(), start + 1);

        perform(&mut editor, Action::Undo);
        assert_eq!(editor.revision(), start + 2);
    }

    #[test]
    fn move_to_keeps_the_goal_and_other_actions_clear_it() {
        let mut editor = editor("ab|");

        perform(
            &mut editor,
            Action::MoveTo {
                position: Position::new(1),
                extend: false,
                goal: Some(42.0),
            },
        );
        assert_eq!(editor.goal(), Some(42.0));

        perform(&mut editor, step(Motion::Right));
        assert_eq!(editor.goal(), None);
    }

    #[test]
    fn selected_text_gives_the_text_of_the_selection() {
        assert_eq!(editor("a[bc|d").selected_text(), "bc");
    }

    fn replace(start: usize, end: usize, text: &str, select: std::ops::Range<usize>) -> Action {
        Action::Replace {
            range: Position::new(start)..Position::new(end),
            text: text.to_owned(),
            select,
        }
    }

    #[test]
    fn replace_puts_new_text_in_a_range_and_selects_a_part_of_it() {
        assert_eq!(
            after("#te|", &[replace(1, 3, "rgb(r, g, b)", 4..5)]),
            "#rgb([r|, g, b)"
        );
    }

    #[test]
    fn replace_with_an_empty_selection_puts_the_cursor_in_the_new_text() {
        assert_eq!(after("#te|", &[replace(1, 3, "text()", 5..5)]), "#text(|)");
    }

    #[test]
    fn a_replacement_is_one_undo_step() {
        assert_eq!(
            after("#te|", &[replace(1, 3, "text()", 5..5), Action::Undo]),
            "#te|"
        );
    }

    fn editor_with(marked: &str) -> Editor {
        let mut text = String::new();
        let mut ranges = Vec::new();
        let mut anchor = None;
        for char in marked.chars() {
            match char {
                '[' => anchor = Some(text.len()),
                '|' => {
                    let head = Position::new(text.len());
                    let start = anchor.take().map_or(head, Position::new);
                    ranges.push(SelectionRange::new(start, head));
                }
                _ => text.push(char),
            }
        }
        let mut editor = Editor::new(&text);
        let primary = ranges.len() - 1;
        editor
            .select(Selection::new(ranges, primary), Instant::now())
            .unwrap();
        editor
    }

    fn all_marked(editor: &Editor) -> String {
        let mut text = editor.document().buffer().to_string();
        let mut markers = Vec::new();
        for range in editor.document().selection().ranges() {
            markers.push((range.head.offset(), '|'));
            if !range.is_empty() {
                markers.push((range.anchor.offset(), '['));
            }
        }
        markers.sort_by_key(|(offset, marker)| (std::cmp::Reverse(*offset), *marker == '|'));
        for (offset, marker) in markers {
            text.insert(offset, marker);
        }
        text
    }

    fn after_all(marked_text: &str, actions: &[Action]) -> String {
        let mut editor = editor_with(marked_text);
        for action in actions {
            perform(&mut editor, action.clone());
        }
        all_marked(&editor)
    }

    fn primary_of(marked_text: &str, actions: &[Action]) -> String {
        let mut editor = editor_with(marked_text);
        for action in actions {
            perform(&mut editor, action.clone());
        }
        marked(&editor)
    }

    #[test]
    fn a_new_cursor_joins_the_others_and_becomes_the_primary_cursor() {
        let add = Action::AddCursor(Position::new(5));

        assert_eq!(
            after_all("ab|c def", std::slice::from_ref(&add)),
            "ab|c d|ef"
        );
        assert_eq!(primary_of("ab|c def", &[add]), "abc d|ef");
    }

    #[test]
    fn a_new_cursor_on_a_cursor_removes_it() {
        assert_eq!(
            after_all("a|b|c", &[Action::AddCursor(Position::new(1))]),
            "ab|c"
        );
    }

    #[test]
    fn the_only_cursor_stays() {
        assert_eq!(
            after_all("a|bc", &[Action::AddCursor(Position::new(1))]),
            "a|bc"
        );
    }

    #[test]
    fn typing_puts_the_text_at_each_cursor() {
        assert_eq!(after_all("a|b|c", &[insert("x")]), "ax|bx|c");
    }

    #[test]
    fn a_new_line_keeps_the_indent_of_each_line() {
        assert_eq!(after_all("  a|\nb|", &[Action::Newline]), "  a\n  |\nb\n|");
    }

    #[test]
    fn undo_gives_back_all_cursors() {
        assert_eq!(after_all("a|b|c", &[insert("x"), Action::Undo]), "a|b|c");
    }

    #[test]
    fn a_cursor_line_goes_below_the_last_cursor_at_the_same_column() {
        let down = Action::AddCursorLine { down: true };

        assert_eq!(
            after_all("ab|c\ndef\ng", std::slice::from_ref(&down)),
            "ab|c\nde|f\ng"
        );
        assert_eq!(
            after_all("ab|c\ndef\ng", &[down.clone(), down]),
            "ab|c\nde|f\ng|"
        );
    }

    #[test]
    fn a_cursor_line_goes_above_the_first_cursor() {
        assert_eq!(
            after_all("abc\nde|f", &[Action::AddCursorLine { down: false }]),
            "ab|c\nde|f"
        );
    }

    #[test]
    fn no_cursor_line_goes_above_the_first_line() {
        assert_eq!(
            after_all("ab|c\ndef", &[Action::AddCursorLine { down: false }]),
            "ab|c\ndef"
        );
    }

    #[test]
    fn select_next_selects_the_word_and_then_the_next_word() {
        let text = "ca|t and cat and catalog";

        assert_eq!(
            after_all(text, &[Action::SelectNext]),
            "[cat| and cat and catalog"
        );
        assert_eq!(
            after_all(text, &[Action::SelectNext, Action::SelectNext]),
            "[cat| and [cat| and catalog"
        );
        assert_eq!(
            after_all(
                text,
                &[Action::SelectNext, Action::SelectNext, Action::SelectNext]
            ),
            "[cat| and [cat| and catalog"
        );
    }

    #[test]
    fn select_next_after_a_selection_also_finds_parts_of_words() {
        assert_eq!(
            after_all("[cat| and catalog", &[Action::SelectNext]),
            "[cat| and [cat|alog"
        );
    }

    #[test]
    fn select_next_goes_around_to_the_start() {
        assert_eq!(after_all("ab x [ab|", &[Action::SelectNext]), "[ab| x [ab|");
        assert_eq!(primary_of("ab x [ab|", &[Action::SelectNext]), "[ab| x ab");
    }

    #[test]
    fn select_every_selects_all_places_of_the_text() {
        assert_eq!(
            after_all("[ab| x ab y ab", &[Action::SelectEvery]),
            "[ab| x [ab| y [ab|"
        );
    }

    #[test]
    fn single_cursor_keeps_only_the_primary_range() {
        assert_eq!(after_all("a|b|c", &[Action::SingleCursor]), "ab|c");
    }

    #[test]
    fn move_each_moves_the_head_of_each_range() {
        let moves = Action::MoveEach {
            heads: vec![Position::new(0), Position::new(3)],
            extend: false,
        };

        assert_eq!(after_all("a|b|cd", &[moves]), "|abc|d");
    }

    #[test]
    fn a_paste_with_one_line_for_each_cursor_puts_one_line_at_each() {
        assert_eq!(
            after_all("a|\nb|", &[Action::Paste("x\ny".to_owned())]),
            "ax|\nby|"
        );
    }

    #[test]
    fn a_paste_with_other_lines_puts_all_text_at_each_cursor() {
        assert_eq!(
            after_all("a|\nb|", &[Action::Paste("x".to_owned())]),
            "ax|\nbx|"
        );
    }

    fn folded(marked_text: &str, hidden: std::ops::Range<usize>) -> Editor {
        let mut editor = editor(marked_text);
        perform(
            &mut editor,
            Action::Fold(Position::new(hidden.start)..Position::new(hidden.end)),
        );
        editor
    }

    fn fold_spans(editor: &Editor) -> Vec<(usize, usize)> {
        editor
            .folds()
            .iter()
            .map(|fold| (fold.start.offset(), fold.end.offset()))
            .collect()
    }

    #[test]
    fn a_fold_moves_a_hidden_cursor_to_the_end_of_its_header() {
        let editor = folded("head\nbo|dy\ntail", 5..10);

        assert_eq!(marked(&editor), "head|\nbody\ntail");
        assert_eq!(fold_spans(&editor), [(5, 10)]);
    }

    #[test]
    fn moving_right_from_the_header_goes_over_the_fold() {
        let mut editor = folded("head|\nbody\ntail", 5..10);

        perform(&mut editor, step(Motion::Right));

        assert_eq!(marked(&editor), "head\nbody\n|tail");
        assert_eq!(fold_spans(&editor), [(5, 10)]);
    }

    #[test]
    fn moving_left_after_the_fold_goes_back_to_the_header() {
        let mut editor = folded("head\nbody\n|tail", 5..10);

        perform(&mut editor, step(Motion::Left));

        assert_eq!(marked(&editor), "head|\nbody\ntail");
    }

    #[test]
    fn a_jump_into_a_fold_opens_it() {
        let mut editor = folded("head|\nbody\ntail", 5..10);

        perform(&mut editor, move_to(Position::new(7), false));

        assert_eq!(fold_spans(&editor), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn typing_before_a_fold_moves_it() {
        let mut editor = folded("head|\nbody\ntail", 5..10);

        perform(&mut editor, insert("!"));

        assert_eq!(fold_spans(&editor), [(6, 11)]);
    }

    #[test]
    fn a_remote_change_moves_a_fold() {
        let mut editor = folded("head|\nbody\ntail", 5..10);
        let changes = ChangeSet::new(14, [Edit::insert(Position::new(0), "ab")]).unwrap();

        editor.apply_remote(&changes).unwrap();

        assert_eq!(fold_spans(&editor), [(7, 12)]);
    }

    #[test]
    fn replacing_the_folded_text_removes_the_fold() {
        let mut editor = folded("head|\nbody\ntail", 5..10);

        perform(&mut editor, Action::SelectAll);
        perform(&mut editor, insert("x"));

        assert_eq!(fold_spans(&editor), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn unfold_shows_the_text_again() {
        let mut editor = folded("head|\nbody\ntail", 5..10);
        let revision = editor.fold_revision();

        perform(
            &mut editor,
            Action::Unfold(Position::new(5)..Position::new(10)),
        );

        assert_eq!(fold_spans(&editor), Vec::<(usize, usize)>::new());
        assert!(editor.fold_revision() > revision);
    }

    fn replace_all(ranges: &[(usize, usize, &str)]) -> Action {
        Action::ReplaceAll(
            ranges
                .iter()
                .map(|(start, end, text)| {
                    (
                        Position::new(*start)..Position::new(*end),
                        (*text).to_owned(),
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn replace_all_changes_each_range_and_keeps_the_cursor() {
        assert_eq!(
            after(
                "cat, |and cat",
                &[replace_all(&[(0, 3, "dog"), (9, 12, "dog")])]
            ),
            "dog, |and dog"
        );
    }

    #[test]
    fn replace_all_is_one_undo_step() {
        assert_eq!(
            after(
                "cat, |and cat",
                &[replace_all(&[(0, 3, "dog"), (9, 12, "dog")]), Action::Undo]
            ),
            "cat, |and cat"
        );
    }

    #[test]
    fn replace_all_without_ranges_changes_nothing() {
        let mut editor = editor("cat|");
        let revision = editor.revision();

        let changes = editor.perform(replace_all(&[]), Instant::now()).unwrap();

        assert_eq!(changes, None);
        assert_eq!(editor.revision(), revision);
    }
}
