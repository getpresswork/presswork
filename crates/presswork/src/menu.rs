use iced::alignment::Vertical;
use iced::keyboard::{self, key};
use iced::widget::{
    button, column, container, mouse_area, opaque, pin, row, rule, space, stack, text,
};
use iced::{Element, Event, Fill, event, window};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font_size, size, space as gap, style};

use crate::app::Message;
use crate::chrome::{self, Controls};
use crate::i18n::Localizer;

const MENU: &str = "presswork-menu-items";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    OpenFolder,
    Save,
    Connect,
    Typst,
    Logs,
    Quit,
}

impl Item {
    fn text_id(self) -> &'static str {
        match self {
            Self::OpenFolder => "menu-open-folder",
            Self::Save => "menu-save",
            Self::Connect => "menu-connect",
            Self::Typst => "menu-typst",
            Self::Logs => "menu-logs",
            Self::Quit => "menu-quit",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::OpenFolder => Icon::FolderOpen,
            Self::Save => Icon::Save,
            Self::Connect => Icon::Server,
            Self::Typst => Icon::Package,
            Self::Logs => Icon::ScrollText,
            Self::Quit => Icon::Power,
        }
    }

    fn key(self) -> Option<char> {
        match self {
            Self::OpenFolder => Some('O'),
            Self::Save => Some('S'),
            Self::Quit => Some('Q'),
            Self::Connect | Self::Typst | Self::Logs => None,
        }
    }
}

pub(crate) fn view<'a>(
    localizer: &Localizer,
    can_save: bool,
    controls: Controls,
) -> Element<'a, Message> {
    let items = column![
        entry(localizer, Item::OpenFolder, true),
        entry(localizer, Item::Save, can_save),
        separator(),
        entry(localizer, Item::Connect, true),
        separator(),
        entry(localizer, Item::Typst, true),
        entry(localizer, Item::Logs, true),
        separator(),
        entry(localizer, Item::Quit, true),
    ];
    let card = container(items)
        .id(MENU)
        .padding(gap::XS)
        .width(size::MENU)
        .style(style::popup);
    stack![
        mouse_area(space().width(Fill).height(Fill))
            .on_press(Message::CloseMenu)
            .on_right_press(Message::CloseMenu),
        pin(opaque(card))
            .x(controls.inset())
            .y(size::TOP_BAR + gap::XXS),
    ]
    .into()
}

pub(crate) fn menu_key(
    event: Event,
    _status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key: pressed, .. }) = event else {
        return None;
    };
    (pressed == keyboard::Key::Named(key::Named::Escape)).then_some(Message::CloseMenu)
}

fn entry<'a>(localizer: &Localizer, item: Item, enabled: bool) -> Element<'a, Message> {
    let mut content = row![
        icon(item.icon()).size(size::ICON_SMALL),
        text(localizer.text(item.text_id())),
        space::horizontal(),
    ]
    .spacing(gap::S)
    .align_y(Vertical::Center);
    if let Some(key) = item.key() {
        let keys = text(chrome::shortcut(localizer, key)).size(font_size::SMALL);
        content = content.push(if enabled {
            keys.style(style::muted)
        } else {
            keys
        });
    }
    button(content)
        .width(Fill)
        .padding([gap::XS + gap::XXS, gap::S])
        .style(style::menu_item)
        .on_press_maybe(enabled.then_some(Message::MenuItem(item)))
        .into()
}

fn separator<'a>() -> Element<'a, Message> {
    container(rule::horizontal(1).style(style::hairline))
        .padding([gap::XS, gap::XXS])
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key;
    use iced::{Element, Point, Settings, event, window};
    use iced_test::simulator::{self, simulator};
    use iced_test::{Simulator, selector};
    use presswork_ui::size;

    use super::*;
    use crate::app::Message;
    use crate::chrome::Controls;
    use crate::i18n::Localizer;
    use crate::testing::key_press;

    const LABELS: [(&str, Item); 6] = [
        ("Open folder…", Item::OpenFolder),
        ("Save", Item::Save),
        ("Connect to a server…", Item::Connect),
        ("Typst versions", Item::Typst),
        ("Logs", Item::Logs),
        ("Quit", Item::Quit),
    ];

    fn menu(can_save: bool, controls: Controls) -> Element<'static, Message> {
        view(&Localizer::english(), can_save, controls)
    }

    #[test]
    fn the_menu_shows_each_command() {
        let mut ui = simulator(menu(true, Controls::Custom));

        for (label, _) in LABELS {
            assert!(ui.find(label).is_ok(), "{label} is not in the menu");
        }
    }

    #[test]
    fn the_menu_shows_the_keys_of_the_commands() {
        let modifier = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        let mut ui = simulator(menu(true, Controls::Custom));

        for letter in ['O', 'S', 'Q'] {
            let keys = format!("{modifier}+{letter}");
            assert!(ui.find(keys.as_str()).is_ok(), "{keys} is not in the menu");
        }
    }

    #[test]
    fn each_item_sends_its_command() {
        for (label, item) in LABELS {
            let mut ui = simulator(menu(true, Controls::Custom));

            ui.click(label).unwrap();

            let messages: Vec<Message> = ui.into_messages().collect();
            assert!(
                matches!(messages.as_slice(), [Message::MenuItem(sent)] if *sent == item),
                "{label}: {messages:?}"
            );
        }
    }

    #[test]
    fn save_is_not_available_without_a_project() {
        let mut ui = simulator(menu(false, Controls::Custom));

        ui.click("Save").unwrap();

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn a_click_outside_of_the_menu_closes_it() {
        let mut ui = Simulator::with_size(
            Settings::default(),
            (800.0, 600.0),
            menu(true, Controls::Custom),
        );

        ui.point_at(Point::new(600.0, 400.0));
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            matches!(messages.as_slice(), [Message::CloseMenu]),
            "{messages:?}"
        );
    }

    #[test]
    fn the_menu_opens_under_the_mark() {
        for controls in [Controls::Custom, Controls::Native] {
            let mut ui = simulator(menu(true, controls));

            let card = ui.find(selector::id(MENU)).unwrap().bounds();

            assert!(
                (card.x - controls.inset()).abs() < 0.5,
                "{controls:?}: {card:?}"
            );
            assert!(card.y > size::TOP_BAR, "{card:?}");
        }
    }

    #[test]
    fn escape_closes_the_menu() {
        let id = window::Id::unique();

        assert!(matches!(
            menu_key(key_press(key::Named::Escape), event::Status::Captured, id),
            Some(Message::CloseMenu)
        ));
        assert!(menu_key(key_press(key::Named::Enter), event::Status::Ignored, id).is_none());
    }
}
