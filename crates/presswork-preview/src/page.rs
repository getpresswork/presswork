use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Shell, mouse, overlay};
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, window,
};

pub(crate) const MARK_TIME: Duration = Duration::from_millis(1500);
const MARK_RADIUS: f32 = 9.0;
const MARK_BORDER: f32 = 2.0;
const MARK_FILL_ALPHA: f32 = 0.2;
const TRIM_WIDTH: f32 = 1.0;
const GRID_ALPHA: f32 = 0.45;

pub(crate) struct PageArea<'a, Message> {
    content: Element<'a, Message>,
    on_click: Box<dyn Fn(Point) -> Message + 'a>,
    mark: Option<(Point, Instant)>,
    trim: Option<Rectangle>,
    grid: Vec<Rectangle>,
}

pub(crate) fn page_area<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_click: impl Fn(Point) -> Message + 'a,
) -> PageArea<'a, Message> {
    PageArea {
        content: content.into(),
        on_click: Box::new(on_click),
        mark: None,
        trim: None,
        grid: Vec::new(),
    }
}

impl<Message> PageArea<'_, Message> {
    pub(crate) fn mark(mut self, mark: Option<(Point, Instant)>) -> Self {
        self.mark = mark;
        self
    }

    pub(crate) fn trim(mut self, trim: Option<Rectangle>) -> Self {
        self.trim = trim;
        self
    }

    pub(crate) fn grid(mut self, grid: Vec<Rectangle>) -> Self {
        self.grid = grid;
        self
    }
}

fn shows(since: Instant, now: Instant) -> bool {
    now.saturating_duration_since(since) < MARK_TIME
}

impl<Message> Widget<Message, Theme, Renderer> for PageArea<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if shell.is_event_captured() {
            return;
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(layout.bounds()) {
                    shell.publish((self.on_click)(position));
                    shell.capture_event();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some((_, since)) = self.mark
                    && shows(since, *now)
                {
                    shell.request_redraw_at(since + MARK_TIME);
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let bounds = layout.bounds();
        let mark = self.mark.filter(|(_, since)| shows(*since, Instant::now()));
        if mark.is_none() && self.trim.is_none() && self.grid.is_empty() {
            return;
        }
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let colors = presswork_ui::theme::colors(theme);
        renderer.with_layer(clip, |renderer| {
            for line in &self.grid {
                renderer.fill_quad(
                    Quad {
                        bounds: *line + Vector::new(bounds.x, bounds.y),
                        ..Quad::default()
                    },
                    Color {
                        a: GRID_ALPHA,
                        ..colors.guide
                    },
                );
            }
            if let Some(trim) = self.trim {
                renderer.fill_quad(
                    Quad {
                        bounds: trim + Vector::new(bounds.x, bounds.y),
                        border: Border {
                            color: colors.guide,
                            width: TRIM_WIDTH,
                            radius: 0.0.into(),
                        },
                        ..Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            }
            if let Some((point, _)) = mark {
                let corner = Point::new(
                    bounds.x + point.x - MARK_RADIUS,
                    bounds.y + point.y - MARK_RADIUS,
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            corner,
                            Size::new(2.0 * MARK_RADIUS, 2.0 * MARK_RADIUS),
                        ),
                        border: Border {
                            color: colors.accent,
                            width: MARK_BORDER,
                            radius: MARK_RADIUS.into(),
                        },
                        ..Quad::default()
                    },
                    Color {
                        a: MARK_FILL_ALPHA,
                        ..colors.accent
                    },
                );
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<PageArea<'a, Message>> for Element<'a, Message> {
    fn from(area: PageArea<'a, Message>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::space;
    use iced_test::simulator;

    use super::*;
    use crate::testing::pixel;

    #[test]
    fn a_click_gives_the_point_in_the_area() {
        let area = page_area(space().width(200).height(100), |point| point);
        let mut ui = simulator(area);

        ui.point_at(Point::new(50.0, 20.0));
        let _ = ui.simulate(simulator::click());

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            [Point::new(50.0, 20.0)]
        );
    }

    #[test]
    fn a_click_outside_the_area_gives_nothing() {
        let area = page_area(space().width(200).height(100), |point| point);
        let mut ui = simulator(area);

        ui.point_at(Point::new(300.0, 20.0));
        let _ = ui.simulate(simulator::click());

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn the_mark_shows_for_a_short_time() {
        let now = Instant::now();

        assert!(shows(now, now + Duration::from_millis(100)));
        assert!(!shows(now, now + MARK_TIME));
    }

    #[test]
    fn the_mark_shows_over_the_page_image() {
        let page = iced::widget::image(iced::widget::image::Handle::from_rgba(
            40,
            40,
            vec![255; 40 * 40 * 4],
        ))
        .width(40)
        .height(40);
        let area =
            page_area(page, |point| point).mark(Some((Point::new(20.0, 20.0), Instant::now())));
        let mut ui = simulator(area);
        let folder = tempfile::tempdir().unwrap();

        let snapshot = ui.snapshot(&presswork_ui::theme::light()).unwrap();
        snapshot.matches_image(folder.path().join("mark")).unwrap();

        assert_ne!(pixel(folder.path(), 40, 40), [255, 255, 255, 255]);
    }

    fn marked_center(rgba: [u8; 4]) -> [u8; 4] {
        let page = iced::widget::image(iced::widget::image::Handle::from_rgba(
            40,
            40,
            rgba.repeat(40 * 40),
        ))
        .width(40)
        .height(40);
        let area =
            page_area(page, |point| point).mark(Some((Point::new(20.0, 20.0), Instant::now())));
        let mut ui = simulator(area);
        let folder = tempfile::tempdir().unwrap();
        let theme = presswork_ui::theme::light();
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let snapshot = ui.snapshot(&theme).unwrap();
        snapshot.matches_image(folder.path().join("mark")).unwrap();
        pixel(folder.path(), 40, 40)
    }

    #[test]
    fn the_trim_line_shows_over_the_page_image() {
        let page = iced::widget::image(iced::widget::image::Handle::from_rgba(
            40,
            40,
            vec![255; 40 * 40 * 4],
        ))
        .width(40)
        .height(40);
        let trim = Rectangle::new(Point::new(10.0, 10.0), Size::new(20.0, 20.0));
        let area = page_area(page, |point| point).trim(Some(trim));
        let mut ui = simulator(area);
        let folder = tempfile::tempdir().unwrap();
        let theme = presswork_ui::theme::light();
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(folder.path().join("trim"))
            .unwrap();

        let [red, green, blue, _] = pixel(folder.path(), 20, 40);
        let guide = presswork_ui::color::LIGHT.guide.into_rgba8();
        let near = |a: u8, b: u8| a.abs_diff(b) < 40;
        assert!(
            near(red, guide[0]) && near(green, guide[1]) && near(blue, guide[2]),
            "{:?}",
            [red, green, blue]
        );
    }

    #[test]
    fn a_grid_line_shows_over_the_page_image() {
        let page = iced::widget::image(iced::widget::image::Handle::from_rgba(
            40,
            40,
            vec![255; 40 * 40 * 4],
        ))
        .width(40)
        .height(40);
        let line = Rectangle::new(Point::new(0.0, 20.0), Size::new(40.0, 1.0));
        let area = page_area(page, |point| point).grid(vec![line]);
        let mut ui = simulator(area);
        let folder = tempfile::tempdir().unwrap();
        let theme = presswork_ui::theme::light();
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(folder.path().join("grid"))
            .unwrap();

        assert_ne!(pixel(folder.path(), 40, 40), [255, 255, 255, 255]);
        assert_eq!(pixel(folder.path(), 40, 20), [255, 255, 255, 255]);
    }

    #[test]
    fn the_page_shows_through_the_mark() {
        let on_white = marked_center([255, 255, 255, 255]);
        let on_black = marked_center([0, 0, 0, 255]);

        assert_ne!(on_white, on_black);
    }
}
