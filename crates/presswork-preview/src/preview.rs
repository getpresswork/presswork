use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, image, responsive, row, scrollable, space, text};
use iced::{Element, Fill, Size};
use presswork_typst::Output;
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

const PX_PER_PT: f32 = 96.0 / 72.0;
const STEPS: [u16; 11] = [25, 50, 67, 75, 90, 100, 125, 150, 200, 300, 400];
const FULL_SIZE: u16 = 100;
const PADDING: f32 = 32.0;
const PAGE_GAP: f32 = 24.0;
const EMPTY_WIDTH: f32 = 320.0;
const PERCENT_WIDTH: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zoom {
    #[default]
    FitWidth,
    Percent(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ZoomIn,
    ZoomOut,
    FitWidth,
}

#[derive(Debug, Clone)]
struct Page {
    handle: image::Handle,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Preview {
    pages: Vec<Page>,
    zoom: Zoom,
    problem: Option<String>,
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
        };
    }

    pub fn view<'a, Message: Clone + 'a>(
        &'a self,
        empty: String,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let mut content = column![];
        if let Some(problem) = &self.problem {
            content = content.push(problem_banner(problem));
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

    fn scale(&self, width: f32) -> f32 {
        match self.zoom {
            Zoom::Percent(percent) => f32::from(percent) / f32::from(FULL_SIZE),
            Zoom::FitWidth => {
                let widest = self.pages.iter().map(|page| page.width).fold(0.0, f32::max);
                if widest > 0.0 {
                    ((width - 2.0 * PADDING) / (widest * PX_PER_PT)).clamp(0.1, 4.0)
                } else {
                    1.0
                }
            }
        }
    }

    fn pages_view<'a, Message: Clone + 'a>(
        &'a self,
        size: Size,
        on_action: impl Fn(Action) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let scale = self.scale(size.width);
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
        ]
        .spacing(gap::XXS)
        .align_y(Vertical::Center)
        .padding([gap::XS, gap::S]);
        let pages = column(self.pages.iter().map(|page| {
            container(
                image(page.handle.clone())
                    .width(page.width * PX_PER_PT * scale)
                    .height(page.height * PX_PER_PT * scale),
            )
            .style(style::paper)
            .into()
        }))
        .spacing(PAGE_GAP)
        .padding(PADDING)
        .align_x(Horizontal::Center)
        .width(Fill);
        column![toolbar, scrollable(pages).width(Fill).height(Fill)].into()
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "page images are far smaller than 2^24 pixels"
)]
fn pixels(value: u32) -> f32 {
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

fn problem_banner<'a, Message: 'a>(problem: &'a str) -> Element<'a, Message> {
    container(
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
    .style(style::badge(Tone::Danger))
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

    #[test]
    fn the_view_tells_what_to_expect_before_the_first_compilation() {
        let preview = Preview::default();
        let mut ui = simulator(preview.view(EMPTY.to_owned(), |action| action));

        assert!(ui.find(EMPTY).is_ok());
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
}
