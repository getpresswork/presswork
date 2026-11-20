use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{AbsoluteOffset, Direction, Scrollbar};
use iced::widget::{
    button, column, container, image, operation, responsive, row, scrollable, space, text,
};
use iced::{Element, Fill, Point, Size, Task};
use presswork_typst::{Output, PagePoint};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zoom {
    #[default]
    FitWidth,
    Percent(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    ZoomIn,
    ZoomOut,
    FitWidth,
    ShowProblem,
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
    problem: Option<String>,
    detached: bool,
    viewport: Option<Viewport>,
    mark: Option<(PagePoint, Instant)>,
}

impl Preview {
    pub fn show(&mut self, output: &Output, pixel_per_pt: f32) {
        if let Some(pages) = &output.pages {
            self.pages = pages
                .iter()
                .map(|page| Page {
                    handle: image::Handle::from_rgba(page.width, page.height, page.rgba.clone()),
                    width: pixels(page.width) / pixel_per_pt,
                    height: pixels(page.height) / pixel_per_pt,
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
            Action::ShowProblem | Action::Detach | Action::Attach | Action::Click(_) => return,
        };
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
        let above = self.pages.get(..point.page)?;
        self.pages.get(point.page)?;
        let Frame { scale, .. } = self.frame(viewport.width);
        let pages_above: f32 = above
            .iter()
            .map(|page| page.height * PX_PER_PT * scale + PAGE_GAP)
            .sum();
        let y = PADDING + pages_above + to_f32(point.y) * PX_PER_PT * scale;
        let margin = viewport.height * SHOWN_MARGIN;
        if y >= viewport.top + margin && y <= viewport.top + viewport.height - margin {
            return None;
        }
        Some((y - viewport.height * SHOWN_AT).max(0.0))
    }

    fn mark_on(&self, page: usize, scale: f32) -> Option<(Point, Instant)> {
        let (point, since) = self.mark?;
        (point.page == page).then(|| {
            let ratio = PX_PER_PT * scale;
            (
                Point::new(to_f32(point.x) * ratio, to_f32(point.y) * ratio),
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
        let widest = self.pages.iter().map(|page| page.width).fold(0.0, f32::max) * PX_PER_PT;
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
        let Frame { scale, width } = self.frame(size.width);
        let percent = scale * 100.0;
        let toolbar = row![
            space().width(Fill),
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
        let pages = column(self.pages.iter().enumerate().map(|(index, page)| {
            let content = container(
                image(page.handle.clone())
                    .width(page.width * PX_PER_PT * scale)
                    .height(page.height * PX_PER_PT * scale),
            )
            .style(style::paper)
            .id(format!("page-{index}"));
            page_area(content, move |point| {
                on_action(Action::Click(PagePoint {
                    page: index,
                    x: f64::from(point.x / (PX_PER_PT * scale)),
                    y: f64::from(point.y / (PX_PER_PT * scale)),
                }))
            })
            .mark(self.mark_on(index, scale))
            .into()
        }))
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
        column![toolbar, pages].into()
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
}
