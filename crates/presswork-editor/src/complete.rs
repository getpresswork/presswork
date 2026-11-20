use std::ops::Range;

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::{Point, Rectangle, Size};
use presswork_text::Position;

use crate::action::Action;

pub(crate) const ROWS: usize = 8;
pub(crate) const ROW_HEIGHT: f32 = 24.0;
pub(crate) const LIST_WIDTH: f32 = 380.0;
pub(crate) const LIST_PADDING: f32 = 4.0;
pub(crate) const LIST_GAP: f32 = 4.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub text: String,
    pub select: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionList {
    pub from: Position,
    pub items: Vec<CompletionItem>,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionEvent {
    Request,
    Next,
    Previous,
    Accept,
    Choose(usize),
    Close,
}

impl CompletionList {
    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + self.items.len() - 1) % self.items.len();
        }
    }

    #[must_use]
    pub fn apply(&self, index: usize, head: Position) -> Option<Action> {
        let item = self.items.get(index)?;
        Some(Action::Replace {
            range: self.from..head,
            text: item.text.clone(),
            select: item.select.clone(),
        })
    }

    pub(crate) fn visible(&self) -> Range<usize> {
        let rows = self.items.len().min(ROWS);
        let first = (self.selected + 1).saturating_sub(rows);
        first..first + rows
    }
}

pub(crate) fn completion_key(
    key: &Key,
    modifiers: Modifiers,
    open: bool,
) -> Option<CompletionEvent> {
    match key {
        Key::Named(Named::Space) if modifiers.control() => Some(CompletionEvent::Request),
        _ if !open => None,
        Key::Named(Named::ArrowDown) => Some(CompletionEvent::Next),
        Key::Named(Named::ArrowUp) => Some(CompletionEvent::Previous),
        Key::Named(Named::Enter | Named::Tab) => Some(CompletionEvent::Accept),
        Key::Named(Named::Escape) => Some(CompletionEvent::Close),
        _ => None,
    }
}

pub(crate) fn rows_height(rows: usize) -> f32 {
    f32::from(u16::try_from(rows).unwrap_or(u16::MAX)) * ROW_HEIGHT
}

pub(crate) fn list_bounds(anchor: Rectangle, rows: usize, bounds: Rectangle) -> Rectangle {
    let size = Size::new(LIST_WIDTH, rows_height(rows) + 2.0 * LIST_PADDING);
    let below = anchor.y + anchor.height + LIST_GAP;
    let y = if below + size.height <= bounds.y + bounds.height {
        below
    } else {
        anchor.y - LIST_GAP - size.height
    };
    let x = anchor
        .x
        .min(bounds.x + bounds.width - size.width)
        .max(bounds.x);
    Rectangle::new(Point::new(x, y), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_owned(),
            detail: None,
            text: label.to_owned(),
            select: label.len()..label.len(),
        }
    }

    fn list(count: usize) -> CompletionList {
        CompletionList {
            from: Position::new(1),
            items: (0..count)
                .map(|index| item(&format!("item{index}")))
                .collect(),
            selected: 0,
        }
    }

    #[test]
    fn next_and_previous_go_around_the_list() {
        let mut list = list(3);

        list.previous();
        assert_eq!(list.selected, 2);
        list.next();
        assert_eq!(list.selected, 0);
        list.next();
        assert_eq!(list.selected, 1);
    }

    #[test]
    fn apply_replaces_the_typed_text_with_the_item() {
        let list = CompletionList {
            from: Position::new(1),
            items: vec![CompletionItem {
                label: "text".to_owned(),
                detail: None,
                text: "text()".to_owned(),
                select: 5..5,
            }],
            selected: 0,
        };

        assert_eq!(
            list.apply(0, Position::new(3)),
            Some(Action::Replace {
                range: Position::new(1)..Position::new(3),
                text: "text()".to_owned(),
                select: 5..5,
            })
        );
        assert_eq!(list.apply(1, Position::new(3)), None);
    }

    #[test]
    fn the_visible_rows_follow_the_selected_item() {
        let mut list = list(20);
        assert_eq!(list.visible(), 0..ROWS);

        list.selected = 12;
        assert_eq!(list.visible(), 5..13);

        let short = super::tests::list(3);
        assert_eq!(short.visible(), 0..3);
    }

    #[test]
    fn keys_control_an_open_list() {
        let none = Modifiers::empty();

        assert_eq!(
            completion_key(&Key::Named(Named::ArrowDown), none, true),
            Some(CompletionEvent::Next)
        );
        assert_eq!(
            completion_key(&Key::Named(Named::ArrowUp), none, true),
            Some(CompletionEvent::Previous)
        );
        assert_eq!(
            completion_key(&Key::Named(Named::Enter), none, true),
            Some(CompletionEvent::Accept)
        );
        assert_eq!(
            completion_key(&Key::Named(Named::Tab), none, true),
            Some(CompletionEvent::Accept)
        );
        assert_eq!(
            completion_key(&Key::Named(Named::Escape), none, true),
            Some(CompletionEvent::Close)
        );
    }

    #[test]
    fn without_a_list_only_control_space_asks_for_completions() {
        assert_eq!(
            completion_key(&Key::Named(Named::ArrowDown), Modifiers::empty(), false),
            None
        );
        assert_eq!(
            completion_key(&Key::Named(Named::Space), Modifiers::CTRL, false),
            Some(CompletionEvent::Request)
        );
        assert_eq!(
            completion_key(&Key::Named(Named::Space), Modifiers::empty(), false),
            None
        );
    }

    fn screen() -> Rectangle {
        Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0))
    }

    fn caret_at(x: f32, y: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(1.0, 22.0))
    }

    fn height(rows: usize) -> f32 {
        f32::from(u16::try_from(rows).unwrap()) * ROW_HEIGHT + 2.0 * LIST_PADDING
    }

    #[test]
    fn the_list_goes_below_its_line() {
        let placed = list_bounds(caret_at(100.0, 100.0), 3, screen());

        assert_eq!(
            placed,
            Rectangle::new(
                Point::new(100.0, 100.0 + 22.0 + LIST_GAP),
                Size::new(LIST_WIDTH, height(3))
            )
        );
    }

    #[test]
    fn a_list_without_room_below_goes_above_its_line() {
        let placed = list_bounds(caret_at(100.0, 550.0), 8, screen());

        assert!(
            (placed.y - (550.0 - LIST_GAP - height(8))).abs() < 0.01,
            "{placed:?}"
        );
    }

    #[test]
    fn the_list_stays_inside_the_right_edge() {
        let placed = list_bounds(caret_at(700.0, 100.0), 3, screen());

        assert!((placed.x - (800.0 - LIST_WIDTH)).abs() < 0.01, "{placed:?}");
    }
}
