use iced::widget::{button, column, row, rule, space, text};
use iced::{Element, Fill, Subscription, Theme};
use presswork_log::LogStore;
use presswork_ui::space as gap;

use crate::i18n::Localizer;
use crate::log_panel::{self, LogPanel};
use crate::workspace::Workspace;

pub(crate) struct App {
    localizer: Localizer,
    workspace: Workspace,
    logs: LogPanel,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
    Logs(log_panel::Message),
}

impl App {
    pub(crate) fn new(store: LogStore) -> Self {
        Self {
            localizer: Localizer::english(),
            workspace: Workspace::default(),
            logs: LogPanel::new(store),
        }
    }

    pub(crate) fn title(&self) -> String {
        self.localizer.text("app-title")
    }

    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::SelectWorkspace(workspace) => {
                tracing::debug!(?workspace, "Workspace selected");
                self.workspace = workspace;
            }
            Message::Logs(message) => self.logs.update(message),
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        self.logs.subscription().map(Message::Logs)
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let mut content = column![
            self.workspace_bar(),
            rule::horizontal(1),
            space().width(Fill).height(Fill)
        ];
        if self.logs.is_open() {
            content = content
                .push(rule::horizontal(1))
                .push(self.logs.view(&self.localizer).map(Message::Logs));
        }
        content
            .push(rule::horizontal(1))
            .push(self.status_bar())
            .into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        row![
            space().width(Fill),
            self.logs.toggle_button(&self.localizer).map(Message::Logs)
        ]
        .padding([gap::XS, gap::S])
        .into()
    }

    fn workspace_bar(&self) -> Element<'_, Message> {
        let buttons = Workspace::ALL.into_iter().map(|workspace| {
            let style: fn(&Theme, button::Status) -> button::Style = if workspace == self.workspace
            {
                button::primary
            } else {
                button::text
            };

            button(text(self.localizer.text(workspace.text_id())))
                .style(style)
                .on_press(Message::SelectWorkspace(workspace))
                .into()
        });

        row(buttons).spacing(gap::XS).padding(gap::S).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    fn app() -> App {
        App::new(LogStore::with_capacity(10))
    }

    #[test]
    fn starts_in_the_write_workspace() {
        assert_eq!(app().workspace, Workspace::Write);
    }

    #[test]
    fn status_bar_shows_the_logs_button() {
        let app = app();
        let mut ui = simulator(app.view());

        assert!(ui.find("Logs").is_ok());
    }

    #[test]
    fn click_on_logs_opens_the_log_panel() {
        let mut app = app();
        let mut ui = simulator(app.view());

        ui.click("Logs").expect("Logs is in the view");
        for message in ui.into_messages() {
            app.update(message);
        }

        assert!(app.logs.is_open());
    }

    #[test]
    fn select_workspace_changes_the_workspace() {
        let mut app = app();

        app.update(Message::SelectWorkspace(Workspace::Review));

        assert_eq!(app.workspace, Workspace::Review);
    }

    #[test]
    fn title_is_presswork() {
        assert_eq!(app().title(), "Presswork");
    }

    #[test]
    fn workspace_bar_shows_each_workspace() {
        let app = app();
        let mut ui = simulator(app.view());

        for label in [
            "Plan",
            "Research",
            "Write",
            "Translate",
            "Review",
            "Typeset",
            "Publish",
        ] {
            assert!(ui.find(label).is_ok(), "{label} is not in the view");
        }
    }

    #[test]
    fn click_on_a_workspace_selects_it() {
        let mut app = app();
        let mut ui = simulator(app.view());

        ui.click("Plan").expect("Plan is in the view");
        for message in ui.into_messages() {
            app.update(message);
        }

        assert_eq!(app.workspace, Workspace::Plan);
    }
}
