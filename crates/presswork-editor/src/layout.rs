use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::graphics::text::cosmic_text::{Cursor, LayoutRun};
use iced::advanced::text::{self, Paragraph as _, Span, Text};
use iced::alignment::Vertical;
use iced::font::{Style as FontStyle, Weight};
use iced::{Color, Font, Pixels, Point, Rectangle, Size};
use presswork_text::Position;
use presswork_ui::color::Colors;
use presswork_ui::font;

use crate::editor::Editor;
use crate::highlight::{Highlighter, Role, Style};

pub(crate) const TEXT_SIZE: f32 = 14.0;
pub(crate) const LINE_HEIGHT: f32 = 1.6;
const BREAK_WIDTH: f32 = TEXT_SIZE * 0.5;

pub(crate) fn row_height() -> f32 {
    TEXT_SIZE * LINE_HEIGHT
}

#[derive(Debug, Clone)]
pub(crate) struct Line {
    pub(crate) start: usize,
    pub(crate) len: usize,
    pub(crate) top: f32,
    pub(crate) height: f32,
    pub(crate) paragraph: Paragraph,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    editor: u64,
    revision: u64,
    width: u32,
    colors: Colors,
}

#[derive(Debug, Default)]
pub(crate) struct Layout {
    lines: Vec<Line>,
    key: Option<Key>,
    cache: HashMap<u64, Paragraph>,
}

impl Layout {
    pub(crate) fn update(
        &mut self,
        editor: &Editor,
        highlighter: &mut Highlighter,
        colors: &Colors,
        width: f32,
    ) {
        let key = Key {
            editor: editor.id(),
            revision: editor.revision(),
            width: width.to_bits(),
            colors: *colors,
        };
        if self.key == Some(key) {
            return;
        }
        if self
            .key
            .is_none_or(|old| old.width != key.width || old.colors != *colors)
        {
            self.cache.clear();
        }
        highlighter.sync(editor);
        let buffer = editor.document().buffer();
        let spans = highlighter.spans(0..buffer.len());
        let mut previous = std::mem::take(&mut self.cache);
        let mut lines = Vec::with_capacity(buffer.line_count());
        let mut top = 0.0;
        let mut next_span = 0;
        for index in 0..buffer.line_count() {
            let start = buffer.line_start(index).offset();
            let end = buffer.line_end(index).offset();
            let text = buffer.line(index).unwrap_or_default();
            let pieces = pieces(&spans, &mut next_span, start..end);
            let hash = hash_line(&text, &pieces);
            let paragraph = self
                .cache
                .get(&hash)
                .cloned()
                .or_else(|| previous.remove(&hash))
                .unwrap_or_else(|| shape(&text, &pieces, colors, width));
            self.cache.insert(hash, paragraph.clone());
            let height = paragraph.min_height().max(row_height());
            lines.push(Line {
                start,
                len: end - start,
                top,
                height,
                paragraph,
            });
            top += height;
        }
        self.lines = lines;
        self.key = Some(key);
    }

    pub(crate) fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub(crate) fn height(&self) -> f32 {
        self.lines.last().map_or(0.0, |line| line.top + line.height)
    }

    pub(crate) fn line_index(&self, position: Position) -> usize {
        self.lines
            .partition_point(|line| line.start <= position.offset())
            .saturating_sub(1)
    }

    pub(crate) fn visible(&self, range: Range<f32>) -> impl Iterator<Item = (usize, &Line)> {
        let first = self
            .lines
            .partition_point(|line| line.top + line.height <= range.start);
        self.lines
            .iter()
            .enumerate()
            .skip(first)
            .take_while(move |(_, line)| line.top < range.end)
    }

    pub(crate) fn caret(&self, position: Position) -> Rectangle {
        let Some(line) = self.lines.get(self.line_index(position)) else {
            return Rectangle::new(Point::ORIGIN, Size::new(0.0, row_height()));
        };
        let column = position.offset().saturating_sub(line.start).min(line.len);
        let buffer = line.paragraph.buffer();
        let runs: Vec<LayoutRun<'_>> = buffer.layout_runs().collect();
        let run = runs
            .iter()
            .rev()
            .find(|run| run.glyphs.first().is_none_or(|glyph| glyph.start <= column));
        match run {
            Some(run) => Rectangle::new(
                Point::new(caret_x(run, column), line.top + run.line_top),
                Size::new(0.0, run.line_height),
            ),
            None => Rectangle::new(Point::new(0.0, line.top), Size::new(0.0, row_height())),
        }
    }

    pub(crate) fn hit(&self, point: Point) -> Position {
        let Some(last) = self.lines.last() else {
            return Position::default();
        };
        if point.y >= self.height() {
            return Position::new(last.start + last.len);
        }
        let index = self
            .lines
            .partition_point(|line| line.top + line.height <= point.y)
            .min(self.lines.len() - 1);
        let line = &self.lines[index];
        let local = Point::new(
            point.x.max(0.0),
            (point.y - line.top).clamp(0.0, (line.height - 1.0).max(0.0)),
        );
        let column = match line.paragraph.hit_test(local) {
            Some(text::Hit::CharOffset(column)) => column,
            None => 0,
        };
        Position::new(line.start + column.min(line.len))
    }

    pub(crate) fn vertical(
        &self,
        head: Position,
        goal: Option<f32>,
        delta: f32,
    ) -> (Position, f32) {
        let caret = self.caret(head);
        let goal = goal.unwrap_or(caret.x);
        let y = caret.center_y() + delta;
        if y < 0.0 {
            return (Position::default(), goal);
        }
        (self.hit(Point::new(goal, y)), goal)
    }

    pub(crate) fn selection(&self, range: Range<Position>) -> Vec<Rectangle> {
        let mut rows = Vec::new();
        if range.start >= range.end {
            return rows;
        }
        let first = self.line_index(range.start);
        let last = self.line_index(range.end);
        for line in &self.lines[first..=last] {
            let start = range
                .start
                .offset()
                .saturating_sub(line.start)
                .min(line.len);
            let end = range.end.offset().saturating_sub(line.start).min(line.len);
            let selects_break = range.end.offset() > line.start + line.len;
            let runs: Vec<LayoutRun<'_>> = line.paragraph.buffer().layout_runs().collect();
            for run in &runs {
                if let Some((x, width)) = run.highlight(Cursor::new(0, start), Cursor::new(0, end))
                    && width > 0.0
                {
                    rows.push(Rectangle::new(
                        Point::new(x, line.top + run.line_top),
                        Size::new(width, run.line_height),
                    ));
                }
            }
            if selects_break {
                let (x, top, height) = runs.last().map_or((0.0, line.top, row_height()), |run| {
                    (run.line_w, line.top + run.line_top, run.line_height)
                });
                match rows.last_mut() {
                    Some(row) if (row.y - top).abs() < 0.5 => row.width += BREAK_WIDTH,
                    _ => rows.push(Rectangle::new(
                        Point::new(x, top),
                        Size::new(BREAK_WIDTH, height),
                    )),
                }
            }
        }
        rows
    }
}

fn pieces(
    spans: &[(Range<usize>, Style)],
    next_span: &mut usize,
    line: Range<usize>,
) -> Vec<(Range<usize>, Style)> {
    let mut pieces: Vec<(Range<usize>, Style)> = Vec::new();
    let mut covered = line.start;
    while let Some((range, style)) = spans.get(*next_span) {
        if range.start >= line.end {
            break;
        }
        let start = range.start.max(line.start);
        let end = range.end.min(line.end);
        if start < end {
            if covered < start {
                pieces.push((covered - line.start..start - line.start, Style::default()));
            }
            pieces.push((start - line.start..end - line.start, *style));
            covered = end;
        }
        if range.end > line.end {
            break;
        }
        *next_span += 1;
    }
    if covered < line.end {
        pieces.push((
            covered - line.start..line.end - line.start,
            Style::default(),
        ));
    }
    pieces
}

fn hash_line(text: &str, pieces: &[(Range<usize>, Style)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    pieces.hash(&mut hasher);
    hasher.finish()
}

fn shape(text: &str, pieces: &[(Range<usize>, Style)], colors: &Colors, width: f32) -> Paragraph {
    let spans: Vec<Span<'_, ()>> = pieces
        .iter()
        .map(|(range, style)| {
            Span::new(&text[range.clone()])
                .color(color(colors, style.role))
                .font(font_of(*style))
        })
        .collect();
    Paragraph::with_spans(Text {
        content: spans.as_slice(),
        bounds: Size::new(width, f32::INFINITY),
        size: Pixels(TEXT_SIZE),
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        font: font::MONO,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::WordOrGlyph,
    })
}

pub(crate) fn color(colors: &Colors, role: Option<Role>) -> Color {
    let editor = colors.editor;
    match role {
        None => colors.ink,
        Some(Role::Comment) => editor.comment,
        Some(Role::Keyword) => editor.keyword,
        Some(Role::Function) => editor.function,
        Some(Role::String) => editor.string,
        Some(Role::Number) => editor.number,
        Some(Role::Operator) => editor.operator,
        Some(Role::Reference) => editor.reference,
        Some(Role::Error) => editor.error,
    }
}

fn font_of(style: Style) -> Font {
    Font {
        weight: if style.bold {
            Weight::Bold
        } else {
            Weight::Normal
        },
        style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..font::MONO
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a cluster has few bytes, so the fraction is exact"
)]
fn caret_x(run: &LayoutRun<'_>, column: usize) -> f32 {
    for glyph in run.glyphs {
        if glyph.start == column {
            return if run.rtl { glyph.x + glyph.w } else { glyph.x };
        }
        if glyph.start < column && column < glyph.end {
            let fraction = (column - glyph.start) as f32 / (glyph.end - glyph.start) as f32;
            return glyph.x + glyph.w * fraction;
        }
    }
    run.glyphs.last().map_or(
        0.0,
        |glyph| {
            if run.rtl { glyph.x } else { glyph.x + glyph.w }
        },
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Once;

    use presswork_ui::color::LIGHT;

    use super::*;

    pub(crate) fn load_fonts() {
        static FONTS: Once = Once::new();
        FONTS.call_once(|| {
            let mut font_system = iced::advanced::graphics::text::font_system()
                .write()
                .unwrap();
            for bytes in font::all() {
                font_system.load_font(bytes);
            }
        });
    }

    fn layout(text: &str, width: f32) -> Layout {
        load_fonts();
        let mut layout = Layout::default();
        layout.update(
            &Editor::new(text),
            &mut Highlighter::default(),
            &LIGHT,
            width,
        );
        layout
    }

    fn positions(text: &str) -> Vec<Position> {
        (0..=text.len())
            .filter(|&offset| text.is_char_boundary(offset))
            .map(Position::new)
            .collect()
    }

    fn glyph_ids(text: &str) -> Vec<u16> {
        load_fonts();
        shape(text, &[(0..text.len(), Style::default())], &LIGHT, 400.0)
            .buffer()
            .layout_runs()
            .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.glyph_id))
            .collect()
    }

    #[test]
    fn the_editor_shows_each_character_as_typed() {
        for text in ["==", "---", "->", "=>", "...", "!=", "<=", ">=", "//", "/*"] {
            let alone: Vec<u16> = text
                .chars()
                .flat_map(|character| glyph_ids(&character.to_string()))
                .collect();
            assert_eq!(glyph_ids(text), alone, "{text}");
        }
    }

    #[test]
    fn lines_stack_from_the_top() {
        let layout = layout("a\nb\nc", 400.0);

        let tops: Vec<f32> = layout.lines().iter().map(|line| line.top).collect();
        assert_eq!(tops, [0.0, row_height(), 2.0 * row_height()]);
        assert!((layout.height() - 3.0 * row_height()).abs() < 0.01);
    }

    #[test]
    fn a_long_line_wraps_at_the_width() {
        let text = "word ".repeat(40);
        let layout = layout(&text, 200.0);

        assert_eq!(layout.lines().len(), 1);
        assert!(layout.lines()[0].height > 2.0 * row_height());
    }

    #[test]
    fn the_caret_moves_right_along_a_line() {
        let layout = layout("abc", 400.0);

        let xs: Vec<f32> = positions("abc")
            .into_iter()
            .map(|position| layout.caret(position).x)
            .collect();
        assert!(xs.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_hit_at_the_caret_finds_the_same_position() {
        let text = "Typst and Presswork make books.\n\nA second paragraph wraps here.";
        let layout = layout(text, 160.0);

        for position in positions(text) {
            let caret = layout.caret(position);
            let hit = layout.hit(Point::new(caret.x + 0.5, caret.center_y()));
            assert_eq!(hit, position, "caret {caret:?}");
        }
    }

    #[test]
    fn vertical_movement_keeps_the_goal_column() {
        let layout = layout("abcdef\nab\nabcdef", 400.0);

        let (down, goal) = layout.vertical(Position::new(5), None, row_height());
        assert_eq!(down, Position::new(9));

        let (again, _) = layout.vertical(down, Some(goal), row_height());
        assert_eq!(again, Position::new(15));
    }

    #[test]
    fn vertical_movement_stops_at_the_ends_of_the_text() {
        let layout = layout("ab\ncd", 400.0);

        assert_eq!(
            layout.vertical(Position::new(1), None, -row_height()).0,
            Position::new(0)
        );
        assert_eq!(
            layout.vertical(Position::new(4), None, row_height()).0,
            Position::new(5)
        );
    }

    #[test]
    fn a_selection_has_one_rectangle_for_each_row() {
        let text = "word ".repeat(40);
        let layout = layout(&text, 200.0);

        let rows = layout.selection(Position::new(0)..Position::new(text.len()));

        assert!(rows.len() > 2);
        assert!(rows.windows(2).all(|pair| pair[0].y < pair[1].y));
    }

    #[test]
    fn a_selection_over_a_line_break_marks_the_break() {
        let layout = layout("ab\n\ncd", 400.0);

        let rows = layout.selection(Position::new(1)..Position::new(5));

        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.width > 0.0));
    }
}
