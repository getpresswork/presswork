use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{AbsoluteOffset, Direction, Scrollbar};
use iced::widget::{
    button, column, container, image, operation, pin, responsive, row, rule, scrollable, space,
    stack, text, tooltip,
};
use iced::{Border, Color, Element, Fill, Point, Rectangle, Size, Task};
use presswork_typst::{Insets, Output, PagePoint};
use presswork_ui::color::LIGHT;
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::page::page_area;

const PAGES: &str = "preview-pages";
const SHOWN_MARGIN: f32 = 1.0 / 6.0;
const SHOWN_AT: f32 = 1.0 / 3.0;
const PX_PER_PT: f32 = 96.0 / 72.0;
const STEPS: [u16; 11] = [25, 50, 67, 75, 90, 100, 125, 150, 200, 300, 400];
const FULL_SIZE: u16 = 100;
const PADDING: f32 = 32.0;
const PAGE_GAP: f32 = 24.0;
const SCROLLBAR_SPACE: f32 = 12.0;
const EMPTY_WIDTH: f32 = 320.0;
const PERCENT_WIDTH: f32 = 48.0;
const THUMBNAIL_WIDTH: f32 = 72.0;
const THUMBNAIL_STRIP: f32 = 120.0;
const PROBLEM_PADDING: f32 = 2.0;
const PROBLEM_BORDER: f32 = 1.5;
const PROBLEM_RADIUS: f32 = 2.0;
const PROBLEM_FILL_ALPHA: f32 = 0.12;
const TIP_WIDTH: f32 = 320.0;
const TIP_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProblemMark {
    pub page: usize,
    pub area: presswork_typst::Area,
    pub text: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zoom {
    #[default]
    FitWidth,
    Percent(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arrangement {
    #[default]
    Pages,
    Spreads,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Visibility {
    #[default]
    Shown,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    ZoomIn,
    ZoomOut,
    FitWidth,
    Spreads,
    Guides,
    Thumbnails,
    GoToPage(usize),
    ShowProblem,
    ProblemMarks,
    Detach,
    Attach,
    Click(PagePoint),
    Scrolled(Viewport),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Viewport {
    top: f32,
    height: f32,
    width: f32,
}

impl From<scrollable::Viewport> for Viewport {
    fn from(viewport: scrollable::Viewport) -> Self {
        let bounds = viewport.bounds();
        Self {
            top: viewport.absolute_offset().y,
            height: bounds.height,
            width: bounds.width,
        }
    }
}

#[derive(Debug, Clone)]
struct Page {
    handle: image::Handle,
    width: f32,
    height: f32,
    bleed: Insets,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Frame {
    scale: f32,
    width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Preview {
    pages: Vec<Page>,
    zoom: Zoom,
    arrangement: Arrangement,
    thumbnails: bool,
    guides: bool,
    problem: Option<String>,
    detached: bool,
    viewport: Option<Viewport>,
    mark: Option<(PagePoint, Instant)>,
    problem_marks: Vec<ProblemMark>,
    problem_marks_visibility: Visibility,
}

impl Preview {
    pub fn set_problem_marks(&mut self, marks: Vec<ProblemMark>) {
        self.problem_marks = marks;
    }

    #[must_use]
    pub fn problem_marks(&self) -> &[ProblemMark] {
        &self.problem_marks
    }

    #[must_use]
    pub fn shows_problem_marks(&self) -> bool {
        self.problem_marks_visibility == Visibility::Shown
    }

    fn problem_marks_on(&self, page: usize, scale: f32) -> Vec<(Rectangle, &ProblemMark)> {
        let Some(shown) = self.pages.get(page).filter(|_| self.shows_problem_marks()) else {
            return Vec::new();
        };
        let ratio = PX_PER_PT * scale;
        let bounds = Rectangle::new(
            Point::ORIGIN,
            Size::new(shown.width * ratio, shown.height * ratio),
        );
        self.problem_marks
            .iter()
            .filter(|mark| mark.page == page)
            .filter_map(|mark| {
                let area = mark.area;
                let marked = Rectangle::new(
                    Point::new(
                        (to_f32(area.x) + shown.bleed.left) * ratio - PROBLEM_PADDING,
                        (to_f32(area.y) + shown.bleed.top) * ratio - PROBLEM_PADDING,
                    ),
                    Size::new(
                        to_f32(area.width) * ratio + 2.0 * PROBLEM_PADDING,
                        to_f32(area.height) * ratio + 2.0 * PROBLEM_PADDING,
                    ),
                );
                marked.intersection(&bounds).map(|visible| (visible, mark))
            })
            .collect()
    }

    pub fn show(&mut self, output: &Output, pixel_per_pt: f32) {
        if let Some(pages) = &output.pages {
            self.pages = pages
                .iter()
                .map(|page| Page {
                    handle: image::Handle::from_rgba(page.width, page.height, page.rgba.clone()),
                    width: pixels(page.width) / pixel_per_pt,
                    height: pixels(page.height) / pixel_per_pt,
                    bleed: page.bleed,
                })
                .collect();
        }
    }

    pub fn set_problem(&mut self, problem: Option<String>) {
        self.problem = problem;
    }

    pub fn set_detached(&mut self, detached: bool) {
        self.detached = detached;
    }

    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    #[must_use]
    pub fn zoom(&self) -> Zoom {
        self.zoom
    }

    #[must_use]
    pub fn marked(&self) -> Option<PagePoint> {
        self.mark.map(|(point, _)| point)
    }

    #[must_use]
    pub fn arrangement(&self) -> Arrangement {
        self.arrangement
    }

    #[must_use]
    pub fn guides(&self) -> bool {
        self.guides
    }

    fn trim_on(&self, page: usize, scale: f32) -> Option<Rectangle> {
        let page = self.pages.get(page).filter(|_| self.guides)?;
        let bleed = page.bleed;
        if bleed == Insets::default() {
            return None;
        }
        let ratio = PX_PER_PT * scale;
        Some(Rectangle::new(
            Point::new(bleed.left * ratio, bleed.top * ratio),
            Size::new(
                (page.width - bleed.left - bleed.right) * ratio,
                (page.height - bleed.top - bleed.bottom) * ratio,
            ),
        ))
    }

    fn rows(&self) -> Vec<std::ops::Range<usize>> {
        let count = self.pages.len();
        match self.arrangement {
            Arrangement::Pages => (0..count).map(|index| index..index + 1).collect(),
            Arrangement::Spreads => {
                let mut rows = Vec::new();
                if count > 0 {
                    rows.push(0..1);
                }
                let mut start = 1;
                while start < count {
                    rows.push(start..(start + 2).min(count));
                    start += 2;
                }
                rows
            }
        }
    }

    fn page_target(&self, page: usize) -> Option<f32> {
        let viewport = self.viewport?;
        let Frame { scale, .. } = self.frame(viewport.width);
        Some((self.row_top(page, scale)? - PAGE_GAP / 2.0).max(0.0))
    }

    fn current_page(&self) -> Option<usize> {
        let viewport = self.viewport?;
        let Frame { scale, .. } = self.frame(viewport.width);
        let middle = viewport.top + viewport.height / 2.0;
        let rows = self.rows();
        let mut bottom = PADDING;
        for row in &rows {
            bottom += self.row_height(row) * PX_PER_PT * scale + PAGE_GAP;
            if middle < bottom {
                return Some(row.start);
            }
        }
        rows.last().map(|row| row.start)
    }

    fn row_height(&self, row: &std::ops::Range<usize>) -> f32 {
        self.pages[row.clone()]
            .iter()
            .map(|page| page.height)
            .fold(0.0, f32::max)
    }

    fn row_top(&self, page: usize, scale: f32) -> Option<f32> {
        let rows = self.rows();
        let row = rows.iter().position(|row| row.contains(&page))?;
        let above: f32 = rows[..row]
            .iter()
            .map(|row| self.row_height(row) * PX_PER_PT * scale + PAGE_GAP)
            .sum();
        Some(PADDING + above)
    }

    pub fn perform(&mut self, action: Action) {
        let current = match self.zoom {
            Zoom::Percent(percent) => percent,
            Zoom::FitWidth => FULL_SIZE,
        };
        self.zoom = match action {
            Action::FitWidth => Zoom::FitWidth,
            Action::ZoomIn => Zoom::Percent(
                STEPS
                    .into_iter()
                    .find(|&step| step > current)
                    .unwrap_or(STEPS[STEPS.len() - 1]),
            ),
            Action::ZoomOut => Zoom::Percent(
                STEPS
                    .into_iter()
                    .rev()
                    .find(|&step| step < current)
                    .unwrap_or(STEPS[0]),
            ),
            Action::Scrolled(viewport) => {
                self.viewport = Some(viewport);
                return;
            }
            Action::Spreads => {
                self.arrangement = match self.arrangement {
                    Arrangement::Pages => Arrangement::Spreads,
                    Arrangement::Spreads => Arrangement::Pages,
                };
                return;
            }
            Action::Thumbnails => {
                self.thumbnails = !self.thumbnails;
                return;
            }
            Action::Guides => {
                self.guides = !self.guides;
                return;
            }
            Action::ProblemMarks => {
                self.problem_marks_visibility = match self.problem_marks_visibility {
                    Visibility::Shown => Visibility::Hidden,
                    Visibility::Hidden => Visibility::Shown,
                };
                return;
            }
            Action::GoToPage(_)
            | Action::ShowProblem
            | Action::Detach
            | Action::Attach
            | Action::Click(_) => return,
        };
    }

    pub fn go_to_page<T>(&mut self, page: usize) -> Task<T> {
        match self.page_target(page) {
            Some(top) => operation::scroll_to(
                PAGES,
                AbsoluteOffset {
                    x: None,
                    y: Some(top),
                },
            ),
            None => Task::none(),
        }
    }

    pub fn show_point<T>(&mut self, point: PagePoint, now: Instant) -> Task<T> {
        self.mark = Some((point, now));
        match self.scroll_target(point) {
            Some(top) => operation::scroll_to(
                PAGES,
                AbsoluteOffset {
                    x: None,
                    y: Some(top),
                },
            ),
            None => Task::none(),
        }
    }

    fn scroll_target(&self, point: PagePoint) -> Option<f32> {
        let viewport = self.viewport?;
        let Frame { scale, .. } = self.frame(viewport.width);
        let top = self.pages.get(point.page)?.bleed.top;
        let y = self.row_top(point.page, scale)? + (to_f32(point.y) + top) * PX_PER_PT * scale;
        let margin = viewport.height * SHOWN_MARGIN;
        if y >= viewport.top + margin && y <= viewport.top + viewport.height - margin {
            return None;
        }
        Some((y - viewport.height * SHOWN_AT).max(0.0))
    }

    fn mark_on(&self, page: usize, scale: f32) -> Option<(Point, Instant)> {
        let (point, since) = self.mark?;
        let bleed = self.pages.get(page)?.bleed;
        (point.page == page).then(|| {
            let ratio = PX_PER_PT * scale;
            (
                Point::new(
                    (to_f32(point.x) + bleed.left) * ratio,
                    (to_f32(point.y) + bleed.top) * ratio,
                ),
                since,
            )
        })
    }

    pub fn view<'a, Message: Clone + 'a>(
        &'a self,
        empty: String,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let mut content = column![];
        if let Some(problem) = &self.problem {
            content = content.push(problem_banner(problem, on_action(Action::ShowProblem)));
        }
        let body: Element<'a, Message> = if self.pages.is_empty() {
            container(
                column![
                    icon(Icon::FileText)
                        .size(size::ICON_LARGE)
                        .style(style::muted),
                    text(empty)
                        .style(style::muted)
                        .width(EMPTY_WIDTH)
                        .align_x(Horizontal::Center),
                ]
                .spacing(gap::S)
                .align_x(Horizontal::Center),
            )
            .center(Fill)
            .into()
        } else {
            responsive(move |size| self.pages_view(size, on_action)).into()
        };
        container(content.push(body))
            .width(Fill)
            .height(Fill)
            .style(style::chrome)
            .into()
    }

    fn frame(&self, viewport: f32) -> Frame {
        let available = (viewport - SCROLLBAR_SPACE).max(0.0);
        let mut widest = self.pages.iter().map(|page| page.width).fold(0.0, f32::max) * PX_PER_PT;
        if self.arrangement == Arrangement::Spreads {
            widest *= 2.0;
        }
        let scale = match self.zoom {
            Zoom::Percent(percent) => f32::from(percent) / f32::from(FULL_SIZE),
            Zoom::FitWidth if widest > 0.0 => {
                ((available - 2.0 * PADDING) / widest).clamp(0.1, 4.0)
            }
            Zoom::FitWidth => 1.0,
        };
        Frame {
            scale,
            width: available.max(widest * scale + 2.0 * PADDING),
        }
    }

    fn pages_view<'a, Message: Clone + 'a>(
        &'a self,
        size: Size,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let strip = if self.thumbnails {
            THUMBNAIL_STRIP + 1.0
        } else {
            0.0
        };
        let Frame { scale, width } = self.frame(size.width - strip);
        let percent = scale * 100.0;
        let toolbar = row![
            tool(
                Icon::PanelLeft,
                self.thumbnails,
                on_action(Action::Thumbnails)
            ),
            space().width(Fill),
            tool(
                Icon::BookOpen,
                self.arrangement == Arrangement::Spreads,
                on_action(Action::Spreads)
            ),
            tool(Icon::Frame, self.guides, on_action(Action::Guides)),
            tool(
                Icon::TriangleAlert,
                self.shows_problem_marks(),
                on_action(Action::ProblemMarks)
            ),
            space().width(gap::S),
            tool(Icon::ZoomOut, false, on_action(Action::ZoomOut)),
            text(format!("{percent:.0}%"))
                .size(font_size::SMALL)
                .font(font::MONO)
                .style(style::muted)
                .width(PERCENT_WIDTH)
                .align_x(Horizontal::Center),
            tool(Icon::ZoomIn, false, on_action(Action::ZoomIn)),
            tool(
                Icon::MoveHorizontal,
                self.zoom == Zoom::FitWidth,
                on_action(Action::FitWidth)
            ),
            space().width(gap::S),
            if self.detached {
                tool(Icon::Columns2, false, on_action(Action::Attach))
            } else {
                tool(Icon::ExternalLink, false, on_action(Action::Detach))
            },
        ]
        .spacing(gap::XXS)
        .align_y(Vertical::Center)
        .padding([gap::XS, gap::S]);
        let slot = self.pages.iter().map(|page| page.width).fold(0.0, f32::max) * PX_PER_PT * scale;
        let rows = self.rows().into_iter().map(|shown| match self.arrangement {
            Arrangement::Pages => self.page_view(shown.start, scale, on_action),
            Arrangement::Spreads => {
                let half = |index: Option<usize>, align: Horizontal| -> Element<'a, Message> {
                    let content = match index {
                        Some(index) => self.page_view(index, scale, on_action),
                        None => space().into(),
                    };
                    container(content).width(slot).align_x(align).into()
                };
                let left = shown.clone().find(|index| index % 2 == 1);
                let right = shown.clone().find(|index| index % 2 == 0);
                row![half(left, Horizontal::Right), half(right, Horizontal::Left)].into()
            }
        });
        let pages = column(rows)
            .spacing(PAGE_GAP)
            .padding(PADDING)
            .align_x(Horizontal::Center)
            .width(width);
        let pages = scrollable(pages)
            .id(PAGES)
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .on_scroll(move |viewport| on_action(Action::Scrolled(viewport.into())))
            .width(Fill)
            .height(Fill);
        if self.thumbnails {
            let strip = self.thumbnails_view(on_action);
            return column![
                toolbar,
                row![strip, rule::vertical(1).style(style::hairline), pages]
            ]
            .into();
        }
        column![toolbar, pages].into()
    }

    fn thumbnails_view<'a, Message: Clone + 'a>(
        &'a self,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let current = self.current_page();
        let items = self.pages.iter().enumerate().map(|(index, page)| {
            let height = page.height / page.width.max(1.0) * THUMBNAIL_WIDTH;
            let picture = container(
                image(page.handle.clone())
                    .width(THUMBNAIL_WIDTH)
                    .height(height),
            )
            .style(style::paper);
            let number = text((index + 1).to_string())
                .size(font_size::CAPTION)
                .font(font::MONO);
            container(
                button(
                    column![picture, number]
                        .spacing(gap::XS)
                        .align_x(Horizontal::Center),
                )
                .padding(gap::XS)
                .style(style::tab(current == Some(index)))
                .on_press(on_action(Action::GoToPage(index))),
            )
            .id(format!("thumbnail-{index}"))
            .into()
        });
        container(
            scrollable(
                column(items)
                    .spacing(gap::S)
                    .padding(gap::S)
                    .align_x(Horizontal::Center)
                    .width(Fill),
            )
            .height(Fill),
        )
        .width(THUMBNAIL_STRIP)
        .height(Fill)
        .into()
    }

    fn page_view<'a, Message: Clone + 'a>(
        &'a self,
        index: usize,
        scale: f32,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let page = &self.pages[index];
        let content = container(
            image(page.handle.clone())
                .width(page.width * PX_PER_PT * scale)
                .height(page.height * PX_PER_PT * scale),
        )
        .style(style::paper)
        .id(format!("page-{index}"));
        let marks = self.problem_marks_on(index, scale);
        let content: Element<'a, Message> = if marks.is_empty() {
            content.into()
        } else {
            marks
                .into_iter()
                .enumerate()
                .fold(stack![content], |layers, (number, (bounds, mark))| {
                    layers.push(
                        pin(problem_view(
                            mark,
                            bounds.size(),
                            format!("problem-{index}-{number}"),
                        ))
                        .position(bounds.position()),
                    )
                })
                .into()
        };
        page_area(content, move |point| {
            on_action(Action::Click(PagePoint {
                page: index,
                x: f64::from(point.x / (PX_PER_PT * scale) - page.bleed.left),
                y: f64::from(point.y / (PX_PER_PT * scale) - page.bleed.top),
            }))
        })
        .mark(self.mark_on(index, scale))
        .trim(self.trim_on(index, scale))
        .into()
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "page images are far smaller than 2^24 pixels"
)]
fn pixels(value: u32) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "page coordinates are far below the limits of f32"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

fn problem_view<'a, Message: 'a>(
    mark: &'a ProblemMark,
    size: Size,
    id: String,
) -> Element<'a, Message> {
    let color = match mark.severity {
        Severity::Warning => LIGHT.warning,
        Severity::Error => LIGHT.danger,
    };
    let area = container(space().width(size.width).height(size.height))
        .id(id)
        .style(move |_| container::Style {
            background: Some(
                Color {
                    a: PROBLEM_FILL_ALPHA,
                    ..color
                }
                .into(),
            ),
            border: Border {
                color,
                width: PROBLEM_BORDER,
                radius: PROBLEM_RADIUS.into(),
            },
            ..container::Style::default()
        });
    let card = container(text(mark.text.as_str()).size(font_size::CAPTION))
        .max_width(TIP_WIDTH)
        .padding([gap::XXS, gap::S])
        .style(style::popup);
    tooltip(area, card, tooltip::Position::Top)
        .gap(gap::XS)
        .delay(TIP_DELAY)
        .into()
}

fn tool<'a, Message: Clone + 'a>(
    symbol: Icon,
    selected: bool,
    message: Message,
) -> Element<'a, Message> {
    button(icon(symbol))
        .padding(gap::XS)
        .style(style::tab(selected))
        .on_press(message)
        .into()
}

fn problem_banner<'a, Message: Clone + 'a>(
    problem: &'a str,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        row![
            icon(Icon::CircleAlert)
                .size(size::ICON_SMALL)
                .style(style::tone(Tone::Danger)),
            text(problem)
                .size(font_size::SMALL)
                .style(style::tone(Tone::Danger)),
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center),
    )
    .padding([gap::XS, gap::M])
    .width(Fill)
    .style(style::notice(Tone::Danger))
    .on_press(on_press)
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_typst::PageImage;

    use super::*;

    const EMPTY: &str = "The preview shows the pages after the first compilation.";

    fn output(pages: usize) -> Output {
        Output {
            pages: Some(
                (0..pages)
                    .map(|_| PageImage {
                        width: 20,
                        height: 30,
                        rgba: vec![255; 20 * 30 * 4],
                        ..PageImage::default()
                    })
                    .collect(),
            ),
            ..Output::default()
        }
    }

    #[test]
    fn show_takes_the_pages_of_an_output() {
        let mut preview = Preview::default();

        preview.show(&output(2), 2.0);

        assert_eq!(preview.page_count(), 2);
    }

    #[test]
    fn an_output_without_pages_keeps_the_last_pages() {
        let mut preview = Preview::default();
        preview.show(&output(1), 2.0);

        preview.show(&Output::default(), 2.0);

        assert_eq!(preview.page_count(), 1);
    }

    #[test]
    fn zoom_steps_up_and_down_from_the_full_size() {
        let mut preview = Preview::default();

        preview.perform(Action::ZoomIn);
        assert_eq!(preview.zoom(), Zoom::Percent(125));
        preview.perform(Action::ZoomIn);
        assert_eq!(preview.zoom(), Zoom::Percent(150));
        preview.perform(Action::ZoomOut);
        preview.perform(Action::ZoomOut);
        assert_eq!(preview.zoom(), Zoom::Percent(100));
        preview.perform(Action::FitWidth);
        assert_eq!(preview.zoom(), Zoom::FitWidth);
        preview.perform(Action::ZoomOut);
        assert_eq!(preview.zoom(), Zoom::Percent(90));
    }

    #[test]
    fn zoom_stops_at_the_smallest_and_the_largest_step() {
        let mut preview = Preview::default();
        for _ in 0..20 {
            preview.perform(Action::ZoomOut);
        }
        assert_eq!(preview.zoom(), Zoom::Percent(25));

        for _ in 0..20 {
            preview.perform(Action::ZoomIn);
        }
        assert_eq!(preview.zoom(), Zoom::Percent(400));
    }

    fn a5_page() -> Output {
        Output {
            pages: Some(vec![PageImage {
                width: 840,
                height: 1190,
                rgba: vec![255; 840 * 1190 * 4],
                ..PageImage::default()
            }]),
            ..Output::default()
        }
    }

    #[test]
    fn a_zoomed_page_keeps_its_size_and_makes_the_content_wider() {
        let mut preview = Preview::default();
        preview.show(&a5_page(), 2.0);
        preview.perform(Action::ZoomIn);
        preview.perform(Action::ZoomIn);

        let frame = preview.frame(640.0);

        assert!((frame.scale - 1.5).abs() < 1e-6);
        assert!((frame.width - (420.0 * PX_PER_PT * 1.5 + 2.0 * PADDING)).abs() < 0.01);
    }

    #[test]
    fn fit_width_keeps_the_content_inside_the_available_width() {
        let mut preview = Preview::default();
        preview.show(&a5_page(), 2.0);

        let frame = preview.frame(640.0);

        assert!((frame.width - (640.0 - SCROLLBAR_SPACE)).abs() < 0.01);
        let page_width = 420.0 * PX_PER_PT * frame.scale;
        assert!((page_width + 2.0 * PADDING - frame.width).abs() < 0.01);
    }

    fn clicked(preview: &Preview, symbol: Icon) -> Vec<Action> {
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));
        ui.click(symbol.unicode().to_string()).unwrap();
        ui.into_messages().collect()
    }

    #[test]
    fn the_toolbar_offers_its_own_window_for_the_preview() {
        let mut preview = Preview::default();
        preview.show(&output(1), 2.0);

        assert_eq!(clicked(&preview, Icon::ExternalLink), [Action::Detach]);
    }

    #[test]
    fn a_detached_preview_offers_to_go_back_beside_the_editor() {
        let mut preview = Preview::default();
        preview.show(&output(1), 2.0);
        preview.set_detached(true);

        assert_eq!(clicked(&preview, Icon::Columns2), [Action::Attach]);
    }

    #[test]
    fn the_view_tells_what_to_expect_before_the_first_compilation() {
        let preview = Preview::default();
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        assert!(ui.find(EMPTY).is_ok());
    }

    #[test]
    fn a_click_on_the_problem_asks_to_show_it() {
        let mut preview = Preview::default();
        preview.set_problem(Some("Line 2: unknown variable".to_owned()));
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        ui.click("Line 2: unknown variable").unwrap();

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            [Action::ShowProblem]
        );
    }

    #[test]
    fn the_view_shows_the_problem_above_the_pages() {
        let mut preview = Preview::default();
        preview.show(&output(1), 2.0);
        preview.set_problem(Some("Line 2: unknown variable".to_owned()));

        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        assert!(ui.find("Line 2: unknown variable").is_ok());
        assert!(ui.find(EMPTY).is_err());
    }

    fn a5_pages(count: usize) -> Output {
        Output {
            pages: Some(
                (0..count)
                    .map(|_| PageImage {
                        width: 840,
                        height: 1190,
                        rgba: vec![255; 840 * 1190 * 4],
                        ..PageImage::default()
                    })
                    .collect(),
            ),
            ..Output::default()
        }
    }

    fn point(page: usize, x: f64, y: f64) -> PagePoint {
        PagePoint { page, x, y }
    }

    fn scrolled(top: f32, height: f32, width: f32) -> Action {
        Action::Scrolled(Viewport { top, height, width })
    }

    #[test]
    fn a_click_on_a_page_gives_the_point_on_the_page() {
        let mut preview = Preview::default();
        preview.show(&a5_page(), 2.0);
        for _ in 0..20 {
            preview.perform(Action::ZoomOut);
        }
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        ui.click(iced::widget::Id::from("page-0".to_owned()))
            .unwrap();

        let clicks: Vec<PagePoint> = ui
            .into_messages()
            .filter_map(|action| match action {
                Action::Click(point) => Some(point),
                _ => None,
            })
            .collect();
        assert_eq!(clicks.len(), 1, "{clicks:?}");
        assert_eq!(clicks[0].page, 0);
        assert!((clicks[0].x - 210.0).abs() < 1.0, "{clicks:?}");
        assert!((clicks[0].y - 297.5).abs() < 1.0, "{clicks:?}");
    }

    #[test]
    fn a_point_below_the_view_scrolls_to_a_third_of_the_view() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        preview.perform(scrolled(0.0, 600.0, 640.0));
        let scale = preview.frame(640.0).scale;
        let y = PADDING + 595.0 * PX_PER_PT * scale + PAGE_GAP + 100.0 * PX_PER_PT * scale;

        let target = preview.scroll_target(point(1, 50.0, 100.0)).unwrap();

        assert!((target - (y - 200.0)).abs() < 0.5, "{target} {y}");
    }

    #[test]
    fn a_point_in_the_view_does_not_scroll() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        preview.perform(scrolled(0.0, 600.0, 640.0));

        assert_eq!(preview.scroll_target(point(0, 50.0, 200.0)), None);
    }

    #[test]
    fn without_a_known_view_the_preview_does_not_scroll() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);

        assert_eq!(preview.scroll_target(point(2, 50.0, 100.0)), None);
    }

    #[test]
    fn single_pages_have_one_page_in_each_row() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);

        assert_eq!(preview.rows(), [0..1, 1..2, 2..3]);
    }

    #[test]
    fn spreads_put_the_first_page_alone_and_then_two_pages_in_each_row() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(4), 2.0);

        preview.perform(Action::Spreads);

        assert_eq!(preview.arrangement(), Arrangement::Spreads);
        assert_eq!(preview.rows(), [0..1, 1..3, 3..4]);
    }

    #[test]
    fn fit_width_in_spreads_fits_two_pages() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        preview.perform(Action::Spreads);

        let frame = preview.frame(640.0);

        let spread = 2.0 * 420.0 * PX_PER_PT * frame.scale;
        assert!((spread + 2.0 * PADDING - frame.width).abs() < 0.01);
    }

    #[test]
    fn a_point_on_a_right_page_of_a_spread_scrolls_by_rows() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        preview.perform(Action::Spreads);
        preview.perform(scrolled(0.0, 300.0, 640.0));
        let scale = preview.frame(640.0).scale;
        let y = PADDING + 595.0 * PX_PER_PT * scale + PAGE_GAP + 100.0 * PX_PER_PT * scale;

        let target = preview.scroll_target(point(2, 50.0, 100.0)).unwrap();

        assert!((target - (y - 100.0)).abs() < 0.5, "{target} {y}");
    }

    #[test]
    fn the_toolbar_switches_between_pages_and_spreads() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);

        assert_eq!(clicked(&preview, Icon::BookOpen), [Action::Spreads]);
    }

    #[test]
    fn spreads_show_all_pages() {
        let mut preview = Preview::default();
        preview.show(&output(4), 2.0);
        preview.perform(Action::Spreads);

        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        for index in 0..4 {
            assert!(
                ui.find(iced::widget::Id::from(format!("page-{index}")))
                    .is_ok(),
                "page {index}"
            );
        }
    }

    #[test]
    fn the_toolbar_shows_and_hides_the_thumbnails() {
        let mut preview = Preview::default();
        preview.show(&output(3), 2.0);

        assert_eq!(clicked(&preview, Icon::PanelLeft), [Action::Thumbnails]);
        preview.perform(Action::Thumbnails);
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));
        assert!(
            ui.find(iced::widget::Id::from("thumbnail-2".to_owned()))
                .is_ok()
        );
    }

    #[test]
    fn a_click_on_a_thumbnail_goes_to_its_page() {
        let mut preview = Preview::default();
        preview.show(&output(3), 2.0);
        preview.perform(Action::Thumbnails);
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        ui.click(iced::widget::Id::from("thumbnail-1".to_owned()))
            .unwrap();

        assert!(
            ui.into_messages()
                .any(|action| action == Action::GoToPage(1))
        );
    }

    #[test]
    fn a_page_shows_at_the_top_of_the_view() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        preview.perform(scrolled(0.0, 600.0, 640.0));
        let scale = preview.frame(640.0).scale;

        let target = preview.page_target(1).unwrap();

        let top = PADDING + 595.0 * PX_PER_PT * scale + PAGE_GAP / 2.0;
        assert!((target - top).abs() < 0.5, "{target} {top}");
    }

    #[test]
    fn the_current_page_is_in_the_middle_of_the_view() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(3), 2.0);
        let scale = preview.frame(640.0).scale;
        let page = 595.0 * PX_PER_PT * scale + PAGE_GAP;

        preview.perform(scrolled(0.0, 200.0, 640.0));
        assert_eq!(preview.current_page(), Some(0));
        preview.perform(scrolled(PADDING + page, 200.0, 640.0));
        assert_eq!(preview.current_page(), Some(1));
    }

    const BLEED: presswork_typst::Insets = presswork_typst::Insets {
        left: 10.0,
        top: 10.0,
        right: 10.0,
        bottom: 10.0,
    };

    fn a5_pages_with_bleed(count: usize) -> Output {
        Output {
            pages: Some(
                (0..count)
                    .map(|_| PageImage {
                        width: 840,
                        height: 1190,
                        rgba: vec![255; 840 * 1190 * 4],
                        bleed: BLEED,
                    })
                    .collect(),
            ),
            ..Output::default()
        }
    }

    #[test]
    fn the_toolbar_switches_the_guides() {
        let mut preview = Preview::default();
        preview.show(&output(1), 2.0);

        assert_eq!(clicked(&preview, Icon::Frame), [Action::Guides]);
        preview.perform(Action::Guides);
        assert!(preview.guides());
    }

    #[test]
    fn a_click_on_a_page_with_bleed_gives_the_point_on_the_trimmed_page() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(1), 2.0);
        for _ in 0..20 {
            preview.perform(Action::ZoomOut);
        }
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        ui.click(iced::widget::Id::from("page-0".to_owned()))
            .unwrap();

        let clicks: Vec<PagePoint> = ui
            .into_messages()
            .filter_map(|action| match action {
                Action::Click(point) => Some(point),
                _ => None,
            })
            .collect();
        assert!((clicks[0].x - 200.0).abs() < 1.0, "{clicks:?}");
        assert!((clicks[0].y - 287.5).abs() < 1.0, "{clicks:?}");
    }

    #[test]
    fn the_trim_line_is_inside_the_bleed() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(1), 2.0);
        preview.perform(Action::Guides);

        let trim = preview.trim_on(0, 1.0).unwrap();

        let expected = Rectangle::new(
            Point::new(10.0 * PX_PER_PT, 10.0 * PX_PER_PT),
            Size::new(400.0 * PX_PER_PT, 575.0 * PX_PER_PT),
        );
        assert!(
            (trim.x - expected.x).abs() < 0.01
                && (trim.y - expected.y).abs() < 0.01
                && (trim.width - expected.width).abs() < 0.01
                && (trim.height - expected.height).abs() < 0.01,
            "{trim:?}"
        );
    }

    #[test]
    fn there_is_no_trim_line_without_guides_or_without_bleed() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(1), 2.0);
        assert_eq!(preview.trim_on(0, 1.0), None);

        preview.show(&output(1), 2.0);
        preview.perform(Action::Guides);
        assert_eq!(preview.trim_on(0, 1.0), None);
    }

    #[test]
    fn a_mark_on_a_page_with_bleed_is_inside_the_bleed() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(1), 2.0);
        let now = Instant::now();

        let _ = preview.show_point::<Action>(point(0, 30.0, 60.0), now);

        let (mark, _) = preview.mark_on(0, 1.0).unwrap();
        assert!((mark.x - 40.0 * PX_PER_PT).abs() < 0.01, "{mark:?}");
        assert!((mark.y - 70.0 * PX_PER_PT).abs() < 0.01, "{mark:?}");
    }

    #[test]
    fn the_scroll_target_counts_the_bleed() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(3), 2.0);
        preview.perform(scrolled(0.0, 600.0, 640.0));
        let scale = preview.frame(640.0).scale;
        let y = PADDING + 595.0 * PX_PER_PT * scale + PAGE_GAP + 110.0 * PX_PER_PT * scale;

        let target = preview.scroll_target(point(1, 50.0, 100.0)).unwrap();

        assert!((target - (y - 200.0)).abs() < 0.5, "{target} {y}");
    }

    #[test]
    fn a_shown_point_is_marked_on_its_page() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(2), 2.0);
        let now = Instant::now();

        let _ = preview.show_point::<Action>(point(1, 30.0, 60.0), now);

        assert_eq!(preview.mark_on(0, 1.0), None);
        let (mark, since) = preview.mark_on(1, 1.0).unwrap();
        assert!((mark.x - 30.0 * PX_PER_PT).abs() < 0.01, "{mark:?}");
        assert!((mark.y - 60.0 * PX_PER_PT).abs() < 0.01, "{mark:?}");
        assert_eq!(since, now);
    }

    const SHORT_LINE: &str = "The last line of a paragraph has only 3 characters.";

    fn short_line(page: usize) -> ProblemMark {
        ProblemMark {
            page,
            area: presswork_typst::Area {
                x: 30.0,
                y: 60.0,
                width: 100.0,
                height: 12.0,
            },
            text: SHORT_LINE.to_owned(),
            severity: Severity::Warning,
        }
    }

    #[test]
    fn a_problem_mark_covers_its_area_on_its_page() {
        let mut preview = Preview::default();
        preview.show(&a5_pages_with_bleed(2), 2.0);

        preview.set_problem_marks(vec![short_line(1)]);

        assert_eq!(preview.problem_marks_on(0, 1.0), []);
        let marks = preview.problem_marks_on(1, 1.0);
        assert_eq!(marks.len(), 1);
        let (bounds, mark) = marks[0];
        assert_eq!(mark, &short_line(1));
        let expected = Rectangle::new(
            Point::new(
                40.0 * PX_PER_PT - PROBLEM_PADDING,
                70.0 * PX_PER_PT - PROBLEM_PADDING,
            ),
            Size::new(
                100.0 * PX_PER_PT + 2.0 * PROBLEM_PADDING,
                12.0 * PX_PER_PT + 2.0 * PROBLEM_PADDING,
            ),
        );
        assert!(
            (bounds.x - expected.x).abs() < 0.01
                && (bounds.y - expected.y).abs() < 0.01
                && (bounds.width - expected.width).abs() < 0.01
                && (bounds.height - expected.height).abs() < 0.01,
            "{bounds:?}"
        );
    }

    #[test]
    fn the_toolbar_shows_and_hides_the_problem_marks() {
        let mut preview = Preview::default();
        preview.show(&a5_pages(1), 2.0);
        preview.set_problem_marks(vec![short_line(0)]);

        assert_eq!(
            clicked(&preview, Icon::TriangleAlert),
            [Action::ProblemMarks]
        );
        preview.perform(Action::ProblemMarks);
        assert!(!preview.shows_problem_marks());
        assert_eq!(preview.problem_marks_on(0, 1.0), []);
        preview.perform(Action::ProblemMarks);
        assert_eq!(preview.problem_marks_on(0, 1.0).len(), 1);
    }

    #[test]
    fn a_problem_mark_shows_over_the_page_in_its_color() {
        let mut preview = Preview::default();
        preview.show(&a5_page(), 2.0);
        preview.perform(Action::ZoomIn);
        preview.perform(Action::ZoomOut);
        preview.set_problem_marks(vec![short_line(0)]);
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));
        let page = ui
            .find(iced::widget::Id::from("page-0".to_owned()))
            .unwrap()
            .bounds();
        let folder = tempfile::tempdir().unwrap();
        let theme = presswork_ui::theme::light();
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(folder.path().join("problem"))
            .unwrap();

        let left = page.x + 30.0 * PX_PER_PT - PROBLEM_PADDING + PROBLEM_BORDER / 2.0;
        let middle = page.y + 66.0 * PX_PER_PT;
        let [red, green, blue, _] =
            crate::testing::pixel(folder.path(), physical(left), physical(middle));
        let warning = presswork_ui::color::LIGHT.warning.into_rgba8();
        let near = |a: u8, b: u8| a.abs_diff(b) < 40;
        assert!(
            near(red, warning[0]) && near(green, warning[1]) && near(blue, warning[2]),
            "{:?}",
            [red, green, blue]
        );
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "test coordinates are small positive numbers"
    )]
    fn physical(logical: f32) -> usize {
        (logical * 2.0) as usize
    }

    fn picture(ui: &mut iced_test::Simulator<'_, Action>) -> crate::testing::Picture {
        let folder = tempfile::tempdir().unwrap();
        let theme = presswork_ui::theme::light();
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(folder.path().join("view"))
            .unwrap();
        crate::testing::Picture::read(folder.path())
    }

    #[test]
    fn resting_on_a_problem_mark_shows_its_text_above_it() {
        let mut preview = Preview::default();
        preview.show(&a5_page(), 2.0);
        preview.set_problem_marks(vec![short_line(0)]);
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));
        let mark = ui
            .find(iced::widget::Id::from("problem-0-0".to_owned()))
            .unwrap()
            .bounds();
        let before = picture(&mut ui);
        let moved = iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: mark.center(),
        });

        ui.point_at(mark.center());
        let _ = ui.simulate([moved.clone()]);
        std::thread::sleep(TIP_DELAY + std::time::Duration::from_millis(50));
        let _ = ui.simulate([moved]);
        let after = picture(&mut ui);

        let changed = (physical(mark.x)..physical(mark.x + mark.width))
            .flat_map(|x| (physical(mark.y - 30.0)..physical(mark.y)).map(move |y| (x, y)))
            .filter(|&(x, y)| before.pixel(x, y) != after.pixel(x, y))
            .count();
        assert!(changed > 100, "{changed}");
    }
}
