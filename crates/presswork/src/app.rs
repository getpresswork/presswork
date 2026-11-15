use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, rule, space, text};
use iced::{Element, Fill, Subscription, Theme, padding, system};
use presswork_log::LogStore;
use presswork_ui::icon::icon;
use presswork_ui::style;
use presswork_ui::theme::{self as ui_theme, Mode};
use presswork_ui::{font, font_size, size, space as gap};

use crate::i18n::Localizer;
use crate::log_panel::{self, LogPanel};
use crate::workspace::Workspace;

const MARK_SIZE: f32 = 10.0;
const DESCRIPTION_WIDTH: f32 = 380.0;

pub(crate) struct App {
    localizer: Localizer,
    workspace: Workspace,
    logs: LogPanel,
    mode: Mode,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
    Logs(log_panel::Message),
    SystemTheme(Mode),
}

impl App {
    pub(crate) fn new(store: LogStore) -> Self {
        Self {
            localizer: Localizer::english(),
            workspace: Workspace::default(),
            logs: LogPanel::new(store),
            mode: Mode::None,
        }
    }

    pub(crate) fn title(&self) -> String {
        self.localizer.text("app-title")
    }

    pub(crate) fn theme(&self) -> Theme {
        ui_theme::for_mode(self.mode)
    }

    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::SelectWorkspace(workspace) => {
                tracing::debug!(?workspace, "Workspace selected");
                self.workspace = workspace;
            }
            Message::Logs(message) => self.logs.update(message),
            Message::SystemTheme(mode) => {
                tracing::debug!(?mode, "System theme changed");
                self.mode = mode;
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.logs.subscription().map(Message::Logs),
            system::theme_changes().map(Message::SystemTheme),
        ])
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let mut content = column![self.top_bar(), hairline(), self.main_area()];
        if self.logs.is_open() {
            content = content
                .push(hairline())
                .push(self.logs.view(&self.localizer).map(Message::Logs));
        }
        content.push(hairline()).push(self.status_bar()).into()
    }

    fn top_bar(&self) -> Element<'_, Message> {
        let mark = container(space().width(MARK_SIZE).height(MARK_SIZE)).style(style::mark);
        let title = text(self.title())
            .font(font::UI_SEMIBOLD)
            .size(font_size::LARGE);
        let tabs = row(Workspace::ALL
            .into_iter()
            .map(|workspace| self.workspace_tab(workspace)))
        .spacing(gap::XXS);
        container(
            row![mark, title, space().width(gap::XL), tabs]
                .spacing(gap::S)
                .align_y(Vertical::Center),
        )
        .padding([0.0, gap::L])
        .width(Fill)
        .height(size::TOP_BAR)
        .align_y(Vertical::Center)
        .style(style::chrome)
        .into()
    }

    fn workspace_tab(&self, workspace: Workspace) -> Element<'_, Message> {
        let content = row![
            icon(workspace.icon()),
            text(self.localizer.text(workspace.text_id())).font(font::UI_MEDIUM)
        ]
        .spacing(gap::XS + gap::XXS)
        .align_y(Vertical::Center);
        button(content)
            .padding([gap::XS, gap::S + gap::XXS])
            .style(style::tab(workspace == self.workspace))
            .on_press(Message::SelectWorkspace(workspace))
            .into()
    }

    fn main_area(&self) -> Element<'_, Message> {
        let workspace = self.workspace;
        let content = column![
            icon(workspace.icon())
                .size(size::ICON_LARGE)
                .style(style::muted),
            text(self.localizer.text(workspace.text_id()))
                .font(font::UI_SEMIBOLD)
                .size(font_size::TITLE),
            text(self.localizer.text(workspace.description_id()))
                .style(style::muted)
                .width(DESCRIPTION_WIDTH)
                .align_x(Horizontal::Center),
        ]
        .spacing(gap::S)
        .align_x(Horizontal::Center);
        container(content).center(Fill).style(style::page).into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        container(
            row![
                text(self.localizer.text("status-no-project"))
                    .size(font_size::SMALL)
                    .style(style::muted),
                space().width(Fill),
                self.logs.toggle_button(&self.localizer).map(Message::Logs),
            ]
            .align_y(Vertical::Center),
        )
        .padding(padding::left(gap::L).right(gap::S))
        .width(Fill)
        .height(size::STATUS_BAR)
        .align_y(Vertical::Center)
        .style(style::chrome)
        .into()
    }
}

fn hairline<'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::hairline).into()
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
    fn theme_follows_the_system_mode() {
        let mut app = app();

        app.update(Message::SystemTheme(Mode::Dark));
        assert!(app.theme().extended_palette().is_dark);

        app.update(Message::SystemTheme(Mode::Light));
        assert!(!app.theme().extended_palette().is_dark);
    }

    #[test]
    fn main_area_describes_the_workspace() {
        let app = app();
        let mut ui = simulator(app.view());

        let description = app.localizer.text(Workspace::Write.description_id());
        assert!(ui.find(description).is_ok());
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
