use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, rule, space, stack, text};
use iced::{
    Element, Fill, FillPortion, Size, Subscription, Task, Theme, event, keyboard, padding, system,
    window,
};
use presswork_editor::{Editor, Mark, MarkKind};
use presswork_log::{Level, LogStore};
use presswork_preview::Preview;
use presswork_project::Project;
use presswork_text::{Position, Unit};
use presswork_toolchain::{Engine, Version, choose, parse_pin};
use presswork_typst::{Diagnostic, Output, Program, Severity};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::theme::{self as ui_theme, Mode};
use presswork_ui::{font, font_size, size, space as gap};

use crate::chrome::{self, Controls, Frame};
use crate::compiler::{self, Backend, Compiler};
use crate::files;
use crate::i18n::{FluentArgs, Localizer};
use crate::log_panel::{self, LogPanel};
use crate::menu;
use crate::toolchain_panel::{self, Choice, ToolchainPanel, sentence, toolchain_directory};
use crate::workspace::Workspace;

const MARK_SIZE: f32 = 10.0;
const BRAND: &str = "presswork-menu";
const DESCRIPTION_WIDTH: f32 = 380.0;
const MAIN_WINDOW: Size = Size::new(1280.0, 800.0);
const PREVIEW_WINDOW: Size = Size::new(720.0, 900.0);
const SCRATCH: &str = include_str!("../samples/scratch.typ");

pub(crate) struct App {
    localizer: Localizer,
    workspace: Workspace,
    logs: LogPanel,
    toolchains: ToolchainPanel,
    mode: Mode,
    editor: Editor,
    compiler: Compiler,
    preview: Preview,
    compiled: Option<Arc<Output>>,
    compiled_revision: Option<u64>,
    project: Option<Project>,
    saved: u64,
    notice: Option<String>,
    main_window: window::Id,
    preview_window: Option<window::Id>,
    maximized: HashSet<window::Id>,
    menu_open: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
    Logs(log_panel::Message),
    Toolchains(toolchain_panel::Message),
    SystemTheme(Mode),
    Editor(presswork_editor::Action),
    Compiled {
        generation: u64,
        revision: u64,
        output: Arc<Output>,
    },
    CompileFailed {
        generation: u64,
        revision: u64,
        error: String,
    },
    Preview(presswork_preview::Action),
    OpenFolder,
    FolderPicked(Option<PathBuf>),
    Save,
    Key(keyboard::Event),
    DismissNotice,
    ToggleMenu,
    CloseMenu,
    MenuItem(menu::Item),
    Window(window::Id, chrome::Action),
    WindowResized(window::Id),
    Maximized(window::Id, bool),
    WindowClosed(window::Id),
}

impl App {
    pub(crate) fn new(store: LogStore) -> Self {
        Self::with_toolchains(store, toolchain_directory())
    }

    pub(crate) fn with_toolchains(store: LogStore, toolchains: Option<PathBuf>) -> Self {
        let editor = Editor::new(SCRATCH);
        let embedded =
            Version::parse(presswork_typst::version()).unwrap_or_else(|_| Version::new(0, 0, 0));
        Self {
            localizer: Localizer::english(),
            workspace: Workspace::default(),
            logs: LogPanel::new(store),
            toolchains: ToolchainPanel::new(toolchains, embedded),
            mode: Mode::None,
            saved: editor.revision(),
            editor,
            compiler: Compiler::new(),
            preview: Preview::default(),
            compiled: None,
            compiled_revision: None,
            project: None,
            notice: None,
            main_window: window::Id::unique(),
            preview_window: None,
            maximized: HashSet::new(),
            menu_open: false,
        }
    }

    pub(crate) fn open_main_window(&mut self) -> Task<Message> {
        let (id, opened) = window::open(window::Settings {
            position: window::Position::Centered,
            ..chrome::settings(MAIN_WINDOW, Controls::CURRENT)
        });
        self.main_window = id;
        opened.discard()
    }

    fn detach_preview(&mut self) -> Task<Message> {
        if let Some(id) = self.preview_window {
            return window::gain_focus(id);
        }
        let (id, opened) = window::open(chrome::settings(PREVIEW_WINDOW, Controls::CURRENT));
        self.preview_window = Some(id);
        self.preview.set_detached(true);
        tracing::debug!("Preview window opened");
        opened.discard()
    }

    #[cfg(test)]
    pub(crate) fn preview_window(&self) -> Option<window::Id> {
        self.preview_window
    }

    pub(crate) fn window_closed(&mut self, id: window::Id) -> bool {
        self.maximized.remove(&id);
        if self.preview_window == Some(id) {
            self.preview_window = None;
            self.preview.set_detached(false);
            return false;
        }
        id == self.main_window
    }

    pub(crate) fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if self.preview_window == Some(id) {
            let frame = self.frame(id);
            let title = row![
                space().width(gap::S),
                text(self.window_title(id)).font(font::UI_MEDIUM)
            ];
            let content = column![
                chrome::bar(frame, &self.localizer, title),
                hairline(),
                self.preview.view(self.preview_text(), Message::Preview)
            ];
            return chrome::window(frame, content);
        }
        self.view()
    }

    pub(crate) fn window_title(&self, id: window::Id) -> String {
        if self.preview_window != Some(id) {
            return self.title();
        }
        match &self.project {
            Some(project) => {
                let mut args = FluentArgs::new();
                args.set("name", project.manifest().name.clone());
                self.localizer.format("preview-window-title", &args)
            }
            None => self.localizer.text("preview-window-title-scratch"),
        }
    }

    pub(crate) fn window_theme(&self, _id: window::Id) -> Theme {
        self.theme()
    }

    fn frame(&self, id: window::Id) -> Frame {
        Frame {
            id,
            controls: Controls::CURRENT,
            maximized: self.maximized.contains(&id),
        }
    }

    pub(crate) fn marks(&self) -> Vec<Mark> {
        let Some(output) = &self.compiled else {
            return Vec::new();
        };
        if self.compiled_revision != Some(self.editor.revision()) {
            return Vec::new();
        }
        output
            .diagnostics
            .iter()
            .filter_map(|diagnostic| {
                let range = diagnostic.range.as_ref()?;
                Some(Mark {
                    range: Position::new(range.start)..Position::new(range.end),
                    kind: match diagnostic.severity {
                        Severity::Error => MarkKind::Error,
                        Severity::Warning => MarkKind::Warning,
                    },
                })
            })
            .collect()
    }

    fn show_problem(&mut self) {
        let Some(start) = self
            .compiled
            .as_ref()
            .and_then(|output| {
                output
                    .diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.severity == Severity::Error)
            })
            .and_then(|diagnostic| diagnostic.range.as_ref())
            .map(|range| range.start)
        else {
            return;
        };
        let buffer = self.editor.document().buffer();
        let position = buffer.position(
            buffer.line_column(Position::new(start), Unit::Byte),
            Unit::Byte,
        );
        let action = presswork_editor::Action::MoveTo {
            position,
            extend: false,
            goal: None,
        };
        if let Err(error) = self.editor.perform(action, Instant::now()) {
            tracing::warn!(%error, "The cursor did not move to the problem");
        }
    }

    fn open_folder(&self) -> Task<Message> {
        let title = self.localizer.text("open-folder-title");
        Task::perform(files::pick_folder(title), Message::FolderPicked)
    }

    fn open_project(&mut self, root: &Path) -> Task<Message> {
        match files::open(root) {
            Ok(opened) => {
                tracing::info!(root = %root.display(), "Project opened");
                self.editor = Editor::new(&opened.text);
                self.saved = self.editor.revision();
                let root = opened.project.root().to_path_buf();
                let main = opened.project.manifest().main.clone();
                self.project = Some(opened.project);
                let backend = self.backend();
                self.compiler.open(&root, &main, backend);
                self.preview = Preview::default();
                self.compiled = None;
                self.notice = None;
                self.workspace = Workspace::Write;
                self.request_compile()
            }
            Err(error) => {
                tracing::warn!(%error, "The folder did not open as a project");
                self.notice = Some(files::error_text(&self.localizer, &error));
                Task::none()
            }
        }
    }

    fn save(&mut self) {
        let Some(project) = &self.project else {
            return;
        };
        let path = project.main();
        match fs::write(&path, self.editor.document().buffer().to_string()) {
            Ok(()) => {
                tracing::info!(path = %path.display(), "File saved");
                self.saved = self.editor.revision();
            }
            Err(error) => {
                tracing::error!(%error, path = %path.display(), "File not saved");
                let mut args = FluentArgs::new();
                args.set("file", path.display().to_string());
                self.notice = Some(self.localizer.format("save-error", &args));
            }
        }
    }

    fn is_saved(&self) -> bool {
        self.project.is_none() || self.editor.revision() == self.saved
    }

    fn choice(&self) -> Option<Choice> {
        let project = self.project.as_ref()?;
        let Some(pin) = project.manifest().typst.as_deref() else {
            return Some(Choice::Engine(Engine::Embedded));
        };
        Some(match parse_pin(pin) {
            Ok(pin) => Choice::Engine(choose(
                Some(&pin),
                self.toolchains.embedded(),
                self.toolchains.installed(),
            )),
            Err(_) => Choice::InvalidPin(pin.to_owned()),
        })
    }

    fn backend(&self) -> Backend {
        match self.choice() {
            None | Some(Choice::Engine(Engine::Embedded)) => Backend::Embedded,
            Some(Choice::Engine(Engine::Managed(version))) => self
                .toolchains
                .executable(&version)
                .map_or(Backend::Unavailable, |path| {
                    Backend::Program(Program::new(path, version))
                }),
            Some(Choice::Engine(Engine::Missing(_)) | Choice::InvalidPin(_)) => {
                Backend::Unavailable
            }
        }
    }

    fn sync_backend(&mut self) -> Task<Message> {
        let backend = self.backend();
        if !self.compiler.set_backend(backend) {
            return Task::none();
        }
        tracing::info!(backend = ?self.compiler.backend(), "Typst engine changed");
        self.preview = Preview::default();
        self.compiled = None;
        self.compiled_revision = None;
        self.request_compile()
    }

    fn preview_text(&self) -> String {
        if self.compiler.backend() != &Backend::Unavailable {
            return self.localizer.text("preview-empty");
        }
        match self.choice() {
            Some(Choice::Engine(Engine::Missing(pin))) => {
                let mut args = FluentArgs::new();
                args.set("pin", pin.to_string());
                self.localizer.format("preview-needs-typst", &args)
            }
            Some(Choice::InvalidPin(_)) => self.localizer.text("preview-invalid-pin"),
            _ => self.localizer.text("preview-empty"),
        }
    }

    fn use_typst(&mut self, version: &Version) {
        let Some(project) = &mut self.project else {
            return;
        };
        match project.set_pin(&format!("={version}")) {
            Ok(()) => tracing::info!(%version, "Typst pin written"),
            Err(error) => {
                tracing::warn!(%error, %version, "Typst pin not written");
                let mut args = FluentArgs::new();
                args.set("error", sentence(&error.to_string()));
                self.notice = Some(self.localizer.format("toolchains-pin-error", &args));
            }
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
        let generation = self.compiler.generation();
        let revision = self.editor.revision();
        Task::perform(self.compiler.job(text), move |result| {
            compiled(generation, revision, result)
        })
    }

    #[cfg(test)]
    pub(crate) fn compile_now(&mut self) {
        let text = self.editor.document().buffer().to_string();
        let result = self.compiler.run_now(&text);
        let _ = self.update(compiled(
            self.compiler.generation(),
            self.editor.revision(),
            result,
        ));
    }

    fn show_output(
        &mut self,
        generation: u64,
        revision: u64,
        output: Arc<Output>,
    ) -> Task<Message> {
        if generation != self.compiler.generation() {
            return Task::none();
        }
        self.compiled_revision = Some(revision);
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
        Task::none()
    }

    fn failed_output(&self, error: &str) -> Output {
        let mut args = FluentArgs::new();
        args.set("error", sentence(error));
        Output {
            diagnostics: vec![Diagnostic {
                severity: Severity::Error,
                range: None,
                message: self.localizer.format("preview-program-error", &args),
                hints: Vec::new(),
            }],
            ..Output::default()
        }
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
            Message::Logs(message) => {
                self.logs.update(message);
                if self.logs.is_open() {
                    self.toolchains.close();
                }
            }
            Message::Toolchains(toolchain_panel::Message::Use(version)) => {
                self.use_typst(&version);
                return self.sync_backend();
            }
            Message::Toolchains(message) => {
                let toggled = matches!(message, toolchain_panel::Message::Toggle);
                let task = self.toolchains.update(message);
                if toggled && self.toolchains.is_open() {
                    self.logs.update(log_panel::Message::Close);
                }
                let sync = self.sync_backend();
                return Task::batch([task.map(Message::Toolchains), sync]);
            }
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
            Message::Compiled {
                generation,
                revision,
                output,
            } => return self.show_output(generation, revision, output),
            Message::CompileFailed {
                generation,
                revision,
                error,
            } => {
                tracing::warn!(%error, "The Typst program did not compile");
                let output = Arc::new(self.failed_output(&error));
                return self.show_output(generation, revision, output);
            }
            Message::Preview(presswork_preview::Action::ShowProblem) => self.show_problem(),
            Message::Preview(presswork_preview::Action::Detach) => return self.detach_preview(),
            Message::Preview(presswork_preview::Action::Attach) => {
                return self.preview_window.map_or_else(Task::none, window::close);
            }
            Message::ToggleMenu => self.menu_open = !self.menu_open,
            Message::CloseMenu => self.menu_open = false,
            Message::MenuItem(item) => {
                self.menu_open = false;
                return self.choose(item);
            }
            Message::Window(id, action) => return chrome::perform(id, action),
            Message::WindowResized(id) => {
                return window::is_maximized(id)
                    .map(move |maximized| Message::Maximized(id, maximized));
            }
            Message::Maximized(id, true) => {
                self.maximized.insert(id);
            }
            Message::Maximized(id, false) => {
                self.maximized.remove(&id);
            }
            Message::WindowClosed(id) => {
                if self.window_closed(id) {
                    return iced::exit();
                }
            }
            Message::Preview(action) => self.preview.perform(action),
            Message::OpenFolder => return self.open_folder(),
            Message::FolderPicked(Some(root)) => return self.open_project(&root),
            Message::Save => self.save(),
            Message::Key(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) if modifiers.command() => match key.to_latin(physical_key) {
                Some('s') => self.save(),
                Some('o') => return self.open_folder(),
                Some('q') => return iced::exit(),
                _ => {}
            },
            Message::FolderPicked(None) | Message::Key(_) => {}
            Message::DismissNotice => self.notice = None,
        }
        Task::none()
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let menu = if self.menu_open {
            event::listen_with(menu::menu_key)
        } else {
            Subscription::none()
        };
        Subscription::batch([
            self.logs.subscription().map(Message::Logs),
            system::theme_changes().map(Message::SystemTheme),
            keyboard::listen().map(Message::Key),
            window::events().filter_map(window_event),
            menu,
        ])
    }

    fn choose(&mut self, item: menu::Item) -> Task<Message> {
        match item {
            menu::Item::OpenFolder => self.open_folder(),
            menu::Item::Save => {
                self.save();
                Task::none()
            }
            menu::Item::Typst => self.update(Message::Toolchains(toolchain_panel::Message::Toggle)),
            menu::Item::Logs => self.update(Message::Logs(log_panel::Message::Toggle)),
            menu::Item::Quit => iced::exit(),
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let mut content = column![self.top_bar(), hairline(), self.main_area()];
        if self.logs.is_open() {
            content = content
                .push(hairline())
                .push(self.logs.view(&self.localizer).map(Message::Logs));
        }
        if self.toolchains.is_open() {
            let choice = self.choice();
            content = content.push(hairline()).push(
                self.toolchains
                    .view(&self.localizer, choice.as_ref())
                    .map(Message::Toolchains),
            );
        }
        let content = content.push(hairline()).push(self.status_bar());
        let mut layers = stack![content];
        if self.menu_open {
            layers = layers.push(menu::view(
                &self.localizer,
                self.project.is_some(),
                Controls::CURRENT,
            ));
        }
        chrome::window(self.frame(self.main_window), layers)
    }

    fn top_bar(&self) -> Element<'_, Message> {
        let mark = container(space().width(MARK_SIZE).height(MARK_SIZE)).style(style::mark);
        let brand = button(
            row![
                mark,
                text(self.title()).font(font::UI_SEMIBOLD),
                icon(Icon::ChevronDown)
                    .size(size::ICON_SMALL)
                    .style(style::muted)
            ]
            .spacing(gap::XS + gap::XXS)
            .align_y(Vertical::Center),
        )
        .padding([gap::XS, gap::S])
        .style(style::tab(self.menu_open))
        .on_press(Message::ToggleMenu);
        let brand = container(brand).id(BRAND);
        let tabs = row(Workspace::ALL
            .into_iter()
            .map(|workspace| self.workspace_tab(workspace)))
        .spacing(gap::XXS);
        let open = button(
            row![
                icon(Icon::FolderOpen).size(size::ICON_SMALL),
                text(self.localizer.text("open-folder")).size(font_size::SMALL)
            ]
            .spacing(gap::XS)
            .align_y(Vertical::Center),
        )
        .padding([gap::XS, gap::S])
        .style(style::ghost)
        .on_press(Message::OpenFolder);
        let content = row![
            brand,
            space().width(gap::XS),
            tabs,
            space().width(Fill),
            open
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center);
        chrome::bar(self.frame(self.main_window), &self.localizer, content)
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
        match &self.notice {
            Some(notice) => column![notice_bar(notice), self.workspace_area()].into(),
            None => self.workspace_area(),
        }
    }

    fn workspace_area(&self) -> Element<'_, Message> {
        let workspace = self.workspace;
        if workspace == Workspace::Write {
            let colors = ui_theme::colors(&self.theme());
            let editor = presswork_editor::view(&self.editor, colors, Message::Editor)
                .autofocus(true)
                .marks(self.marks());
            if self.preview_window.is_some() {
                return container(editor).width(Fill).height(Fill).into();
            }
            let preview = self.preview.view(self.preview_text(), Message::Preview);
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

    fn location(&self) -> String {
        let Some(project) = &self.project else {
            return self.localizer.text("status-no-project");
        };
        let mut args = FluentArgs::new();
        args.set("name", project.manifest().name.clone());
        args.set("file", project.manifest().main.display().to_string());
        self.localizer.format("status-project", &args)
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

    fn typst_button(&self) -> Element<'_, Message> {
        let mut args = FluentArgs::new();
        let (id, tone) = match self.choice() {
            None | Some(Choice::Engine(Engine::Embedded)) => {
                args.set("version", self.toolchains.embedded().to_string());
                ("status-typst", None)
            }
            Some(Choice::Engine(Engine::Managed(version))) => {
                args.set("version", version.to_string());
                ("status-typst", None)
            }
            Some(Choice::Engine(Engine::Missing(pin))) => {
                args.set("pin", pin.to_string());
                ("status-typst-missing", Some(Tone::Warning))
            }
            Some(Choice::InvalidPin(_)) => ("status-typst-invalid", Some(Tone::Danger)),
        };
        let mut label = text(self.localizer.format(id, &args)).size(font_size::SMALL);
        if let Some(tone) = tone {
            label = label.style(style::tone(tone));
        }
        button(
            row![icon(Icon::Package).size(size::ICON_SMALL), label]
                .spacing(gap::XS)
                .align_y(Vertical::Center),
        )
        .padding([gap::XXS, gap::S])
        .style(style::tab(self.toolchains.is_open()))
        .on_press(Message::Toolchains(toolchain_panel::Message::Toggle))
        .into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let (errors, warnings) = self.logs.counts();
        let mut indicators = row![].spacing(gap::XS).align_y(Vertical::Center);
        if self.workspace == Workspace::Write {
            if let Some(output) = &self.compiled {
                let mut args = FluentArgs::new();
                let milliseconds = u64::try_from(output.duration.as_millis())
                    .unwrap_or(u64::MAX)
                    .max(1);
                args.set("milliseconds", milliseconds);
                indicators =
                    indicators.push(status_text(self.localizer.format("status-compiled", &args)));
            }
            indicators = indicators.push(status_text(self.cursor_position()));
        }
        indicators = indicators.push(self.typst_button());
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
        let mut location = row![
            text(self.location())
                .size(font_size::SMALL)
                .style(style::muted)
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center);
        if !self.is_saved() {
            location = location.push(
                button(
                    text(self.localizer.text("status-not-saved"))
                        .size(font_size::SMALL)
                        .style(style::tone(Tone::Warning)),
                )
                .padding([gap::XXS, gap::XS])
                .style(style::ghost)
                .on_press(Message::Save),
            );
        }
        container(row![location, space().width(Fill), indicators].align_y(Vertical::Center))
            .padding(padding::left(gap::L).right(gap::S))
            .width(Fill)
            .height(size::STATUS_BAR)
            .align_y(Vertical::Center)
            .style(style::chrome)
            .into()
    }
}

fn compiled(generation: u64, revision: u64, result: Result<Output, String>) -> Message {
    match result {
        Ok(output) => Message::Compiled {
            generation,
            revision,
            output: Arc::new(output),
        },
        Err(error) => Message::CompileFailed {
            generation,
            revision,
            error,
        },
    }
}

fn hairline<'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::hairline).into()
}

fn window_event((id, event): (window::Id, window::Event)) -> Option<Message> {
    match event {
        window::Event::Closed => Some(Message::WindowClosed(id)),
        window::Event::CloseRequested => Some(Message::Window(id, chrome::Action::Close)),
        window::Event::Resized(_) => Some(Message::WindowResized(id)),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn notice_bar(notice: &str) -> Element<'_, Message> {
    container(
        row![
            icon(Icon::CircleAlert)
                .size(size::ICON_SMALL)
                .style(style::tone(Tone::Danger)),
            text(notice)
                .size(font_size::SMALL)
                .style(style::tone(Tone::Danger)),
            space().width(Fill),
            button(icon(Icon::X).size(size::ICON_SMALL))
                .padding(gap::XXS)
                .style(style::ghost)
                .on_press(Message::DismissNotice),
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center),
    )
    .padding([gap::XS, gap::L])
    .width(Fill)
    .style(style::badge(Tone::Danger))
    .into()
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
        Message::Compiled {
            generation: 0,
            revision: 0,
            output: Arc::new(output),
        }
    }

    fn error_at(offset: usize) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            range: Some(offset..offset + 1),
            message: "unknown variable: x".to_owned(),
            hints: Vec::new(),
        }
    }

    #[test]
    fn a_click_on_the_problem_moves_the_cursor_to_the_error() {
        let mut app = app();
        let line_two = app.editor.document().buffer().line_start(1).offset();
        send(
            &mut app,
            compiled(Output {
                diagnostics: vec![error_at(line_two)],
                ..Output::default()
            }),
        );

        send(
            &mut app,
            Message::Preview(presswork_preview::Action::ShowProblem),
        );

        assert_eq!(
            app.editor.document().selection().primary().head,
            Position::new(line_two)
        );
    }

    #[test]
    fn marks_show_only_while_the_text_agrees_with_the_compilation() {
        let mut app = app();
        send(
            &mut app,
            compiled(Output {
                diagnostics: vec![error_at(0)],
                ..Output::default()
            }),
        );
        assert_eq!(app.marks().len(), 1);

        send(&mut app, insert("x"));

        assert_eq!(app.marks(), []);
    }

    fn project_folder(files: &[(&str, &str)]) -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        for (path, text) in files {
            fs::write(folder.path().join(path), text).unwrap();
        }
        folder
    }

    fn book() -> tempfile::TempDir {
        project_folder(&[
            ("presswork.toml", "[project]\nname = \"The Book\"\n"),
            ("main.typ", "= Book"),
        ])
    }

    fn open(app: &mut App, folder: &tempfile::TempDir) {
        send(
            app,
            Message::FolderPicked(Some(folder.path().to_path_buf())),
        );
    }

    #[test]
    fn opening_a_project_folder_shows_its_main_file() {
        let folder = book();
        let mut app = app();

        open(&mut app, &folder);

        assert_eq!(app.editor.document().buffer().to_string(), "= Book");
        let mut ui = simulator(app.view());
        assert!(ui.find("The Book · main.typ").is_ok());
    }

    #[test]
    fn a_folder_that_is_not_a_project_shows_a_notice() {
        let folder = project_folder(&[("notes.txt", "")]);
        let mut app = app();

        open(&mut app, &folder);

        let mut ui = simulator(app.view());
        let notice = "This folder is not a Presswork project. A project folder contains a presswork.toml file.";
        assert!(ui.find(notice).is_ok());
        assert!(ui.find("No project open").is_ok());
    }

    #[test]
    fn a_notice_goes_away_when_dismissed() {
        let folder = project_folder(&[("notes.txt", "")]);
        let mut app = app();
        open(&mut app, &folder);

        send(&mut app, Message::DismissNotice);

        assert_eq!(app.notice, None);
    }

    #[test]
    fn save_writes_the_text_to_the_main_file() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);
        send(&mut app, insert("x"));
        assert!(simulator(app.view()).find("Not saved").is_ok());

        send(&mut app, Message::Save);

        let saved = fs::read_to_string(folder.path().join("main.typ")).unwrap();
        assert_eq!(saved, "x= Book");
        assert!(simulator(app.view()).find("Not saved").is_err());
    }

    #[test]
    fn control_and_s_saves_the_file() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);
        send(&mut app, insert("x"));

        send(
            &mut app,
            Message::Key(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character("s".into()),
                modified_key: keyboard::Key::Character("s".into()),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyS),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::COMMAND,
                text: None,
                repeat: false,
            }),
        );

        assert!(app.is_saved());
    }

    #[test]
    fn a_result_from_an_older_compiler_is_ignored() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);

        send(&mut app, compiled(Output::default()));

        assert!(app.compiled.is_none());
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
    fn a_very_fast_compilation_shows_one_millisecond() {
        let mut app = app();
        send(
            &mut app,
            compiled(Output {
                duration: Duration::from_micros(300),
                ..Output::default()
            }),
        );

        let mut ui = simulator(app.view());

        assert!(ui.find("Compiled in 1 ms").is_ok());
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
        App::with_toolchains(LogStore::with_capacity(10), None)
    }

    fn pinned_book(pin: &str) -> tempfile::TempDir {
        project_folder(&[
            (
                "presswork.toml",
                &format!("[project]\nname = \"The Book\"\ntypst = \"{pin}\"\n"),
            ),
            ("main.typ", "= Book"),
        ])
    }

    #[test]
    fn the_status_bar_shows_the_built_in_typst_version() {
        let app = app();
        let mut ui = simulator(app.view());

        assert!(
            ui.find(format!("Typst {}", presswork_typst::version()))
                .is_ok()
        );
    }

    #[test]
    fn a_pin_without_an_installed_version_shows_a_warning() {
        let folder = pinned_book("=0.13.1");
        let mut app = app();
        open(&mut app, &folder);

        let mut ui = simulator(app.view());

        assert!(ui.find("Typst =0.13.1 is not installed").is_ok());
    }

    #[test]
    fn a_pin_that_is_not_valid_shows_a_problem() {
        let folder = pinned_book("latest");
        let mut app = app();
        open(&mut app, &folder);

        let mut ui = simulator(app.view());

        assert!(ui.find("The Typst pin is not valid").is_ok());
    }

    #[test]
    fn a_click_on_the_typst_version_opens_the_toolchain_panel() {
        let mut app = app();
        let mut ui = simulator(app.view());

        ui.click(format!("Typst {}", presswork_typst::version()))
            .unwrap();
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        assert!(app.toolchains.is_open());
    }

    #[test]
    fn use_for_this_project_writes_an_exact_pin() {
        let folder = pinned_book("=0.13.1");
        let mut app = app();
        open(&mut app, &folder);
        let embedded = app.toolchains.embedded().clone();

        send(
            &mut app,
            Message::Toolchains(toolchain_panel::Message::Use(embedded.clone())),
        );

        let manifest = fs::read_to_string(folder.path().join("presswork.toml")).unwrap();
        assert!(manifest.contains(&format!("typst = \"={embedded}\"")));
        assert!(
            simulator(app.view())
                .find(format!("Typst {embedded}"))
                .is_ok()
        );
    }

    fn fake_install(root: &std::path::Path, version: &str) -> PathBuf {
        let folder = root.join("typst").join(version);
        fs::create_dir_all(&folder).unwrap();
        let program = folder.join(presswork_toolchain::Platform::current().executable());
        fs::write(&program, "").unwrap();
        program
    }

    #[test]
    fn a_missing_pinned_version_gives_no_preview_and_tells_what_to_install() {
        let folder = pinned_book("=0.13.1");
        let mut app = app();

        open(&mut app, &folder);

        assert!(!app.compiler.is_running());
        let mut ui = simulator(app.view());
        assert!(
            ui.find("This project needs Typst =0.13.1. Install it from the Typst item in the status bar.")
                .is_ok()
        );
    }

    #[test]
    fn an_installed_pinned_version_compiles_with_its_program() {
        let toolchains = tempfile::tempdir().unwrap();
        let program = fake_install(toolchains.path(), "0.14.2");
        let folder = pinned_book("=0.14.2");
        let mut app = App::with_toolchains(
            LogStore::with_capacity(10),
            Some(toolchains.path().to_path_buf()),
        );

        open(&mut app, &folder);

        assert_eq!(
            app.compiler.backend(),
            &Backend::Program(Program::new(program, Version::new(0, 14, 2)))
        );
        assert!(app.compiler.is_running());
    }

    #[test]
    #[ignore = "compiles with the real Typst 0.14.2 in the folder that PRESSWORK_TOOLCHAINS names"]
    fn a_pinned_project_previews_with_the_real_program() {
        let toolchains = PathBuf::from(std::env::var_os("PRESSWORK_TOOLCHAINS").unwrap());
        let folder = pinned_book("=0.14.2");
        let mut app = App::with_toolchains(LogStore::with_capacity(10), Some(toolchains));
        open(&mut app, &folder);

        app.compile_now();

        assert!(matches!(app.compiler.backend(), Backend::Program(_)));
        assert_eq!(app.preview.page_count(), 1);
    }

    #[test]
    fn installing_the_pinned_version_starts_the_preview() {
        let toolchains = tempfile::tempdir().unwrap();
        let folder = pinned_book("=0.14.2");
        let mut app = App::with_toolchains(
            LogStore::with_capacity(10),
            Some(toolchains.path().to_path_buf()),
        );
        open(&mut app, &folder);
        assert_eq!(app.compiler.backend(), &Backend::Unavailable);

        fake_install(toolchains.path(), "0.14.2");
        send(
            &mut app,
            Message::Toolchains(toolchain_panel::Message::Installed(
                Version::new(0, 14, 2),
                Ok(()),
            )),
        );

        assert!(matches!(app.compiler.backend(), Backend::Program(_)));
        assert!(app.compiler.is_running());
    }

    #[test]
    fn a_program_that_fails_shows_the_problem_above_the_preview() {
        let mut app = app();
        let generation = app.compiler.generation();
        let revision = app.editor.revision();

        send(
            &mut app,
            Message::CompileFailed {
                generation,
                revision,
                error: "the program does not start: not found".to_owned(),
            },
        );

        let mut ui = simulator(app.view());
        assert!(
            ui.find(
                "Presswork cannot run the Typst program. The program does not start: not found."
            )
            .is_ok()
        );
    }

    const EMPTY_PREVIEW: &str = "The preview shows the pages after the first compilation.";

    fn detach(app: &mut App) -> window::Id {
        send(app, Message::Preview(presswork_preview::Action::Detach));
        app.preview_window.unwrap()
    }

    #[test]
    fn detaching_the_preview_moves_it_into_its_own_window() {
        let mut app = app();

        let preview = detach(&mut app);

        assert!(simulator(app.view()).find(EMPTY_PREVIEW).is_err());
        assert!(
            simulator(app.view_window(preview))
                .find(EMPTY_PREVIEW)
                .is_ok()
        );
    }

    #[test]
    fn closing_the_preview_window_brings_the_preview_back() {
        let mut app = app();
        let preview = detach(&mut app);

        send(&mut app, Message::WindowClosed(preview));

        assert_eq!(app.preview_window, None);
        assert!(simulator(app.view()).find(EMPTY_PREVIEW).is_ok());
    }

    #[test]
    fn a_second_detach_keeps_one_preview_window() {
        let mut app = app();
        let preview = detach(&mut app);

        assert_eq!(detach(&mut app), preview);
    }

    #[test]
    fn the_preview_window_has_the_name_of_the_project_in_its_title() {
        let folder = book();
        let mut app = app();
        let preview = detach(&mut app);
        assert_eq!(app.window_title(preview), "Preview");

        open(&mut app, &folder);

        assert_eq!(app.window_title(preview), "Preview · The Book");
        assert_eq!(app.window_title(app.main_window), "Presswork");
    }

    #[test]
    fn the_preview_window_has_its_own_top_bar() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);
        let preview = detach(&mut app);

        let mut ui = simulator(app.view_window(preview));

        assert!(ui.find("Preview · The Book").is_ok());
        let buttons = ui.find(iced_test::selector::id("window-close")).is_ok();
        assert_eq!(buttons, Controls::CURRENT == Controls::Custom);
    }

    #[test]
    fn closing_the_main_window_ends_the_application() {
        let mut app = app();
        let preview = detach(&mut app);

        assert!(!app.window_closed(preview));
        let main = app.main_window;
        assert!(app.window_closed(main));
    }

    #[test]
    fn the_toolchain_panel_and_the_log_panel_do_not_open_together() {
        let mut app = app();
        send(&mut app, Message::Logs(log_panel::Message::Toggle));

        send(
            &mut app,
            Message::Toolchains(toolchain_panel::Message::Toggle),
        );
        assert!(app.toolchains.is_open());
        assert!(!app.logs.is_open());

        send(&mut app, Message::Logs(log_panel::Message::Toggle));
        assert!(app.logs.is_open());
        assert!(!app.toolchains.is_open());
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

    #[test]
    fn window_events_become_messages() {
        let id = window::Id::unique();

        assert!(matches!(
            window_event((id, window::Event::Closed)),
            Some(Message::WindowClosed(target)) if target == id
        ));
        assert!(matches!(
            window_event((id, window::Event::CloseRequested)),
            Some(Message::Window(target, chrome::Action::Close)) if target == id
        ));
        assert!(matches!(
            window_event((id, window::Event::Resized(Size::new(900.0, 600.0)))),
            Some(Message::WindowResized(target)) if target == id
        ));
        assert!(window_event((id, window::Event::Focused)).is_none());
    }

    #[test]
    fn the_frame_knows_when_a_window_is_maximized() {
        let mut app = app();
        let main = app.main_window;

        send(&mut app, Message::Maximized(main, true));
        assert!(app.frame(main).maximized);

        send(&mut app, Message::Maximized(main, false));
        assert!(!app.frame(main).maximized);
    }

    #[test]
    fn a_closed_window_forgets_that_it_was_maximized() {
        let mut app = app();
        let preview = detach(&mut app);
        send(&mut app, Message::Maximized(preview, true));

        send(&mut app, Message::WindowClosed(preview));

        assert!(!app.frame(preview).maximized);
    }

    #[test]
    fn a_click_on_the_mark_opens_the_presswork_menu() {
        let mut app = app();
        let mut ui = simulator(app.view());

        ui.click(iced_test::selector::id("presswork-menu"))
            .expect("the mark is in the top bar");
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        assert!(simulator(app.view()).find("Typst versions").is_ok());
        send(&mut app, Message::CloseMenu);
        assert!(simulator(app.view()).find("Typst versions").is_err());
    }

    #[test]
    fn menu_items_run_their_commands_and_close_the_menu() {
        let mut app = app();

        send(&mut app, Message::ToggleMenu);
        send(&mut app, Message::MenuItem(menu::Item::Logs));
        assert!(app.logs.is_open());
        assert!(!app.menu_open);

        send(&mut app, Message::ToggleMenu);
        send(&mut app, Message::MenuItem(menu::Item::Typst));
        assert!(app.toolchains.is_open());
        assert!(!app.menu_open);
    }

    #[test]
    fn save_in_the_menu_saves_the_file() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);
        send(&mut app, insert("x"));

        send(&mut app, Message::MenuItem(menu::Item::Save));

        assert!(app.is_saved());
    }

    #[test]
    fn only_a_window_without_system_buttons_shows_window_buttons() {
        let app = app();
        let mut ui = simulator(app.view());

        let found = ui.find(iced_test::selector::id("window-close")).is_ok();

        assert_eq!(found, Controls::CURRENT == Controls::Custom);
    }
}
