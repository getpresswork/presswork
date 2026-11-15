use std::sync::Arc;
use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, rule, space, text};
use iced::{Element, Fill, FillPortion, Subscription, Task, Theme, padding, system};
use presswork_editor::Editor;
use presswork_log::{Level, LogStore};
use presswork_preview::Preview;
use presswork_text::{Position, Unit};
use presswork_typst::{Output, Severity};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::theme::{self as ui_theme, Mode};
use presswork_ui::{font, font_size, size, space as gap};

use crate::compiler::{self, Compiler};
use crate::i18n::{FluentArgs, Localizer};
use crate::log_panel::{self, LogPanel};
use crate::workspace::Workspace;

const MARK_SIZE: f32 = 10.0;
const DESCRIPTION_WIDTH: f32 = 380.0;
const SCRATCH: &str = include_str!("../samples/scratch.typ");

pub(crate) struct App {
    localizer: Localizer,
    workspace: Workspace,
    logs: LogPanel,
    mode: Mode,
    editor: Editor,
    compiler: Compiler,
    preview: Preview,
    compiled: Option<Arc<Output>>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
    Logs(log_panel::Message),
    SystemTheme(Mode),
    Editor(presswork_editor::Action),
    Compiled(Arc<Output>),
    Preview(presswork_preview::Action),
}

impl App {
    pub(crate) fn new(store: LogStore) -> Self {
        Self {
            localizer: Localizer::english(),
            workspace: Workspace::default(),
            logs: LogPanel::new(store),
            mode: Mode::None,
            editor: Editor::new(SCRATCH),
            compiler: Compiler::new(),
            preview: Preview::default(),
            compiled: None,
        }
    }

    pub(crate) fn request_compile(&mut self) -> Task<Message> {
        if self.compiler.request() {
            self.start_compile()
        } else {
            Task::none()
        }
    }

    fn start_compile(&self) -> Task<Message> {
        let text = self.editor.document().buffer().to_string();
        Task::perform(self.compiler.job(text), |output| {
            Message::Compiled(Arc::new(output))
        })
    }

    #[cfg(test)]
    pub(crate) fn compile_now(&mut self) {
        let text = self.editor.document().buffer().to_string();
        let output = self.compiler.run_now(&text);
        let _ = self.update(Message::Compiled(Arc::new(output)));
    }

    fn problem(&self, output: &Output) -> Option<String> {
        let error = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == Severity::Error)?;
        let mut args = FluentArgs::new();
        args.set("message", error.message.clone());
        let Some(range) = &error.range else {
            return Some(self.localizer.format("preview-error-without-line", &args));
        };
        let line = self
            .editor
            .document()
            .buffer()
            .line_of(Position::new(range.start));
        args.set("line", line + 1);
        Some(self.localizer.format("preview-error", &args))
    }

    pub(crate) fn title(&self) -> String {
        self.localizer.text("app-title")
    }

    pub(crate) fn theme(&self) -> Theme {
        ui_theme::for_mode(self.mode)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
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
            Message::Editor(action) => {
                let revision = self.editor.revision();
                if let Err(error) = self.editor.perform(action, Instant::now()) {
                    tracing::warn!(%error, "Editor action failed");
                }
                if self.editor.revision() != revision {
                    return self.request_compile();
                }
            }
            Message::Compiled(output) => {
                tracing::debug!(
                    duration_ms = output.duration.as_millis(),
                    diagnostics = output.diagnostics.len(),
                    "Compiled"
                );
                self.preview.show(&output, compiler::SCALE);
                self.preview.set_problem(self.problem(&output));
                self.compiled = Some(output);
                if self.compiler.finish() {
                    return self.start_compile();
                }
            }
            Message::Preview(action) => self.preview.perform(action),
        }
        Task::none()
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
        if workspace == Workspace::Write {
            let colors = ui_theme::colors(&self.theme());
            let editor =
                presswork_editor::view(&self.editor, colors, Message::Editor).autofocus(true);
            let preview = self
                .preview
                .view(self.localizer.text("preview-empty"), Message::Preview);
            return row![
                container(editor).width(FillPortion(1)).height(Fill),
                rule::vertical(1).style(style::hairline),
                container(preview).width(FillPortion(1)).height(Fill),
            ]
            .into();
        }
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

    fn cursor_position(&self) -> String {
        let document = self.editor.document();
        let head = document.selection().primary().head;
        let position = document.buffer().line_column(head, Unit::Char);
        let mut args = FluentArgs::new();
        args.set("line", position.line + 1);
        args.set("column", position.column + 1);
        self.localizer.format("status-position", &args)
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let (errors, warnings) = self.logs.counts();
        let mut indicators = row![].spacing(gap::XS).align_y(Vertical::Center);
        if self.workspace == Workspace::Write {
            if let Some(output) = &self.compiled {
                let mut args = FluentArgs::new();
                let milliseconds = u64::try_from(output.duration.as_millis()).unwrap_or(u64::MAX);
                args.set("milliseconds", milliseconds);
                indicators =
                    indicators.push(status_text(self.localizer.format("status-compiled", &args)));
            }
            indicators = indicators.push(status_text(self.cursor_position()));
        }
        if errors > 0 {
            indicators = indicators.push(count_button(
                Icon::CircleX,
                errors,
                Tone::Danger,
                Level::Error,
            ));
        }
        if warnings > 0 {
            indicators = indicators.push(count_button(
                Icon::TriangleAlert,
                warnings,
                Tone::Warning,
                Level::Warn,
            ));
        }
        indicators = indicators.push(self.logs.toggle_button(&self.localizer).map(Message::Logs));
        container(
            row![
                text(self.localizer.text("status-no-project"))
                    .size(font_size::SMALL)
                    .style(style::muted),
                space().width(Fill),
                indicators,
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
pub(crate) fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn status_text<'a>(content: String) -> Element<'a, Message> {
    container(text(content).size(font_size::SMALL).style(style::muted))
        .padding([0.0, gap::S])
        .into()
}

fn count_button<'a>(symbol: Icon, count: usize, tone: Tone, level: Level) -> Element<'a, Message> {
    let content = row![
        icon(symbol).size(size::ICON_SMALL).style(style::tone(tone)),
        text(count.to_string())
            .size(font_size::SMALL)
            .font(font::UI_MEDIUM)
            .style(style::tone(tone)),
    ]
    .spacing(gap::XXS)
    .align_y(Vertical::Center);
    button(content)
        .padding([gap::XXS, gap::XS + gap::XXS])
        .style(style::ghost)
        .on_press(Message::Logs(log_panel::Message::Show(level)))
        .into()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use iced_test::simulator;
    use presswork_log::LogEntry;
    use presswork_typst::Diagnostic;

    fn insert(text: &str) -> Message {
        Message::Editor(presswork_editor::Action::Insert(text.to_owned()))
    }

    fn compiled(output: Output) -> Message {
        Message::Compiled(Arc::new(output))
    }

    #[test]
    fn an_edit_starts_a_compilation() {
        let mut app = app();
        assert!(!app.compiler.is_running());

        send(&mut app, insert("x"));

        assert!(app.compiler.is_running());
    }

    #[test]
    fn edits_during_a_compilation_start_one_more() {
        let mut app = app();
        send(&mut app, insert("x"));
        send(&mut app, insert("y"));

        send(&mut app, compiled(Output::default()));
        assert!(app.compiler.is_running());

        send(&mut app, compiled(Output::default()));
        assert!(!app.compiler.is_running());
    }

    #[test]
    fn the_preview_waits_for_the_first_compilation() {
        let app = app();
        let mut ui = simulator(app.view());

        assert!(
            ui.find("The preview shows the pages after the first compilation.")
                .is_ok()
        );
    }

    #[test]
    fn the_status_bar_shows_the_time_of_the_last_compilation() {
        let mut app = app();
        send(
            &mut app,
            compiled(Output {
                duration: Duration::from_millis(85),
                ..Output::default()
            }),
        );

        let mut ui = simulator(app.view());

        assert!(ui.find("Compiled in 85 ms").is_ok());
    }

    #[test]
    fn an_error_appears_above_the_preview_with_its_line() {
        let mut app = app();
        let line_two = app.editor.document().buffer().line_start(1).offset();
        let error = Diagnostic {
            severity: Severity::Error,
            range: Some(line_two..line_two + 1),
            message: "unknown variable: x".to_owned(),
            hints: Vec::new(),
        };
        send(
            &mut app,
            compiled(Output {
                diagnostics: vec![error],
                ..Output::default()
            }),
        );

        let mut ui = simulator(app.view());

        assert!(ui.find("Line 2: unknown variable: x").is_ok());
    }

    #[test]
    fn a_compilation_now_shows_the_pages_of_the_scratch_text() {
        let mut app = app();

        app.compile_now();

        assert_eq!(app.preview.page_count(), 1);
    }

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

        send(&mut app, Message::SystemTheme(Mode::Dark));
        assert!(app.theme().extended_palette().is_dark);

        send(&mut app, Message::SystemTheme(Mode::Light));
        assert!(!app.theme().extended_palette().is_dark);
    }

    #[test]
    fn main_area_describes_a_workspace_without_content() {
        let mut app = app();
        send(&mut app, Message::SelectWorkspace(Workspace::Plan));
        let mut ui = simulator(app.view());

        let description = app.localizer.text(Workspace::Plan.description_id());
        assert!(ui.find(description).is_ok());
    }

    #[test]
    fn typing_in_the_write_workspace_changes_the_text() {
        let mut app = app();
        let mut ui = simulator(app.view());

        ui.typewrite("x");
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        assert!(app.editor.document().buffer().to_string().starts_with('x'));
    }

    #[test]
    fn other_workspaces_ignore_typing() {
        let mut app = app();
        send(&mut app, Message::SelectWorkspace(Workspace::Plan));
        let mut ui = simulator(app.view());

        ui.typewrite("x");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn status_bar_shows_the_cursor_position() {
        let app = app();
        let mut ui = simulator(app.view());

        assert!(ui.find("Line 1, column 1").is_ok());
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
            send(&mut app, message);
        }

        assert!(app.logs.is_open());
    }

    #[test]
    fn click_on_the_error_count_shows_only_the_errors() {
        let store = LogStore::with_capacity(10);
        for (level, message) in [(Level::Info, "Compiled"), (Level::Error, "Connection lost")] {
            store.push(LogEntry {
                level,
                message: message.to_owned(),
                ..LogEntry::default()
            });
        }
        let mut app = App::new(store);

        let mut ui = simulator(app.view());
        ui.click("1").expect("the error count is in the view");
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        let mut ui = simulator(app.view());
        assert!(ui.find("Connection lost").is_ok());
        assert!(ui.find("Compiled").is_err());
    }

    #[test]
    fn select_workspace_changes_the_workspace() {
        let mut app = app();

        send(&mut app, Message::SelectWorkspace(Workspace::Review));

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
            send(&mut app, message);
        }

        assert_eq!(app.workspace, Workspace::Plan);
    }
}
