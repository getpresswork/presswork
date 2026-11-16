use iced::keyboard::{self, key};
use iced::widget::{button, column, container, opaque, row, space, text};
use iced::{Element, Event, Fill, Theme, event, window};
use presswork_ui::{font, font_size, size, space as gap, style};

use crate::app::Message;
use crate::i18n::{FluentArgs, Localizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    Save,
    Discard,
    Cancel,
}

pub(crate) fn view<'a>(localizer: &Localizer, file: &str) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("file", file.to_owned());
    let buttons = row![
        space::horizontal(),
        answer(
            localizer.text("unsaved-cancel"),
            Answer::Cancel,
            style::secondary
        ),
        answer(
            localizer.text("unsaved-discard"),
            Answer::Discard,
            style::secondary
        ),
        answer(localizer.text("unsaved-save"), Answer::Save, style::primary),
    ]
    .spacing(gap::S);
    let card = container(
        column![
            text(localizer.format("unsaved-title", &args))
                .font(font::UI_SEMIBOLD)
                .size(font_size::LARGE),
            text(localizer.text("unsaved-body")).style(style::muted),
            space().height(gap::XS),
            buttons,
        ]
        .spacing(gap::S),
    )
    .padding(gap::L)
    .width(size::DIALOG)
    .style(style::popup);
    opaque(container(card).center(Fill).style(style::scrim))
}

pub(crate) fn dialog_key(
    event: Event,
    _status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key: pressed, .. }) = event else {
        return None;
    };
    match pressed {
        keyboard::Key::Named(key::Named::Escape) => Some(Message::Unsaved(Answer::Cancel)),
        keyboard::Key::Named(key::Named::Enter) => Some(Message::Unsaved(Answer::Save)),
        _ => None,
    }
}

fn answer<'a>(
    words: String,
    answer: Answer,
    look: fn(&Theme, button::Status) -> button::Style,
) -> Element<'a, Message> {
    button(text(words))
        .padding([gap::XS + gap::XXS, gap::M])
        .style(look)
        .on_press(Message::Unsaved(answer))
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key;
    use iced::{Element, Point, Settings, event, window};
    use iced_test::Simulator;
    use iced_test::simulator::{self, simulator};

    use super::*;
    use crate::app::Message;
    use crate::i18n::Localizer;
    use crate::testing::key_press;

    fn dialog() -> Element<'static, Message> {
        view(&Localizer::english(), "main.typ")
    }

    #[test]
    fn the_dialog_names_the_file_and_the_answers() {
        let mut ui = simulator(dialog());

        for words in [
            "Save the changes to main.typ?",
            "If you do not save, the changes are lost.",
            "Save",
            "Do not save",
            "Cancel",
        ] {
            assert!(ui.find(words).is_ok(), "{words} is not in the dialog");
        }
    }

    #[test]
    fn each_button_sends_its_answer() {
        for (label, answer) in [
            ("Save", Answer::Save),
            ("Do not save", Answer::Discard),
            ("Cancel", Answer::Cancel),
        ] {
            let mut ui = simulator(dialog());

            ui.click(label).unwrap();

            let messages: Vec<Message> = ui.into_messages().collect();
            assert!(
                matches!(messages.as_slice(), [Message::Unsaved(sent)] if *sent == answer),
                "{label}: {messages:?}"
            );
        }
    }

    #[test]
    fn a_click_beside_the_dialog_does_nothing() {
        let mut ui = Simulator::with_size(Settings::default(), (800.0, 600.0), dialog());

        ui.point_at(Point::new(20.0, 20.0));
        let _ = ui.simulate(simulator::click());

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn escape_cancels_and_enter_saves() {
        let id = window::Id::unique();

        assert!(matches!(
            dialog_key(key_press(key::Named::Escape), event::Status::Captured, id),
            Some(Message::Unsaved(Answer::Cancel))
        ));
        assert!(matches!(
            dialog_key(key_press(key::Named::Enter), event::Status::Captured, id),
            Some(Message::Unsaved(Answer::Save))
        ));
        assert!(dialog_key(key_press(key::Named::Tab), event::Status::Ignored, id).is_none());
    }
}
