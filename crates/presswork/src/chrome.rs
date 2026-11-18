use std::time::Duration;

use iced::alignment::Vertical;
use iced::widget::{button, column, container, mouse_area, row, space, stack, text, tooltip};
use iced::{Element, Fill, Length, Size, Task, Theme, mouse, padding, window};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font_size, size, space as gap, style};

use crate::app::Message;
use crate::i18n::{FluentArgs, Localizer};

pub(crate) const MINIMUM_SIZE: Size = Size::new(720.0, 480.0);
pub(crate) const TOP_BAR: &str = "top-bar";
const TRAFFIC_LIGHTS: f32 = 76.0;
const OUTLINE: bool = cfg!(target_os = "linux");
const TIP_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Controls {
    Native,
    Custom,
}

impl Controls {
    pub(crate) const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::Native
    } else {
        Self::Custom
    };

    pub(crate) fn inset(self) -> f32 {
        match self {
            Self::Native => TRAFFIC_LIGHTS,
            Self::Custom => gap::S,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Drag,
    Resize(Edge),
    Minimize,
    ToggleMaximize,
    SystemMenu,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl Edge {
    fn direction(self) -> window::Direction {
        match self {
            Self::North => window::Direction::North,
            Self::South => window::Direction::South,
            Self::East => window::Direction::East,
            Self::West => window::Direction::West,
            Self::NorthEast => window::Direction::NorthEast,
            Self::NorthWest => window::Direction::NorthWest,
            Self::SouthEast => window::Direction::SouthEast,
            Self::SouthWest => window::Direction::SouthWest,
        }
    }

    fn interaction(self) -> mouse::Interaction {
        match self {
            Self::North | Self::South => mouse::Interaction::ResizingVertically,
            Self::East | Self::West => mouse::Interaction::ResizingHorizontally,
            Self::NorthWest | Self::SouthEast => mouse::Interaction::ResizingDiagonallyDown,
            Self::NorthEast | Self::SouthWest => mouse::Interaction::ResizingDiagonallyUp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Frame {
    pub(crate) id: window::Id,
    pub(crate) controls: Controls,
    pub(crate) maximized: bool,
}

pub(crate) fn settings(size: Size, controls: Controls) -> window::Settings {
    window::Settings {
        size,
        min_size: Some(MINIMUM_SIZE),
        decorations: controls == Controls::Native,
        exit_on_close_request: false,
        icon: crate::icons::window_icon(),
        platform_specific: platform(),
        ..window::Settings::default()
    }
}

#[cfg(target_os = "macos")]
fn platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        title_hidden: true,
        titlebar_transparent: true,
        fullsize_content_view: true,
    }
}

#[cfg(target_os = "windows")]
fn platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        undecorated_shadow: true,
        corner_preference: window::settings::platform::CornerPreference::Round,
        ..window::settings::PlatformSpecific::default()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        application_id: crate::APPLICATION_ID.to_owned(),
        ..window::settings::PlatformSpecific::default()
    }
}

pub(crate) fn perform(id: window::Id, action: Action) -> Task<Message> {
    match action {
        Action::Drag => window::drag(id),
        Action::Resize(edge) => window::drag_resize(id, edge.direction()),
        Action::Minimize => window::minimize(id, true),
        Action::ToggleMaximize => window::toggle_maximize(id),
        Action::SystemMenu => window::show_system_menu(id),
        Action::Close => window::close(id),
    }
}

pub(crate) fn bar<'a>(
    frame: Frame,
    localizer: &Localizer,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = container(content)
        .width(Fill)
        .height(Fill)
        .align_y(Vertical::Center);
    let inset = padding::left(frame.controls.inset());
    let bar = match frame.controls {
        Controls::Custom => row![content, window_buttons(frame, localizer)].padding(inset),
        Controls::Native => row![content].padding(inset.right(gap::S)),
    };
    let id = frame.id;
    let area = mouse_area(
        container(bar.height(size::TOP_BAR))
            .id(TOP_BAR)
            .width(Fill)
            .style(style::chrome),
    )
    .on_press(Message::Window(id, Action::Drag))
    .on_double_click(Message::Window(id, Action::ToggleMaximize));
    match frame.controls {
        Controls::Custom => area
            .on_right_press(Message::Window(id, Action::SystemMenu))
            .into(),
        Controls::Native => area.into(),
    }
}

pub(crate) fn shortcut(localizer: &Localizer, key: char) -> String {
    let mut args = FluentArgs::new();
    args.set("key", key.to_string());
    let id = if cfg!(target_os = "macos") {
        "shortcut-mac"
    } else {
        "shortcut"
    };
    localizer.format(id, &args)
}

pub(crate) fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    words: String,
    position: tooltip::Position,
) -> Element<'a, Message> {
    let card = container(text(words).size(font_size::CAPTION))
        .padding([gap::XXS, gap::S])
        .style(style::popup);
    tooltip(content, card, position)
        .gap(gap::XS)
        .delay(TIP_DELAY)
        .into()
}

pub(crate) fn window<'a>(
    frame: Frame,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    framed(frame, OUTLINE, content.into())
}

fn framed<'a>(frame: Frame, outline: bool, content: Element<'a, Message>) -> Element<'a, Message> {
    if frame.controls == Controls::Native || frame.maximized {
        return content;
    }
    let content: Element<'a, Message> = if outline {
        container(content).padding(1).style(style::outline).into()
    } else {
        content
    };
    stack![content, edges(frame.id)].into()
}

fn window_buttons<'a>(frame: Frame, localizer: &Localizer) -> Element<'a, Message> {
    let (symbol, words) = if frame.maximized {
        (Icon::Copy, "window-restore")
    } else {
        (Icon::Square, "window-maximize")
    };
    row![
        window_button(
            frame.id,
            "window-minimize",
            Icon::Minus,
            localizer.text("window-minimize"),
            Action::Minimize
        ),
        window_button(
            frame.id,
            "window-maximize",
            symbol,
            localizer.text(words),
            Action::ToggleMaximize
        ),
        window_button(
            frame.id,
            "window-close",
            Icon::X,
            localizer.text("window-close"),
            Action::Close
        ),
    ]
    .into()
}

fn window_button<'a>(
    id: window::Id,
    name: &'static str,
    symbol: Icon,
    words: String,
    action: Action,
) -> Element<'a, Message> {
    let look: fn(&Theme, button::Status) -> button::Style = if action == Action::Close {
        style::close_button
    } else {
        style::window_button
    };
    let control = button(container(icon(symbol).size(size::ICON_SMALL)).center(Fill))
        .width(size::WINDOW_BUTTON)
        .height(size::TOP_BAR)
        .padding(0)
        .style(look)
        .on_press(Message::Window(id, action));
    container(tip(control, words, tooltip::Position::Bottom))
        .id(name)
        .into()
}

fn edges<'a>(id: window::Id) -> Element<'a, Message> {
    let edge = |edge: Edge, width: Length, height: Length| -> Element<'a, Message> {
        mouse_area(space().width(width).height(height))
            .interaction(edge.interaction())
            .on_press(Message::Window(id, Action::Resize(edge)))
            .into()
    };
    let thickness = Length::Fixed(size::RESIZE_EDGE);
    column![
        row![
            edge(Edge::NorthWest, thickness, thickness),
            edge(Edge::North, Length::Fill, thickness),
            edge(Edge::NorthEast, thickness, thickness),
        ],
        row![
            edge(Edge::West, thickness, Length::Fill),
            space::horizontal(),
            edge(Edge::East, thickness, Length::Fill),
        ]
        .height(Length::Fill),
        row![
            edge(Edge::SouthWest, thickness, thickness),
            edge(Edge::South, Length::Fill, thickness),
            edge(Edge::SouthEast, thickness, thickness),
        ],
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{container, space, text};
    use iced::{Element, Fill, Point, Settings, Size, mouse, window};
    use iced_test::simulator::{self, simulator};
    use iced_test::{Simulator, selector};
    use presswork_ui::icon::Icon;

    use super::*;
    use crate::app::Message;
    use crate::i18n::Localizer;

    const AREA: (f32, f32) = (800.0, 300.0);

    fn frame(controls: Controls, maximized: bool) -> Frame {
        Frame {
            id: window::Id::unique(),
            controls,
            maximized,
        }
    }

    fn bar_with_content(frame: Frame) -> Element<'static, Message> {
        bar(
            frame,
            &Localizer::english(),
            container(text("Content")).id("content"),
        )
    }

    fn filled() -> Element<'static, Message> {
        container(space().width(Fill).height(Fill))
            .id("content")
            .into()
    }

    fn press(element: Element<'_, Message>, point: Point, button: mouse::Button) -> Vec<Message> {
        let mut ui = Simulator::with_size(Settings::default(), AREA, element);
        ui.point_at(point);
        let _ = ui.simulate([
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)),
        ]);
        ui.into_messages().collect()
    }

    fn sends(messages: &[Message], id: window::Id, action: Action) -> bool {
        messages
            .iter()
            .any(|message| matches!(message, Message::Window(target, sent) if *target == id && *sent == action))
    }

    #[test]
    fn window_buttons_send_their_actions() {
        for (button, action) in [
            ("window-minimize", Action::Minimize),
            ("window-maximize", Action::ToggleMaximize),
            ("window-close", Action::Close),
        ] {
            let frame = frame(Controls::Custom, false);
            let mut ui = simulator(bar_with_content(frame));

            ui.click(selector::id(button)).unwrap();

            let messages: Vec<Message> = ui.into_messages().collect();
            assert_eq!(messages.len(), 1, "{button}: {messages:?}");
            assert!(sends(&messages, frame.id, action), "{button}: {messages:?}");
        }
    }

    #[test]
    fn the_bar_is_thirty_two_pixels_high() {
        let mut ui = simulator(bar_with_content(frame(Controls::Custom, false)));

        let close = ui.find(selector::id("window-close")).unwrap().bounds();

        assert!((close.height - 32.0).abs() < 0.5, "{close:?}");
    }

    #[test]
    fn native_controls_leave_space_for_the_traffic_lights() {
        let mut ui = simulator(bar_with_content(frame(Controls::Native, false)));

        assert!(ui.find(selector::id("window-close")).is_err());
        let content = ui.find(selector::id("content")).unwrap().bounds();
        assert!(content.x >= 76.0, "{content:?}");
    }

    #[test]
    fn a_press_on_an_empty_area_drags_the_window() {
        let frame = frame(Controls::Custom, false);

        let messages = press(
            bar_with_content(frame),
            Point::new(400.0, 16.0),
            mouse::Button::Left,
        );

        assert!(sends(&messages, frame.id, Action::Drag), "{messages:?}");
    }

    #[test]
    fn a_double_click_on_an_empty_area_maximizes_the_window() {
        let frame = frame(Controls::Custom, false);
        let mut ui = Simulator::with_size(Settings::default(), AREA, bar_with_content(frame));

        ui.point_at(Point::new(400.0, 16.0));
        let _ = ui.simulate(simulator::click());
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            sends(&messages, frame.id, Action::ToggleMaximize),
            "{messages:?}"
        );
    }

    #[test]
    fn a_right_click_on_an_empty_area_opens_the_window_menu() {
        let custom = frame(Controls::Custom, false);
        let native = frame(Controls::Native, false);
        let at = Point::new(400.0, 16.0);

        let messages = press(bar_with_content(custom), at, mouse::Button::Right);
        assert!(
            sends(&messages, custom.id, Action::SystemMenu),
            "{messages:?}"
        );

        let messages = press(bar_with_content(native), at, mouse::Button::Right);
        assert!(
            !sends(&messages, native.id, Action::SystemMenu),
            "{messages:?}"
        );
    }

    #[test]
    fn a_maximized_window_shows_the_restore_symbol() {
        let square = Icon::Square.unicode().to_string();
        let restore = Icon::Copy.unicode().to_string();

        let mut ui = simulator(bar_with_content(frame(Controls::Custom, true)));
        assert!(ui.find(restore.clone()).is_ok());
        assert!(ui.find(square.clone()).is_err());

        let mut ui = simulator(bar_with_content(frame(Controls::Custom, false)));
        assert!(ui.find(square).is_ok());
        assert!(ui.find(restore).is_err());
    }

    #[test]
    fn edges_and_corners_resize_the_window() {
        let frame = frame(Controls::Custom, false);
        for (point, edge) in [
            (Point::new(400.0, 2.0), Edge::North),
            (Point::new(400.0, 298.0), Edge::South),
            (Point::new(2.0, 150.0), Edge::West),
            (Point::new(798.0, 150.0), Edge::East),
            (Point::new(2.0, 2.0), Edge::NorthWest),
            (Point::new(798.0, 2.0), Edge::NorthEast),
            (Point::new(2.0, 298.0), Edge::SouthWest),
            (Point::new(798.0, 298.0), Edge::SouthEast),
        ] {
            let messages = press(framed(frame, false, filled()), point, mouse::Button::Left);

            assert_eq!(messages.len(), 1, "{edge:?}: {messages:?}");
            assert!(
                sends(&messages, frame.id, Action::Resize(edge)),
                "{edge:?}: {messages:?}"
            );
        }

        let messages = press(
            framed(frame, false, filled()),
            Point::new(400.0, 150.0),
            mouse::Button::Left,
        );
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn maximized_and_native_windows_have_no_resize_edges() {
        for frame in [
            frame(Controls::Custom, true),
            frame(Controls::Native, false),
        ] {
            let messages = press(
                framed(frame, true, filled()),
                Point::new(400.0, 2.0),
                mouse::Button::Left,
            );

            assert!(messages.is_empty(), "{frame:?}: {messages:?}");
        }
    }

    #[test]
    fn the_outline_takes_one_pixel_at_each_side() {
        let frame = frame(Controls::Custom, false);
        let content = |outline| {
            let mut ui =
                Simulator::with_size(Settings::default(), AREA, framed(frame, outline, filled()));
            ui.find(selector::id("content")).unwrap().bounds()
        };

        let outlined = content(true);
        assert_eq!((outlined.x, outlined.y), (1.0, 1.0));
        assert_eq!(
            (outlined.width, outlined.height),
            (AREA.0 - 2.0, AREA.1 - 2.0)
        );
        let plain = content(false);
        assert_eq!((plain.x, plain.y), (0.0, 0.0));
    }

    #[test]
    fn a_maximized_window_has_no_outline() {
        let mut ui = Simulator::with_size(
            Settings::default(),
            AREA,
            framed(frame(Controls::Custom, true), true, filled()),
        );

        let content = ui.find(selector::id("content")).unwrap().bounds();

        assert_eq!((content.x, content.y), (0.0, 0.0));
    }

    #[test]
    fn settings_hide_the_system_frame_only_for_custom_controls() {
        let size = Size::new(900.0, 700.0);
        let custom = settings(size, Controls::Custom);
        let native = settings(size, Controls::Native);

        assert!(!custom.decorations);
        assert!(native.decorations);
        assert_eq!(custom.size, size);
        assert_eq!(custom.min_size, Some(MINIMUM_SIZE));
        assert!(!custom.exit_on_close_request);
        assert!(!native.exit_on_close_request);
    }

    #[test]
    fn windows_show_the_icon_of_the_application() {
        let settings = settings(Size::new(900.0, 700.0), Controls::Custom);

        let (pixels, size) = settings.icon.unwrap().into_raw();
        assert_eq!(size, Size::new(64, 64));
        assert_eq!(pixels.len(), 64 * 64 * 4);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn linux_windows_have_the_application_id() {
        let settings = settings(Size::new(900.0, 700.0), Controls::Custom);

        assert_eq!(
            settings.platform_specific.application_id,
            crate::APPLICATION_ID
        );
    }

    #[test]
    fn shortcuts_use_the_modifier_of_the_system() {
        let expected = if cfg!(target_os = "macos") {
            "Cmd+O"
        } else {
            "Ctrl+O"
        };

        assert_eq!(shortcut(&Localizer::english(), 'O'), expected);
    }

    #[test]
    fn only_macos_keeps_the_system_buttons() {
        assert_eq!(
            Controls::CURRENT == Controls::Native,
            cfg!(target_os = "macos")
        );
    }
}
