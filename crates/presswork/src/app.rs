use iced::widget::{button, column, row, rule, space, text};
use iced::{Element, Fill, Theme};
use presswork_ui::space as gap;

use crate::i18n::Localizer;
use crate::workspace::Workspace;

pub(crate) struct App {
    localizer: Localizer,
    workspace: Workspace,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            localizer: Localizer::english(),
            workspace: Workspace::default(),
        }
    }

    pub(crate) fn title(&self) -> String {
        self.localizer.text("app-title")
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "iced gives each message to update by value"
    )]
    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::SelectWorkspace(workspace) => self.workspace = workspace,
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        column![
            self.workspace_bar(),
            rule::horizontal(1),
            space().width(Fill).height(Fill)
        ]
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

    #[test]
    fn starts_in_the_write_workspace() {
        assert_eq!(App::new().workspace, Workspace::Write);
    }

    #[test]
    fn select_workspace_changes_the_workspace() {
        let mut app = App::new();

        app.update(Message::SelectWorkspace(Workspace::Review));

        assert_eq!(app.workspace, Workspace::Review);
    }

    #[test]
    fn title_is_presswork() {
        assert_eq!(App::new().title(), "Presswork");
    }

    #[test]
    fn workspace_bar_shows_each_workspace() {
        let app = App::new();
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
        let mut app = App::new();
        let mut ui = simulator(app.view());

        ui.click("Plan").expect("Plan is in the view");
        for message in ui.into_messages() {
            app.update(message);
        }

        assert_eq!(app.workspace, Workspace::Plan);
    }
}
