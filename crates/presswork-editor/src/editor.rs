use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use presswork_text::{
    Assoc, Buffer, ChangeSet, Document, Edit, Position, Selection, SelectionRange, TextError,
    Transaction,
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
}

impl Editor {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            document: Document::new(text),
            revision: 0,
            goal: None,
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

    pub fn perform(&mut self, action: Action, now: Instant) -> Result<(), TextError> {
        self.goal = None;
        let buffer = self.document.buffer();
        match action {
            Action::Move { motion, extend } => {
                let selection = self.document.selection();
                let ranges = selection
                    .ranges()
                    .iter()
                    .map(|range| moved(buffer, *range, motion, extend))
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
                Ok(())
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
            Action::Newline => {
                let head = self.document.selection().primary().head;
                let text = format!(
                    "{}{}",
                    boundary::line_break(buffer),
                    boundary::indentation(buffer, head)
                );
                self.replace_selection(&text, now)
            }
            Action::Delete(deletion) => self.delete(deletion, now),
            Action::Undo => {
                if self.document.undo()?.is_some() {
                    self.revision += 1;
                }
                Ok(())
            }
            Action::Redo => {
                if self.document.redo()?.is_some() {
                    self.revision += 1;
                }
                Ok(())
            }
        }
    }

    fn select(&mut self, selection: Selection, now: Instant) -> Result<(), TextError> {
        let len = self.document.buffer().len();
        let transaction = Transaction::new(ChangeSet::empty(len)).with_selection(selection);
        self.document.apply(transaction, now)
    }

    fn select_range(
        &mut self,
        range: std::ops::Range<Position>,
        now: Instant,
    ) -> Result<(), TextError> {
        self.select(
            Selection::single(SelectionRange::new(range.start, range.end)),
            now,
        )
    }

    fn replace_selection(&mut self, text: &str, now: Instant) -> Result<(), TextError> {
        let selection = self.document.selection().clone();
        let edits = selection
            .ranges()
            .iter()
            .map(|range| Edit::replace(range.start()..range.end(), text));
        let changes = ChangeSet::new(self.document.buffer().len(), edits)?;
        let cursors = selection
            .ranges()
            .iter()
            .map(|range| SelectionRange::cursor(changes.map_position(range.end(), Assoc::After)))
            .collect();
        let selection = Selection::new(cursors, selection.primary_index());
        self.change(Transaction::new(changes).with_selection(selection), now)
    }

    fn delete(&mut self, deletion: Deletion, now: Instant) -> Result<(), TextError> {
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
            return Ok(());
        }
        let changes = ChangeSet::new(buffer.len(), edits)?;
        self.change(Transaction::new(changes), now)
    }

    fn change(&mut self, transaction: Transaction, now: Instant) -> Result<(), TextError> {
        self.document.apply(transaction, now)?;
        self.revision += 1;
        Ok(())
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
}
