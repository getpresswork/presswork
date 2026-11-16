use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Fill};
use jiff::tz::TimeZone;
use presswork_api::live::ChatMessage;
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font, font_size, size, space as gap, style};

use crate::i18n::{FluentArgs, Localizer};

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 5] = [
    "chat-toggle",
    "chat-title",
    "chat-empty",
    "chat-placeholder",
    "chat-send",
];

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Toggle,
    Close,
    Draft(String),
    Send,
}

#[derive(Debug, Default)]
pub(crate) struct ChatPanel {
    open: bool,
    draft: String,
}

impl ChatPanel {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    pub(crate) fn clear_draft(&mut self) {
        self.draft.clear();
    }

    pub(crate) fn update(&mut self, message: Message) -> Option<String> {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Close => self.open = false,
            Message::Draft(draft) => self.draft = draft,
            Message::Send => {
                let text = self.draft.trim();
                return (!text.is_empty()).then(|| text.to_owned());
            }
        }
        None
    }

    pub(crate) fn view<'a>(
        &'a self,
        organization: &str,
        messages: &'a [ChatMessage],
        localizer: &Localizer,
    ) -> Element<'a, Message> {
        let mut args = FluentArgs::new();
        args.set("organization", organization.to_owned());
        let header = row![
            text(localizer.format("chat-title", &args)).font(font::UI_SEMIBOLD),
            space::horizontal(),
            button(icon(Icon::X).size(size::ICON_SMALL))
                .padding(gap::XXS)
                .style(style::ghost)
                .on_press(Message::Close),
        ]
        .align_y(Vertical::Center);
        let list: Element<'a, Message> = if messages.is_empty() {
            container(text(localizer.text("chat-empty")).style(style::muted))
                .center(Fill)
                .into()
        } else {
            scrollable(
                column(messages.iter().map(entry))
                    .spacing(gap::S)
                    .padding([0.0, gap::S]),
            )
            .anchor_bottom()
            .height(Fill)
            .into()
        };
        let ready = !self.draft.trim().is_empty();
        let input = row![
            text_input(&localizer.text("chat-placeholder"), &self.draft)
                .on_input(Message::Draft)
                .on_submit(Message::Send)
                .padding([gap::XS + gap::XXS, gap::S])
                .style(style::field),
            button(text(localizer.text("chat-send")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe(ready.then_some(Message::Send)),
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center);
        container(column![header, list, input].spacing(gap::S))
            .padding([gap::S, gap::L])
            .width(Fill)
            .height(size::PANEL)
            .style(style::chrome)
            .into()
    }
}

fn entry(message: &ChatMessage) -> Element<'_, Message> {
    let time = message
        .sent_at
        .to_zoned_datetime(TimeZone::system())
        .strftime("%H:%M")
        .to_string();
    column![
        row![
            text(&message.author_name)
                .font(font::UI_SEMIBOLD)
                .size(font_size::SMALL),
            text(time).size(font_size::CAPTION).style(style::muted),
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center),
        text(&message.text),
    ]
    .spacing(gap::XXS)
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_api::live::ChatMessage;
    use uuid::Uuid;

    use super::*;
    use crate::i18n::Localizer;

    fn message(author: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::now_v7(),
            organization: Uuid::nil(),
            author: Uuid::nil(),
            author_name: author.to_owned(),
            text: text.to_owned(),
            sent_at: jiff::Timestamp::UNIX_EPOCH,
        }
    }

    fn shows(panel: &ChatPanel, messages: &[ChatMessage], words: &str) -> bool {
        let localizer = Localizer::english();
        simulator(panel.view("Field Press", messages, &localizer))
            .find(words)
            .is_ok()
    }

    fn clicks(panel: &ChatPanel, label: &str) -> Vec<Message> {
        let localizer = Localizer::english();
        let mut ui = simulator(panel.view("Field Press", &[], &localizer));
        ui.click(label).unwrap();
        ui.into_messages().collect()
    }

    #[test]
    fn each_text_of_the_panel_is_in_the_english_messages() {
        let localizer = Localizer::english();

        for id in TEXT_IDS {
            assert_ne!(localizer.text(id), id, "{id} has no English text");
        }
    }

    #[test]
    fn the_panel_shows_the_messages_with_their_authors() {
        let panel = ChatPanel::default();
        let messages = [
            message("Ann", "The proofs are ready."),
            message("Bob", "I will read them today."),
        ];

        for words in [
            "Chat · Field Press",
            "Ann",
            "The proofs are ready.",
            "Bob",
            "I will read them today.",
        ] {
            assert!(
                shows(&panel, &messages, words),
                "{words} is not in the panel"
            );
        }
    }

    #[test]
    fn an_empty_chat_asks_for_the_first_message() {
        let panel = ChatPanel::default();

        assert!(shows(
            &panel,
            &[],
            "No messages yet. Write the first message."
        ));
    }

    #[test]
    fn send_waits_for_some_text() {
        let mut panel = ChatPanel::default();
        assert_eq!(clicks(&panel, "Send").len(), 0);

        panel.update(Message::Draft("  Ready?  ".to_owned()));

        assert!(matches!(clicks(&panel, "Send").as_slice(), [Message::Send]));
        assert_eq!(panel.update(Message::Send), Some("Ready?".to_owned()));
    }

    #[test]
    fn a_sent_draft_goes_away() {
        let mut panel = ChatPanel::default();
        panel.update(Message::Draft("Ready?".to_owned()));

        panel.clear_draft();

        assert_eq!(panel.update(Message::Send), None);
    }

    #[test]
    fn toggle_opens_and_closes_the_panel() {
        let mut panel = ChatPanel::default();

        panel.update(Message::Toggle);
        assert!(panel.is_open());
        panel.update(Message::Toggle);
        assert!(!panel.is_open());
    }
}
