use std::time::{Duration, Instant};

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::input_method::{self, InputMethod};
use iced::advanced::layout;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Renderer as _, Text};
use iced::advanced::widget::{self, Tree, Widget, operation};
use iced::advanced::{Clipboard, Shell, clipboard, mouse};
use iced::alignment::Vertical;
use iced::keyboard::{self, Key, key::Named, key::Physical};
use iced::{
    Border, Element, Event, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, window,
};
use presswork_text::{Position, Unit};
use presswork_ui::color::Colors;
use presswork_ui::font;

use crate::action::{Action, Deletion, Motion};
use crate::boundary;
use crate::complete::{self, CompletionEvent, CompletionList};
use crate::editor::Editor;
use crate::highlight::Highlighter;
use crate::layout::{LINE_HEIGHT, Layout, TEXT_SIZE, row_height};

const PADDING_TOP: f32 = 16.0;
const PADDING_BOTTOM: f32 = 120.0;
const PADDING_RIGHT: f32 = 24.0;
const GUTTER_LEFT: f32 = 16.0;
const GUTTER_GAP: f32 = 16.0;
const MIN_DIGITS: usize = 3;
const MIN_TEXT_WIDTH: f32 = 80.0;
const CARET_WIDTH: f32 = 2.0;
const BLINK: Duration = Duration::from_millis(500);
const WHEEL_LINES: f32 = 3.0;
const MARK_THICKNESS: f32 = 1.5;
const MARK_OFFSET: f32 = 4.0;
const EMPTY_MARK_WIDTH: f32 = 6.0;
const TAB: &str = "  ";
const HOVER_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_WIDTH: f32 = 420.0;
const TOOLTIP_PADDING: f32 = 8.0;
const TOOLTIP_GAP: f32 = 4.0;
const TOOLTIP_TEXT_SIZE: f32 = 13.0;

pub struct EditorView<'a, Message> {
    editor: &'a Editor,
    colors: &'a Colors,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
    autofocus: bool,
    marks: Vec<Mark>,
    id: Option<widget::Id>,
    on_hover: Option<Box<dyn Fn(Option<Position>) -> Message + 'a>>,
    tooltip: Option<&'a Tooltip>,
    on_complete: Option<Box<dyn Fn(CompletionEvent) -> Message + 'a>>,
    completions: Option<&'a CompletionList>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tooltip {
    pub position: Position,
    pub text: String,
    pub code: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub range: std::ops::Range<Position>,
    pub kind: MarkKind,
}

pub fn view<'a, Message>(
    editor: &'a Editor,
    colors: &'a Colors,
    on_action: impl Fn(Action) -> Message + 'a,
) -> EditorView<'a, Message> {
    EditorView {
        editor,
        colors,
        on_action: Box::new(on_action),
        autofocus: false,
        marks: Vec::new(),
        id: None,
        on_hover: None,
        tooltip: None,
        on_complete: None,
        completions: None,
    }
}

impl<'a, Message> EditorView<'a, Message> {
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    #[must_use]
    pub fn marks(mut self, marks: Vec<Mark>) -> Self {
        self.marks = marks;
        self
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[must_use]
    pub fn on_hover(mut self, on_hover: impl Fn(Option<Position>) -> Message + 'a) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: Option<&'a Tooltip>) -> Self {
        self.tooltip = tooltip;
        self
    }

    #[must_use]
    pub fn on_complete(mut self, on_complete: impl Fn(CompletionEvent) -> Message + 'a) -> Self {
        self.on_complete = Some(Box::new(on_complete));
        self
    }

    #[must_use]
    pub fn completions(mut self, completions: Option<&'a CompletionList>) -> Self {
        self.completions = completions;
        self
    }

    fn draw_marks(
        &self,
        state: &State,
        renderer: &mut Renderer,
        origin: Point,
        visible: &std::ops::Range<f32>,
    ) {
        for mark in &self.marks {
            let color = match mark.kind {
                MarkKind::Error => self.colors.editor.error,
                MarkKind::Warning => self.colors.warning,
            };
            let rows = if mark.range.start < mark.range.end {
                state.layout.selection(mark.range.clone())
            } else {
                let caret = state.layout.caret(mark.range.start);
                vec![Rectangle::new(
                    caret.position(),
                    Size::new(EMPTY_MARK_WIDTH, caret.height),
                )]
            };
            for row in rows {
                if row.y + row.height < visible.start || row.y > visible.end {
                    continue;
                }
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                origin.x + row.x,
                                origin.y + row.y + row.height - MARK_OFFSET,
                            ),
                            Size::new(row.width, MARK_THICKNESS),
                        ),
                        ..Quad::default()
                    },
                    color,
                );
            }
        }
    }

    fn snap(&self, position: Position) -> Position {
        let buffer = self.editor.document().buffer();
        buffer.position(buffer.line_column(position, Unit::Byte), Unit::Byte)
    }

    fn press(
        &self,
        state: &mut State,
        bounds: Rectangle,
        point: Option<Point>,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(point) = point else {
            if state.focus.is_some() {
                state.focus = None;
                state.dragging = false;
                shell.request_redraw();
            }
            return;
        };
        let click = mouse::Click::new(point, mouse::Button::Left, state.last_click);
        let position = self.snap(state.layout.hit(state.to_content(bounds, point)));
        let action = match click.kind() {
            mouse::click::Kind::Single => Action::MoveTo {
                position,
                extend: state.modifiers.shift(),
                goal: None,
            },
            mouse::click::Kind::Double => Action::SelectWord(position),
            mouse::click::Kind::Triple => Action::SelectLine(position),
        };
        state.dragging = click.kind() == mouse::click::Kind::Single;
        state.last_click = Some(click);
        state.focus = Some(Focus::new());
        shell.publish((self.on_action)(action));
        shell.capture_event();
    }

    fn list_area(
        &self,
        state: &State,
        bounds: Rectangle,
    ) -> Option<(Rectangle, std::ops::Range<usize>)> {
        let list = self.completions.filter(|list| !list.items.is_empty())?;
        let origin = state.origin(bounds);
        let caret = state.layout.caret(list.from);
        let anchor = Rectangle::new(
            Point::new(origin.x + caret.x, origin.y + caret.y),
            Size::new(CARET_WIDTH, caret.height),
        );
        let visible = list.visible();
        Some((
            complete::list_bounds(anchor, visible.len(), bounds),
            visible,
        ))
    }

    fn choose(
        &self,
        state: &State,
        bounds: Rectangle,
        point: Point,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let Some((area, visible)) = self.list_area(state, bounds) else {
            return false;
        };
        if !area.contains(point) {
            return false;
        }
        let chosen = visible
            .enumerate()
            .find(|(row, _)| {
                point.y < area.y + complete::LIST_PADDING + complete::rows_height(row + 1)
            })
            .map(|(_, index)| index);
        if let (Some(index), Some(on_complete)) = (chosen, &self.on_complete) {
            shell.publish(on_complete(CompletionEvent::Choose(index)));
        }
        shell.capture_event();
        true
    }

    fn draw_completions(&self, state: &State, renderer: &mut Renderer, bounds: Rectangle) {
        let (Some(list), Some((area, visible))) = (self.completions, self.list_area(state, bounds))
        else {
            return;
        };
        let colors = self.colors;
        let detail = list
            .items
            .get(list.selected)
            .and_then(|item| item.detail.as_deref())
            .map(|detail| {
                Paragraph::with_text(small_text(
                    detail,
                    font::UI,
                    complete::LIST_WIDTH - 2.0 * TOOLTIP_PADDING,
                    text::Wrapping::WordOrGlyph,
                ))
            });
        renderer.with_layer(bounds, |renderer| {
            fill_box(renderer, area, colors);
            for (row, index) in visible.enumerate() {
                let Some(item) = list.items.get(index) else {
                    continue;
                };
                let top = area.y + complete::LIST_PADDING + complete::rows_height(row);
                let row_bounds = Rectangle::new(
                    Point::new(area.x + complete::LIST_PADDING, top),
                    Size::new(
                        area.width - 2.0 * complete::LIST_PADDING,
                        complete::ROW_HEIGHT,
                    ),
                );
                if index == list.selected {
                    renderer.fill_quad(
                        Quad {
                            bounds: row_bounds,
                            border: Border {
                                radius: 4.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        colors.accent_soft,
                    );
                }
                let inset = (complete::ROW_HEIGHT - TOOLTIP_TEXT_SIZE * LINE_HEIGHT) / 2.0;
                renderer.fill_text(
                    small_text(
                        item.label.clone(),
                        font::MONO,
                        row_bounds.width - 2.0 * TOOLTIP_PADDING,
                        text::Wrapping::None,
                    ),
                    Point::new(row_bounds.x + TOOLTIP_PADDING, top + inset),
                    colors.ink,
                    row_bounds,
                );
            }
            if let Some(paragraph) = &detail {
                let text_size = paragraph.min_bounds();
                let size = Size::new(
                    complete::LIST_WIDTH,
                    text_size.height + 2.0 * TOOLTIP_PADDING,
                );
                let below = area.y + area.height + complete::LIST_GAP;
                let y = if below + size.height <= bounds.y + bounds.height {
                    below
                } else {
                    area.y - complete::LIST_GAP - size.height
                };
                let detail_box = Rectangle::new(Point::new(area.x, y), size);
                fill_box(renderer, detail_box, colors);
                renderer.fill_paragraph(
                    paragraph,
                    Point::new(detail_box.x + TOOLTIP_PADDING, y + TOOLTIP_PADDING),
                    colors.ink_muted,
                    bounds,
                );
            }
        });
    }

    fn text_at(&self, state: &State, bounds: Rectangle, point: Point) -> Option<Position> {
        if !bounds.contains(point) || point.x < bounds.x + state.gutter {
            return None;
        }
        let content = state.to_content(bounds, point);
        let position = self.snap(state.layout.hit(content));
        let caret = state.layout.caret(position);
        let near = content.x <= caret.x + row_height()
            && content.y <= caret.y + caret.height + row_height();
        if !near {
            return None;
        }
        if content.x < caret.x {
            Some(boundary::previous_grapheme(
                self.editor.document().buffer(),
                position,
            ))
        } else {
            Some(position)
        }
    }

    fn hover(
        &self,
        state: &mut State,
        bounds: Rectangle,
        point: Option<Point>,
        shell: &mut Shell<'_, Message>,
    ) {
        if self.on_hover.is_none() {
            return;
        }
        let target = point.and_then(|point| self.text_at(state, bounds, point));
        if state.hover.map(|hover| hover.position) == target {
            return;
        }
        self.end_hover(state, shell);
        if let Some(position) = target {
            let since = Instant::now();
            state.hover = Some(Hover {
                position,
                since,
                reported: false,
            });
            shell.request_redraw_at(since + HOVER_DELAY);
        }
    }

    fn end_hover(&self, state: &mut State, shell: &mut Shell<'_, Message>) {
        if let Some(hover) = state.hover.take()
            && hover.reported
            && let Some(on_hover) = &self.on_hover
        {
            shell.publish(on_hover(None));
        }
    }

    fn report_hover(&self, state: &mut State, now: Instant, shell: &mut Shell<'_, Message>) {
        let Some(on_hover) = &self.on_hover else {
            return;
        };
        if let Some(hover) = &mut state.hover
            && !hover.reported
        {
            if now.saturating_duration_since(hover.since) >= HOVER_DELAY {
                hover.reported = true;
                shell.publish(on_hover(Some(hover.position)));
            } else {
                shell.request_redraw_at(hover.since + HOVER_DELAY);
            }
        }
    }

    fn draw_tooltip(
        &self,
        state: &State,
        renderer: &mut Renderer,
        bounds: Rectangle,
        tooltip: &Tooltip,
    ) {
        let origin = state.origin(bounds);
        let caret = state.layout.caret(tooltip.position);
        let anchor = Rectangle::new(
            Point::new(origin.x + caret.x, origin.y + caret.y),
            Size::new(CARET_WIDTH, caret.height),
        );
        if !bounds.intersects(&anchor) {
            return;
        }
        let paragraph = Paragraph::with_text(small_text(
            tooltip.text.as_str(),
            if tooltip.code { font::MONO } else { font::UI },
            TOOLTIP_WIDTH - 2.0 * TOOLTIP_PADDING,
            text::Wrapping::WordOrGlyph,
        ));
        let text_size = paragraph.min_bounds();
        let size = Size::new(
            text_size.width + 2.0 * TOOLTIP_PADDING,
            text_size.height + 2.0 * TOOLTIP_PADDING,
        );
        let placed = tooltip_bounds(anchor, size, bounds);
        renderer.with_layer(bounds, |renderer| {
            fill_box(renderer, placed, self.colors);
            renderer.fill_paragraph(
                &paragraph,
                Point::new(placed.x + TOOLTIP_PADDING, placed.y + TOOLTIP_PADDING),
                self.colors.ink,
                bounds,
            );
        });
    }

    fn input_method(&self, state: &State, bounds: Rectangle) -> InputMethod<&'static str> {
        let caret = state
            .layout
            .caret(self.editor.document().selection().primary().head);
        let origin = state.origin(bounds);
        InputMethod::Enabled {
            cursor: Rectangle::new(
                Point::new(origin.x + caret.x, origin.y + caret.y),
                Size::new(1.0, caret.height),
            ),
            purpose: input_method::Purpose::Normal,
            preedit: None,
        }
    }

    fn run(
        &self,
        command: Command,
        state: &mut State,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let head = self.editor.document().selection().primary().head;
        let action = match command {
            Command::Act(action) => action,
            Command::Vertical { rows, extend } => {
                let delta = rows * row_height();
                let (position, goal) = state.layout.vertical(head, self.editor.goal(), delta);
                Action::MoveTo {
                    position: self.snap(position),
                    extend,
                    goal: Some(goal),
                }
            }
            Command::Page { down, extend } => {
                let page = (state.viewport.height - row_height()).max(row_height());
                let delta = if down { page } else { -page };
                let (position, goal) = state.layout.vertical(head, self.editor.goal(), delta);
                state.scroll_by(delta);
                Action::MoveTo {
                    position: self.snap(position),
                    extend,
                    goal: Some(goal),
                }
            }
            Command::Copy | Command::Cut => {
                let text = self.editor.selected_text();
                if text.is_empty() {
                    return;
                }
                clipboard.write(clipboard::Kind::Standard, text);
                if command == Command::Copy {
                    return;
                }
                Action::Delete(Deletion::Backward)
            }
            Command::Paste => match clipboard.read(clipboard::Kind::Standard) {
                Some(text) => Action::Insert(text),
                None => return,
            },
        };
        shell.publish((self.on_action)(action));
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Command {
    Act(Action),
    Vertical { rows: f32, extend: bool },
    Page { down: bool, extend: bool },
    Copy,
    Cut,
    Paste,
}

fn command(
    key: &Key,
    physical_key: Physical,
    modifiers: keyboard::Modifiers,
    text: Option<&str>,
) -> Option<Command> {
    let extend = modifiers.shift();
    if modifiers.command() {
        let shortcut = match key.to_latin(physical_key) {
            Some('a') => Some(Command::Act(Action::SelectAll)),
            Some('c') => Some(Command::Copy),
            Some('x') => Some(Command::Cut),
            Some('v') if !modifiers.alt() => Some(Command::Paste),
            Some('z') if extend => Some(Command::Act(Action::Redo)),
            Some('z') => Some(Command::Act(Action::Undo)),
            Some('y') => Some(Command::Act(Action::Redo)),
            _ => None,
        };
        if shortcut.is_some() {
            return shortcut;
        }
    }
    let jump = modifiers.jump();
    let line = modifiers.macos_command();
    let step = |motion| Some(Command::Act(Action::Move { motion, extend }));
    let deletion = |word, single| {
        Some(Command::Act(Action::Delete(if jump {
            word
        } else {
            single
        })))
    };
    match key {
        Key::Named(Named::Enter) => Some(Command::Act(Action::Newline)),
        Key::Named(Named::Tab) => Some(Command::Act(Action::Insert(TAB.to_owned()))),
        Key::Named(Named::Backspace) => deletion(Deletion::WordBackward, Deletion::Backward),
        Key::Named(Named::Delete) => deletion(Deletion::WordForward, Deletion::Forward),
        Key::Named(Named::ArrowLeft) if line => step(Motion::LineStart),
        Key::Named(Named::ArrowRight) if line => step(Motion::LineEnd),
        Key::Named(Named::ArrowLeft) => step(if jump { Motion::WordLeft } else { Motion::Left }),
        Key::Named(Named::ArrowRight) => step(if jump {
            Motion::WordRight
        } else {
            Motion::Right
        }),
        Key::Named(Named::ArrowUp) if line => step(Motion::DocumentStart),
        Key::Named(Named::ArrowDown) if line => step(Motion::DocumentEnd),
        Key::Named(Named::ArrowUp) => Some(Command::Vertical { rows: -1.0, extend }),
        Key::Named(Named::ArrowDown) => Some(Command::Vertical { rows: 1.0, extend }),
        Key::Named(Named::Home) if modifiers.command() => step(Motion::DocumentStart),
        Key::Named(Named::End) if modifiers.command() => step(Motion::DocumentEnd),
        Key::Named(Named::Home) => step(Motion::LineStart),
        Key::Named(Named::End) => step(Motion::LineEnd),
        Key::Named(Named::PageUp) => Some(Command::Page {
            down: false,
            extend,
        }),
        Key::Named(Named::PageDown) => Some(Command::Page { down: true, extend }),
        _ => {
            let text = text?;
            let shortcut_only = modifiers.command() && !modifiers.alt();
            if shortcut_only || text.is_empty() || text.chars().any(char::is_control) {
                return None;
            }
            Some(Command::Act(Action::Insert(text.to_owned())))
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Hover {
    position: Position,
    since: Instant,
    reported: bool,
}

fn tooltip_bounds(anchor: Rectangle, size: Size, bounds: Rectangle) -> Rectangle {
    let above = anchor.y - TOOLTIP_GAP - size.height;
    let y = if above >= bounds.y {
        above
    } else {
        anchor.y + anchor.height + TOOLTIP_GAP
    };
    let x = anchor
        .x
        .min(bounds.x + bounds.width - size.width)
        .max(bounds.x);
    Rectangle::new(Point::new(x, y), size)
}

#[derive(Debug, Clone, Copy)]
struct Focus {
    since: Instant,
    now: Instant,
    window_focused: bool,
}

impl Focus {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            since: now,
            now,
            window_focused: true,
        }
    }

    fn caret_visible(&self) -> bool {
        let elapsed = self.now.saturating_duration_since(self.since).as_millis();
        self.window_focused && (elapsed / BLINK.as_millis()).is_multiple_of(2)
    }
}

#[derive(Debug, Default)]
struct State {
    layout: Layout,
    highlighter: Highlighter,
    scroll: f32,
    gutter: f32,
    gutter_digits: usize,
    viewport: Size,
    focus: Option<Focus>,
    last_click: Option<mouse::Click>,
    dragging: bool,
    modifiers: keyboard::Modifiers,
    revealed: Option<(u64, u64, Position)>,
    hover: Option<Hover>,
}

impl State {
    fn origin(&self, bounds: Rectangle) -> Point {
        Point::new(bounds.x + self.gutter, bounds.y + PADDING_TOP - self.scroll)
    }

    fn to_content(&self, bounds: Rectangle, point: Point) -> Point {
        let origin = self.origin(bounds);
        Point::new(point.x - origin.x, point.y - origin.y)
    }

    fn max_scroll(&self) -> f32 {
        (self.layout.height() + PADDING_TOP + PADDING_BOTTOM - self.viewport.height).max(0.0)
    }

    fn scroll_by(&mut self, delta: f32) {
        self.scroll = (self.scroll + delta).clamp(0.0, self.max_scroll());
    }

    fn update_gutter(&mut self, line_count: usize) {
        let digits = line_count.to_string().len().max(MIN_DIGITS);
        if digits != self.gutter_digits {
            let numbers = Paragraph::with_text(number_text("0".repeat(digits).as_str(), 0.0));
            self.gutter = GUTTER_LEFT + numbers.min_width() + GUTTER_GAP;
            self.gutter_digits = digits;
        }
    }

    fn reveal(&mut self, editor: &Editor) {
        let head = editor.document().selection().primary().head;
        let key = (editor.id(), editor.revision(), head);
        if self.revealed != Some(key) {
            self.revealed = Some(key);
            let caret = self.layout.caret(head);
            let top = caret.y + PADDING_TOP - row_height();
            let bottom = caret.y + caret.height + PADDING_TOP + row_height();
            if top < self.scroll {
                self.scroll = top;
            } else if bottom > self.scroll + self.viewport.height {
                self.scroll = bottom - self.viewport.height;
            }
        }
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::new());
    }

    fn unfocus(&mut self) {
        self.focus = None;
    }
}

fn small_text<Content>(
    content: Content,
    font: iced::Font,
    width: f32,
    wrapping: text::Wrapping,
) -> Text<Content> {
    Text {
        content,
        bounds: Size::new(width, f32::INFINITY),
        size: Pixels(TOOLTIP_TEXT_SIZE),
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping,
    }
}

fn fill_box(renderer: &mut Renderer, bounds: Rectangle, colors: &Colors) {
    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                color: colors.rule,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Quad::default()
        },
        colors.raised,
    );
}

fn number_text<Content>(content: Content, width: f32) -> Text<Content> {
    Text {
        content,
        bounds: Size::new(width, row_height()),
        size: Pixels(TEXT_SIZE),
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        font: font::MONO,
        align_x: text::Alignment::Right,
        align_y: Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    }
}

impl<Message> Widget<Message, Theme, Renderer> for EditorView<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            focus: self.autofocus.then(Focus::new),
            ..State::default()
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.width(Length::Fill).height(Length::Fill).max();
        state.viewport = size;
        state.update_gutter(self.editor.document().buffer().line_count());
        let width = (size.width - state.gutter - PADDING_RIGHT).max(MIN_TEXT_WIDTH);
        state
            .layout
            .update(self.editor, &mut state.highlighter, self.colors, width);
        state.reveal(self.editor);
        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let colors = self.colors;
        renderer.fill_quad(
            Quad {
                bounds,
                ..Quad::default()
            },
            colors.page,
        );
        renderer.with_layer(bounds, |renderer| {
            let origin = state.origin(bounds);
            let offset = Vector::new(origin.x, origin.y);
            let visible =
                (state.scroll - PADDING_TOP)..(state.scroll - PADDING_TOP + bounds.height);
            let selection = self.editor.document().selection();
            let current = state.layout.line_index(selection.primary().head);
            if let Some(line) = state.layout.lines().get(current) {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(bounds.x, origin.y + line.top),
                            Size::new(bounds.width, line.height),
                        ),
                        ..Quad::default()
                    },
                    colors.editor.current_line,
                );
            }
            for range in selection.ranges().iter().filter(|range| !range.is_empty()) {
                for row in state.layout.selection(range.start()..range.end()) {
                    if row.y + row.height < visible.start || row.y > visible.end {
                        continue;
                    }
                    renderer.fill_quad(
                        Quad {
                            bounds: row + offset,
                            border: Border {
                                radius: 2.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        colors.editor.selection,
                    );
                }
            }
            for (index, line) in state.layout.visible(visible.clone()) {
                renderer.fill_paragraph(
                    &line.paragraph,
                    Point::new(origin.x, origin.y + line.top),
                    colors.ink,
                    bounds,
                );
                let number_color = if index == current {
                    colors.ink
                } else {
                    colors.editor.line_number
                };
                renderer.fill_text(
                    number_text((index + 1).to_string(), state.gutter - GUTTER_GAP),
                    Point::new(origin.x - GUTTER_GAP, origin.y + line.top),
                    number_color,
                    bounds,
                );
            }
            self.draw_marks(state, renderer, origin, &visible);
            if state.focus.is_some_and(|focus| focus.caret_visible()) {
                for range in selection.ranges() {
                    let caret = state.layout.caret(range.head);
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(
                                    origin.x + caret.x - CARET_WIDTH / 2.0,
                                    origin.y + caret.y,
                                ),
                                Size::new(CARET_WIDTH, caret.height),
                            ),
                            ..Quad::default()
                        },
                        colors.accent,
                    );
                }
            }
        });
        if let Some(tooltip) = self.tooltip {
            self.draw_tooltip(state, renderer, bounds, tooltip);
        }
        self.draw_completions(state, renderer, bounds);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        if matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed { .. })
                | Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::WheelScrolled { .. })
        ) {
            self.end_hover(state, shell);
        }
        match event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = &mut state.focus {
                    focus.window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = &mut state.focus {
                    focus.window_focused = true;
                    focus.since = Instant::now();
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(focus) = &mut state.focus
                    && focus.window_focused
                {
                    focus.now = *now;
                    let elapsed = now.saturating_duration_since(focus.since);
                    let blinks = elapsed.as_millis() / BLINK.as_millis() + 1;
                    let next = BLINK * u32::try_from(blinks).unwrap_or(u32::MAX);
                    shell.request_redraw_at(focus.since + next);
                    shell.request_input_method(&self.input_method(state, bounds));
                }
                self.report_hover(state, *now, shell);
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_over(bounds);
                if !point.is_some_and(|point| self.choose(state, bounds, point, shell)) {
                    self.press(state, bounds, point, shell);
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                let target = self.snap(state.layout.hit(state.to_content(bounds, *position)));
                shell.publish((self.on_action)(Action::MoveTo {
                    position: target,
                    extend: true,
                    goal: None,
                }));
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                self.hover(state, bounds, Some(*position), shell);
            }
            Event::Mouse(mouse::Event::CursorLeft) => self.hover(state, bounds, None, shell),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let pixels = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -y * WHEEL_LINES * row_height(),
                    mouse::ScrollDelta::Pixels { y, .. } => -y,
                };
                state.scroll_by(pixels);
                shell.request_redraw();
                shell.capture_event();
            }
            Event::InputMethod(input_method::Event::Commit(text)) if state.focus.is_some() => {
                shell.publish((self.on_action)(Action::Insert(text.clone())));
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            }) if state.focus.is_some() => {
                let open = self.completions.is_some_and(|list| !list.items.is_empty());
                if let Some(event) = complete::completion_key(key, *modifiers, open)
                    && let Some(on_complete) = &self.on_complete
                {
                    shell.publish(on_complete(event));
                    shell.capture_event();
                } else if let Some(command) =
                    command(key, *physical_key, *modifiers, text.as_deref())
                {
                    self.run(command, state, clipboard, shell);
                    if let Some(focus) = &mut state.focus {
                        focus.since = Instant::now();
                    }
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        match cursor.position_in(layout.bounds()) {
            Some(point) if point.x >= state.gutter => mouse::Interaction::Text,
            _ => mouse::Interaction::default(),
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(self.id.as_ref(), layout.bounds(), state);
    }
}

impl<'a, Message: 'a> From<EditorView<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(view: EditorView<'a, Message>) -> Self {
        Element::new(view)
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Code;
    use iced::keyboard::{Location, Modifiers};
    use iced_test::{Simulator, simulator};
    use presswork_ui::color::LIGHT;

    use super::*;
    use crate::layout::tests::load_fonts;

    fn actions(text: &str, interact: impl FnOnce(&mut Simulator<'_, Action>)) -> Vec<Action> {
        load_fonts();
        let editor = Editor::new(text);
        let mut ui = simulator(view(&editor, &LIGHT, |action| action).autofocus(true));
        interact(&mut ui);
        ui.into_messages().collect()
    }

    fn shortcut(letter: &str, code: Code, modifiers: Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Character(letter.into()),
            modified_key: Key::Character(letter.into()),
            physical_key: Physical::Code(code),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    fn press(point: Point) -> impl FnOnce(&mut Simulator<'_, Action>) {
        move |ui| {
            ui.point_at(point);
            ui.simulate([
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
        }
    }

    fn move_to(offset: usize, extend: bool, goal: Option<f32>) -> Action {
        Action::MoveTo {
            position: Position::new(offset),
            extend,
            goal,
        }
    }

    #[test]
    fn typing_inserts_text() {
        let actions = actions("", |ui| {
            ui.typewrite("ab");
        });

        assert_eq!(
            actions,
            [
                Action::Insert("a".to_owned()),
                Action::Insert("b".to_owned())
            ]
        );
    }

    #[test]
    fn named_keys_edit_and_move() {
        let actions = actions("ab", |ui| {
            for key in [
                Named::Enter,
                Named::Backspace,
                Named::Delete,
                Named::ArrowRight,
                Named::End,
            ] {
                ui.tap_key(key);
            }
        });

        assert_eq!(
            actions,
            [
                Action::Newline,
                Action::Delete(Deletion::Backward),
                Action::Delete(Deletion::Forward),
                Action::Move {
                    motion: Motion::Right,
                    extend: false
                },
                Action::Move {
                    motion: Motion::LineEnd,
                    extend: false
                },
            ]
        );
    }

    #[test]
    fn shortcuts_select_all_and_undo() {
        let actions = actions("ab", |ui| {
            ui.simulate([
                shortcut("a", Code::KeyA, Modifiers::COMMAND),
                shortcut("z", Code::KeyZ, Modifiers::COMMAND),
                shortcut("z", Code::KeyZ, Modifiers::COMMAND | Modifiers::SHIFT),
            ]);
        });

        assert_eq!(actions, [Action::SelectAll, Action::Undo, Action::Redo]);
    }

    #[test]
    fn down_moves_to_the_next_line_and_keeps_the_goal() {
        let actions = actions("abc\ndef", |ui| {
            ui.tap_key(Named::ArrowDown);
        });

        assert_eq!(actions, [move_to(4, false, Some(0.0))]);
    }

    #[test]
    fn a_click_in_the_gutter_puts_the_cursor_at_the_line_start() {
        let actions = actions(
            "abc\ndef",
            press(Point::new(4.0, PADDING_TOP + row_height() * 1.5)),
        );

        assert_eq!(actions, [move_to(4, false, None)]);
    }

    #[test]
    fn a_click_below_the_text_puts_the_cursor_at_the_end() {
        let actions = actions("abc\ndef", press(Point::new(300.0, 600.0)));

        assert_eq!(actions, [move_to(7, false, None)]);
    }

    #[test]
    fn marks_draw_under_the_text() {
        load_fonts();
        let editor = Editor::new("#let x = \nmore text");
        let marks = vec![
            Mark {
                range: Position::new(5)..Position::new(6),
                kind: MarkKind::Error,
            },
            Mark {
                range: Position::new(12)..Position::new(12),
                kind: MarkKind::Warning,
            },
        ];
        let mut ui = simulator(view(&editor, &LIGHT, |action| action).marks(marks));

        assert!(ui.snapshot(&presswork_ui::theme::light()).is_ok());
    }

    #[test]
    fn the_view_gives_its_id_to_the_focus_operations() {
        load_fonts();
        let editor = Editor::new("ab");
        let mut ui = simulator(view(&editor, &LIGHT, |action| action).id("editor"));

        assert!(ui.find(widget::Id::from("editor")).is_ok());
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Report {
        Act(Action),
        Hover(Option<Position>),
        Complete(CompletionEvent),
    }

    fn completion_list(from: usize, labels: &[&str]) -> CompletionList {
        CompletionList {
            from: Position::new(from),
            items: labels
                .iter()
                .map(|label| crate::complete::CompletionItem {
                    label: (*label).to_owned(),
                    detail: Some(format!("About {label}.")),
                    text: (*label).to_owned(),
                    select: label.len()..label.len(),
                })
                .collect(),
            selected: 0,
        }
    }

    fn completion_reports(
        text: &str,
        list: Option<&CompletionList>,
        interact: impl FnOnce(&mut Simulator<'_, Report>),
    ) -> Vec<Report> {
        load_fonts();
        let mut editor = Editor::new(text);
        editor
            .perform(
                Action::Move {
                    motion: Motion::DocumentEnd,
                    extend: false,
                },
                Instant::now(),
            )
            .unwrap();
        let mut ui = simulator(
            view(&editor, &LIGHT, Report::Act)
                .autofocus(true)
                .on_complete(Report::Complete)
                .completions(list),
        );
        interact(&mut ui);
        ui.into_messages().collect()
    }

    #[test]
    fn keys_go_to_an_open_completion_list() {
        let list = completion_list(1, &["text", "table"]);

        let reports = completion_reports("#te", Some(&list), |ui| {
            ui.tap_key(Named::ArrowDown);
            ui.tap_key(Named::Enter);
            ui.tap_key(Named::Escape);
        });

        assert_eq!(
            reports,
            [
                Report::Complete(CompletionEvent::Next),
                Report::Complete(CompletionEvent::Accept),
                Report::Complete(CompletionEvent::Close)
            ]
        );
    }

    #[test]
    fn control_space_asks_for_completions() {
        let reports = completion_reports("#te", None, |ui| {
            ui.simulate([Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Space),
                modified_key: Key::Named(Named::Space),
                physical_key: Physical::Code(Code::Space),
                location: Location::Standard,
                modifiers: Modifiers::CTRL,
                text: Some(" ".into()),
                repeat: false,
            })]);
        });

        assert_eq!(reports, [Report::Complete(CompletionEvent::Request)]);
    }

    #[test]
    fn a_click_on_a_completion_row_chooses_it() {
        let list = completion_list(1, &["text", "table", "terms"]);
        let row = PADDING_TOP
            + row_height()
            + complete::LIST_GAP
            + complete::LIST_PADDING
            + complete::ROW_HEIGHT * 1.5;

        let reports = completion_reports("#te", Some(&list), |ui| {
            ui.point_at(Point::new(200.0, row));
            ui.simulate([
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
        });

        assert_eq!(reports, [Report::Complete(CompletionEvent::Choose(1))]);
    }

    #[test]
    fn a_view_with_completions_draws() {
        load_fonts();
        let editor = Editor::new("#te");
        let list = completion_list(1, &["text", "table", "terms"]);
        let mut ui = simulator(view(&editor, &LIGHT, |action| action).completions(Some(&list)));

        assert!(ui.snapshot(&presswork_ui::theme::light()).is_ok());
    }

    fn hovers(
        text: &str,
        interact: impl FnOnce(&mut Simulator<'_, Report>),
    ) -> Vec<Option<Position>> {
        load_fonts();
        let editor = Editor::new(text);
        let mut ui = simulator(
            view(&editor, &LIGHT, Report::Act)
                .autofocus(true)
                .on_hover(Report::Hover),
        );
        interact(&mut ui);
        ui.into_messages()
            .filter_map(|report| match report {
                Report::Hover(hover) => Some(hover),
                Report::Act(_) | Report::Complete(_) => None,
            })
            .collect()
    }

    fn rest(ui: &mut Simulator<'_, Report>, point: Point, wait: Duration) {
        ui.point_at(point);
        ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: point })]);
        ui.simulate([Event::Window(window::Event::RedrawRequested(
            Instant::now() + wait,
        ))]);
    }

    const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";
    const REST: Duration = Duration::from_millis(600);

    fn first_line(x: f32) -> Point {
        Point::new(x, PADDING_TOP + row_height() * 0.5)
    }

    #[test]
    fn resting_the_mouse_on_text_reports_its_position() {
        let hovers = hovers(ALPHABET, |ui| rest(ui, first_line(200.0), REST));

        assert!(
            matches!(
                hovers.as_slice(),
                [Some(position)] if position.offset() > 0 && position.offset() < ALPHABET.len()
            ),
            "{hovers:?}"
        );
    }

    #[test]
    fn a_short_rest_reports_nothing() {
        let hovers = hovers(ALPHABET, |ui| {
            rest(ui, first_line(200.0), Duration::from_millis(100));
        });

        assert_eq!(hovers, []);
    }

    #[test]
    fn a_rest_after_the_end_of_a_line_reports_nothing() {
        let hovers = hovers("ab", |ui| rest(ui, first_line(600.0), REST));

        assert_eq!(hovers, []);
    }

    #[test]
    fn moving_the_mouse_away_ends_a_reported_hover() {
        let hovers = hovers(ALPHABET, |ui| {
            rest(ui, first_line(200.0), REST);
            let away = Point::new(200.0, 600.0);
            ui.point_at(away);
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: away })]);
        });

        assert_eq!(hovers.len(), 2, "{hovers:?}");
        assert_eq!(hovers[1], None);
    }

    #[test]
    fn a_key_ends_a_reported_hover() {
        let hovers = hovers(ALPHABET, |ui| {
            rest(ui, first_line(200.0), REST);
            ui.tap_key(Named::ArrowLeft);
        });

        assert_eq!(hovers.len(), 2, "{hovers:?}");
        assert_eq!(hovers[1], None);
    }

    fn screen() -> Rectangle {
        Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0))
    }

    fn caret_at(x: f32, y: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(1.0, 22.0))
    }

    #[test]
    fn a_tooltip_goes_above_its_line() {
        let placed = tooltip_bounds(caret_at(100.0, 300.0), Size::new(200.0, 50.0), screen());

        assert_eq!(
            placed,
            Rectangle::new(
                Point::new(100.0, 300.0 - TOOLTIP_GAP - 50.0),
                Size::new(200.0, 50.0)
            )
        );
    }

    #[test]
    fn a_tooltip_without_room_above_goes_below_its_line() {
        let placed = tooltip_bounds(caret_at(100.0, 10.0), Size::new(200.0, 50.0), screen());

        assert!(
            (placed.y - (10.0 + 22.0 + TOOLTIP_GAP)).abs() < 0.01,
            "{placed:?}"
        );
    }

    #[test]
    fn a_tooltip_stays_inside_the_right_edge() {
        let placed = tooltip_bounds(caret_at(700.0, 300.0), Size::new(200.0, 50.0), screen());

        assert!((placed.x - 600.0).abs() < 0.01, "{placed:?}");
    }

    #[test]
    fn a_view_with_a_tooltip_draws() {
        load_fonts();
        let editor = Editor::new("#text(red)[Hi]");
        let tooltip = Tooltip {
            position: Position::new(1),
            text: "Customizes the look and layout of text.".to_owned(),
            code: false,
        };
        let mut ui = simulator(view(&editor, &LIGHT, |action| action).tooltip(Some(&tooltip)));

        assert!(ui.snapshot(&presswork_ui::theme::light()).is_ok());
    }

    #[test]
    fn without_focus_keys_do_nothing() {
        load_fonts();
        let editor = Editor::new("ab");
        let mut ui = simulator(view(&editor, &LIGHT, |action| action));

        ui.typewrite("x");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
