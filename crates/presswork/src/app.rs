use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, column, container, operation, responsive, row, rule, space, stack, text, tooltip,
};
use iced::{
    Element, Fill, FillPortion, Size, Subscription, Task, Theme, event, keyboard, padding, system,
    window,
};
use presswork_api::live::ServerMessage;
use presswork_api::{Document, SearchHit};
use presswork_editor::{
    CompletionEvent, CompletionItem, CompletionList, Editor, Mark, MarkKind, Motion,
};
use presswork_log::{Level, LogStore};
use presswork_preview::Preview;
use presswork_project::Project;
use presswork_text::{ChangeSet, Position, Unit};
use presswork_toolchain::{Engine, Version, choose, parse_pin};
use presswork_typst::{Analysis, Diagnostic, Jump, Output, PagePoint, Program, Severity};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::theme::{self as ui_theme, Mode};
use presswork_ui::{font, font_size, size, space as gap};

use crate::assist;
use crate::chat_panel::{self, ChatPanel};
use crate::chrome::{self, Controls, Frame};
use crate::compiler::{self, Backend, Compiler};
use crate::files;
use crate::i18n::{FluentArgs, Localizer};
use crate::library_view::{self, LibraryView};
use crate::live_session::{self, LiveSession};
use crate::log_panel::{self, LogPanel};
use crate::menu;
use crate::server_dialog::{self, ServerDialog};
use crate::task_list::{self, TaskList};
use crate::toolchain_panel::{self, Choice, ToolchainPanel, sentence, toolchain_directory};
use crate::unsaved;
use crate::workspace::Workspace;

const MARK_SIZE: f32 = 10.0;
const BRAND: &str = "presswork-menu";
const EDITOR: &str = "editor";
const LABEL_ADVANCE: f32 = 0.58;
const DRAG_SPACE: f32 = 48.0;
const DESCRIPTION_WIDTH: f32 = 380.0;
const MAIN_WINDOW: Size = Size::new(1280.0, 800.0);
const PREVIEW_WINDOW: Size = Size::new(720.0, 900.0);
const SCRATCH: &str = include_str!("../samples/scratch.typ");
const BIBLIOGRAPHY_FILE: &str = "library.yml";

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
    pages_analysis: Option<Analysis>,
    hovered: Option<Position>,
    tooltip: Option<presswork_editor::Tooltip>,
    completions: Option<presswork_editor::CompletionList>,
    explicit_completion: bool,
    project: Option<Project>,
    saved: u64,
    notice: Option<String>,
    main_window: window::Id,
    preview_window: Option<window::Id>,
    maximized: HashSet<window::Id>,
    menu_open: bool,
    quitting: bool,
    server_dialog: ServerDialog,
    live: Option<LiveSession>,
    chat: ChatPanel,
    tasks: Option<TaskList>,
    library: Option<LibraryView>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectWorkspace(Workspace),
    Logs(log_panel::Message),
    Toolchains(toolchain_panel::Message),
    SystemTheme(Mode),
    Editor(presswork_editor::Action),
    Hover(Option<Position>),
    Definition(Position),
    DefinitionFound {
        revision: u64,
        definition: Option<presswork_typst::Definition>,
    },
    Complete(presswork_editor::CompletionEvent),
    CompletionsFound {
        revision: u64,
        offset: usize,
        explicit: bool,
        completions: Option<presswork_typst::Completions>,
    },
    TooltipFound {
        position: Position,
        revision: u64,
        tooltip: Option<presswork_typst::Tooltip>,
    },
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
    Unsaved(unsaved::Answer),
    Server(server_dialog::Message),
    Live(live_session::Event),
    Chat(chat_panel::Message),
    Tasks(task_list::Message),
    Library(library_view::Message),
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
            pages_analysis: None,
            hovered: None,
            tooltip: None,
            completions: None,
            explicit_completion: false,
            project: None,
            notice: None,
            main_window: window::Id::unique(),
            preview_window: None,
            maximized: HashSet::new(),
            menu_open: false,
            quitting: false,
            server_dialog: ServerDialog::default(),
            live: None,
            chat: ChatPanel::default(),
            tasks: None,
            library: None,
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
        self.move_cursor(start);
    }

    fn open_folder(&self) -> Task<Message> {
        let title = self.localizer.text("open-folder-title");
        Task::perform(files::pick_folder(title), Message::FolderPicked)
    }

    fn open_project(&mut self, root: &Path) -> Task<Message> {
        match files::open(root) {
            Ok(opened) => {
                tracing::info!(root = %root.display(), "Project opened");
                if let Some(live) = &mut self.live {
                    live.close_document();
                }
                self.tasks = None;
                self.library = None;
                self.editor = Editor::new(&opened.text);
                self.saved = self.editor.revision();
                let root = opened.project.root().to_path_buf();
                let main = opened.project.manifest().main.clone();
                self.project = Some(opened.project);
                let backend = self.backend();
                self.compiler.open(&root, &main, backend);
                self.preview = Preview::default();
                self.compiled = None;
                self.pages_analysis = None;
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

    fn open_server_document(&mut self, document: Document) -> Task<Message> {
        let organization = self.server_dialog.organization_of(&document);
        let project = document.project_id;
        let project_name = self.server_dialog.project_name(project).unwrap_or_default();
        let Some(live) = &mut self.live else {
            return Task::none();
        };
        self.tasks = Some(TaskList::new(project, project_name));
        self.library = organization
            .as_ref()
            .map(|(id, name)| LibraryView::new(*id, name.clone()));
        tracing::info!(path = %document.path, "Server document opened");
        live.open_document(document);
        if let Some((organization, name)) = organization {
            live.join_chat(organization, name);
        }
        self.editor = Editor::new("");
        self.saved = self.editor.revision();
        self.project = None;
        self.compiler.scratch();
        self.preview = Preview::default();
        self.compiled = None;
        self.compiled_revision = None;
        self.pages_analysis = None;
        self.notice = None;
        self.workspace = Workspace::Write;
        Task::batch([self.request_compile(), self.load_library()])
    }

    fn load_library(&self) -> Task<Message> {
        match (&self.library, &self.live) {
            (Some(library), Some(live)) => library.load(live.connection()).map(Message::Library),
            _ => Task::none(),
        }
    }

    fn library_message(&mut self, message: library_view::Message) -> Task<Message> {
        if let library_view::Message::Cite(hit) = &message {
            return self.cite(hit);
        }
        let works_changed = matches!(
            message,
            library_view::Message::Loaded(Ok(_))
                | library_view::Message::Uploaded(Ok(_))
                | library_view::Message::Changed(Ok(_))
        );
        let task = match (&mut self.library, &self.live) {
            (Some(library), Some(live)) => library
                .update(message, live.connection())
                .map(Message::Library),
            _ => return Task::none(),
        };
        if works_changed {
            return Task::batch([task, self.give_bibliography()]);
        }
        task
    }

    fn give_bibliography(&mut self) -> Task<Message> {
        let Some(works) = self.library.as_ref().and_then(LibraryView::works) else {
            return Task::none();
        };
        match presswork_library::bibliography(works) {
            Ok(yaml) => {
                self.compiler.set_file(Path::new(BIBLIOGRAPHY_FILE), &yaml);
                self.request_compile()
            }
            Err(error) => {
                tracing::warn!(%error, "The bibliography of the library was not made");
                Task::none()
            }
        }
    }

    fn cite(&mut self, hit: &SearchHit) -> Task<Message> {
        self.workspace = Workspace::Write;
        let mut tasks = Vec::new();
        if !self
            .editor
            .document()
            .buffer()
            .to_string()
            .contains("#bibliography(")
        {
            let cursor = self.editor.document().selection().primary().head;
            tasks.push(self.edit(presswork_editor::Action::Move {
                motion: Motion::DocumentEnd,
                extend: false,
            }));
            tasks.push(self.edit(presswork_editor::Action::Insert(format!(
                "\n#bibliography(\"{BIBLIOGRAPHY_FILE}\")\n"
            ))));
            tasks.push(self.edit(presswork_editor::Action::MoveTo {
                position: cursor,
                extend: false,
                goal: None,
            }));
        }
        tasks.push(self.edit(presswork_editor::Action::Insert(format!(
            "@{}[p. {}]",
            hit.key, hit.label
        ))));
        Task::batch(tasks)
    }

    fn edit(&mut self, action: presswork_editor::Action) -> Task<Message> {
        let typed = matches!(action, presswork_editor::Action::Insert(_));
        let revision = self.editor.revision();
        let head = self.cursor();
        match self.editor.perform(action, Instant::now()) {
            Ok(Some(changes)) => self.share(&changes),
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "Editor action failed"),
        }
        if self.editor.revision() != revision {
            self.tooltip = None;
            let compile = self.request_compile();
            if !typed && self.completions.is_none() {
                return compile;
            }
            let explicit = self.completions.is_some() && self.explicit_completion;
            return Task::batch([compile, self.request_completions(explicit)]);
        }
        if self.cursor() == head {
            return Task::none();
        }
        self.completions = None;
        self.follow_cursor()
    }

    fn cursor(&self) -> Position {
        self.editor.document().selection().primary().head
    }

    fn preview_action(&mut self, action: presswork_preview::Action) -> Task<Message> {
        match action {
            presswork_preview::Action::ShowProblem => self.show_problem(),
            presswork_preview::Action::Click(point) => return self.jump_to_source(point),
            presswork_preview::Action::Detach => return self.detach_preview(),
            presswork_preview::Action::Attach => {
                return self.preview_window.map_or_else(Task::none, window::close);
            }
            action => self.preview.perform(action),
        }
        Task::none()
    }

    fn assist_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Hover(position) => return self.hover(position),
            Message::TooltipFound {
                position,
                revision,
                tooltip,
            } => self.tooltip_found(position, revision, tooltip),
            Message::Definition(position) => return self.find_definition(position),
            Message::DefinitionFound {
                revision,
                definition,
            } => return self.definition_found(revision, definition),
            Message::Complete(event) => return self.complete(event),
            Message::CompletionsFound {
                revision,
                offset,
                explicit,
                completions,
            } => self.completions_found(revision, offset, explicit, completions),
            _ => {}
        }
        Task::none()
    }

    fn find_definition(&self, position: Position) -> Task<Message> {
        let Some(analysis) = self.assist_analysis() else {
            return Task::none();
        };
        let text = self.editor.document().buffer().to_string();
        let revision = self.editor.revision();
        Task::perform(
            assist::definition(analysis, text, position.offset()),
            move |definition| Message::DefinitionFound {
                revision,
                definition,
            },
        )
    }

    fn definition_found(
        &mut self,
        revision: u64,
        definition: Option<presswork_typst::Definition>,
    ) -> Task<Message> {
        if revision != self.editor.revision() {
            return Task::none();
        }
        match definition {
            Some(presswork_typst::Definition::Main(range)) => {
                let start = self.snapped(range.start);
                let end = self.snapped(range.end);
                let at_start = self.edit(presswork_editor::Action::MoveTo {
                    position: start,
                    extend: false,
                    goal: None,
                });
                let to_end = self.edit(presswork_editor::Action::MoveTo {
                    position: end,
                    extend: true,
                    goal: None,
                });
                Task::batch([at_start, to_end])
            }
            Some(presswork_typst::Definition::File(path, _)) => {
                let mut args = FluentArgs::new();
                args.set("file", path.display().to_string());
                self.notice = Some(self.localizer.format("definition-other-file", &args));
                Task::none()
            }
            Some(presswork_typst::Definition::Library(name)) => {
                let mut args = FluentArgs::new();
                args.set("name", name);
                self.notice = Some(self.localizer.format("definition-in-library", &args));
                Task::none()
            }
            None => Task::none(),
        }
    }

    fn complete(&mut self, event: CompletionEvent) -> Task<Message> {
        match event {
            CompletionEvent::Request => return self.request_completions(true),
            CompletionEvent::Next => {
                if let Some(list) = &mut self.completions {
                    list.next();
                }
            }
            CompletionEvent::Previous => {
                if let Some(list) = &mut self.completions {
                    list.previous();
                }
            }
            CompletionEvent::Accept => {
                if let Some(index) = self.completions.as_ref().map(|list| list.selected) {
                    return self.apply_completion(index);
                }
            }
            CompletionEvent::Choose(index) => return self.apply_completion(index),
            CompletionEvent::Close => self.completions = None,
        }
        Task::none()
    }

    fn apply_completion(&mut self, index: usize) -> Task<Message> {
        let Some(list) = self.completions.take() else {
            return Task::none();
        };
        match list.apply(index, self.cursor()) {
            Some(action) => self.edit(action),
            None => Task::none(),
        }
    }

    fn request_completions(&self, explicit: bool) -> Task<Message> {
        let Some(analysis) = self.assist_analysis() else {
            return Task::none();
        };
        let text = self.editor.document().buffer().to_string();
        let revision = self.editor.revision();
        let offset = self.cursor().offset();
        Task::perform(
            assist::completions(analysis, text, offset, explicit),
            move |completions| Message::CompletionsFound {
                revision,
                offset,
                explicit,
                completions,
            },
        )
    }

    fn completions_found(
        &mut self,
        revision: u64,
        offset: usize,
        explicit: bool,
        completions: Option<presswork_typst::Completions>,
    ) {
        if revision != self.editor.revision() || offset != self.cursor().offset() {
            return;
        }
        self.completions = completions.and_then(|completions| self.completion_list(completions));
        self.explicit_completion = explicit && self.completions.is_some();
    }

    fn completion_list(&self, completions: presswork_typst::Completions) -> Option<CompletionList> {
        let cursor = self.cursor().offset();
        let from = completions.from.min(cursor);
        let typed = self
            .editor
            .document()
            .buffer()
            .slice(Position::new(from)..Position::new(cursor))
            .to_lowercase();
        let items: Vec<CompletionItem> = completions
            .items
            .into_iter()
            .filter(|item| item.label.to_lowercase().starts_with(&typed))
            .map(|item| CompletionItem {
                label: item.label,
                detail: item.detail,
                text: item.text,
                select: item.select,
            })
            .collect();
        (!items.is_empty()).then(|| CompletionList {
            from: Position::new(from),
            items,
            selected: 0,
        })
    }

    fn hover(&mut self, position: Option<Position>) -> Task<Message> {
        self.hovered = position;
        self.tooltip = None;
        let (Some(position), Some(analysis)) = (position, self.assist_analysis()) else {
            return Task::none();
        };
        let text = self.editor.document().buffer().to_string();
        let revision = self.editor.revision();
        Task::perform(
            assist::tooltip(analysis, text, position.offset()),
            move |tooltip| Message::TooltipFound {
                position,
                revision,
                tooltip,
            },
        )
    }

    fn assist_analysis(&self) -> Option<Analysis> {
        self.pages_analysis
            .clone()
            .or_else(|| self.compiled.as_ref()?.analysis.clone())
    }

    fn tooltip_found(
        &mut self,
        position: Position,
        revision: u64,
        tooltip: Option<presswork_typst::Tooltip>,
    ) {
        if self.hovered != Some(position) || revision != self.editor.revision() {
            return;
        }
        self.tooltip = tooltip.map(|tooltip| {
            let (text, code) = match tooltip {
                presswork_typst::Tooltip::Text(text) => (text, false),
                presswork_typst::Tooltip::Code(code) => (code, true),
            };
            presswork_editor::Tooltip {
                position,
                text,
                code,
            }
        });
    }

    fn follow_cursor(&mut self) -> Task<Message> {
        let offset = self.cursor().offset();
        let point = self
            .pages_analysis
            .as_ref()
            .and_then(|analysis| analysis.jump_from_cursor(offset).first().copied());
        match point {
            Some(point) => self.preview.show_point(point, Instant::now()),
            None => Task::none(),
        }
    }

    fn jump_to_source(&mut self, point: PagePoint) -> Task<Message> {
        let Some(jump) = self
            .pages_analysis
            .as_ref()
            .map(|analysis| analysis.jump_from_click(point))
        else {
            if matches!(self.compiler.backend(), Backend::Program(_)) {
                self.notice = Some(self.localizer.text("preview-jump-needs-engine"));
            }
            return Task::none();
        };
        match jump {
            Some(Jump::Main(offset)) => {
                self.move_cursor(offset);
                Task::batch([operation::focus(EDITOR), self.follow_cursor()])
            }
            Some(Jump::File(path, _)) => {
                let mut args = FluentArgs::new();
                args.set("file", path.display().to_string());
                self.notice = Some(self.localizer.format("preview-jump-other-file", &args));
                Task::none()
            }
            Some(Jump::Page(target)) => self.preview.show_point(target, Instant::now()),
            None => Task::none(),
        }
    }

    fn snapped(&self, offset: usize) -> Position {
        let buffer = self.editor.document().buffer();
        let start = Position::new(offset.min(buffer.len()));
        buffer.position(buffer.line_column(start, Unit::Byte), Unit::Byte)
    }

    fn move_cursor(&mut self, offset: usize) {
        let action = presswork_editor::Action::MoveTo {
            position: self.snapped(offset),
            extend: false,
            goal: None,
        };
        if let Err(error) = self.editor.perform(action, Instant::now()) {
            tracing::warn!(%error, "The cursor did not move");
        }
    }

    fn server(&mut self, message: server_dialog::Message) -> Task<Message> {
        if let server_dialog::Message::SignedIn(Ok(connection)) = &message
            && self.server_dialog.is_open()
        {
            self.live = Some(LiveSession::new(connection.clone()));
        }
        let opened = match &message {
            server_dialog::Message::Open(document) => Some(document.clone()),
            _ => None,
        };
        let dialog = self.server_dialog.update(message).map(Message::Server);
        match opened {
            Some(document) => Task::batch([dialog, self.open_server_document(document)]),
            None => dialog,
        }
    }

    fn chat_message(&mut self, message: chat_panel::Message) {
        let opening = matches!(message, chat_panel::Message::Toggle) && !self.chat.is_open();
        if let Some(text) = self.chat.update(message)
            && let Some(live) = &mut self.live
            && live.say(&text)
        {
            self.chat.clear_draft();
        }
        if opening {
            self.logs.update(log_panel::Message::Close);
            self.toolchains.close();
        }
    }

    fn select_workspace(&mut self, workspace: Workspace) -> Task<Message> {
        tracing::debug!(?workspace, "Workspace selected");
        self.workspace = workspace;
        match (workspace, &self.tasks, &self.live) {
            (Workspace::Plan, Some(tasks), Some(live)) => {
                tasks.load(live.connection()).map(Message::Tasks)
            }
            (Workspace::Research, _, _) => self.load_library(),
            _ => Task::none(),
        }
    }

    fn task_message(&mut self, message: task_list::Message) -> Task<Message> {
        match (&mut self.tasks, &self.live) {
            (Some(tasks), Some(live)) => {
                tasks.update(message, live.connection()).map(Message::Tasks)
            }
            _ => Task::none(),
        }
    }

    fn chat_button(&self) -> Element<'_, Message> {
        button(
            row![
                icon(Icon::MessagesSquare).size(size::ICON_SMALL),
                text(self.localizer.text("chat-toggle")).size(font_size::SMALL)
            ]
            .spacing(gap::XS)
            .align_y(Vertical::Center),
        )
        .padding([gap::XXS, gap::S])
        .style(style::tab(self.chat.is_open()))
        .on_press(Message::Chat(chat_panel::Message::Toggle))
        .into()
    }

    fn share(&mut self, changes: &ChangeSet) {
        if let Some(live) = &mut self.live
            && let Err(error) = live.local(changes)
        {
            tracing::warn!(%error, "A local change did not go into the live document");
        }
    }

    fn live_event(&mut self, event: live_session::Event) -> Task<Message> {
        if let live_session::Event::Received(ServerMessage::WorkChanged { work }) = &event {
            let changed = self
                .library
                .as_mut()
                .is_some_and(|library| library.changed((**work).clone()));
            return if changed {
                self.give_bibliography()
            } else {
                Task::none()
            };
        }
        let Some(live) = &mut self.live else {
            return Task::none();
        };
        match live.event(event) {
            Ok(Some(changes)) if !changes.is_empty() => {
                if let Err(error) = self.editor.apply_remote(&changes) {
                    tracing::error!(%error, "A remote change does not agree with the editor");
                }
                return self.request_compile();
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "A live change was not applied"),
        }
        Task::none()
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
        self.pages_analysis = None;
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
        if output.pages.is_some() {
            self.pages_analysis.clone_from(&output.analysis);
        }
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
            Message::SelectWorkspace(workspace) => return self.select_workspace(workspace),
            Message::Logs(message) => {
                self.logs.update(message);
                if self.logs.is_open() {
                    self.toolchains.close();
                    self.chat.close();
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
                    self.chat.close();
                }
                let sync = self.sync_backend();
                return Task::batch([task.map(Message::Toolchains), sync]);
            }
            Message::SystemTheme(mode) => {
                tracing::debug!(?mode, "System theme changed");
                self.mode = mode;
            }
            Message::Editor(_) if self.quitting => {}
            Message::Editor(action) => return self.edit(action),
            message @ (Message::Hover(_)
            | Message::TooltipFound { .. }
            | Message::Definition(_)
            | Message::DefinitionFound { .. }
            | Message::Complete(_)
            | Message::CompletionsFound { .. }) => return self.assist_message(message),
            Message::Server(message) => return self.server(message),
            Message::Live(event) => return self.live_event(event),
            Message::Chat(message) => self.chat_message(message),
            Message::Tasks(message) => return self.task_message(message),
            Message::Library(message) => return self.library_message(message),
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
            Message::Preview(action) => return self.preview_action(action),
            Message::ToggleMenu => self.menu_open = !self.menu_open,
            Message::CloseMenu => self.menu_open = false,
            Message::MenuItem(item) => {
                self.menu_open = false;
                return self.choose(item);
            }
            Message::Unsaved(answer) => return self.answer(answer),
            Message::Window(id, chrome::Action::Close) => return self.close_window(id),
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
                Some('q') => return self.quit(),
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
        let dialog = if self.quitting {
            event::listen_with(unsaved::dialog_key)
        } else if self.server_dialog.is_open() {
            event::listen_with(server_key)
        } else {
            Subscription::none()
        };
        let live = self.live.as_ref().map_or_else(Subscription::none, |live| {
            live.subscription().map(Message::Live)
        });
        Subscription::batch([
            self.logs.subscription().map(Message::Logs),
            system::theme_changes().map(Message::SystemTheme),
            keyboard::listen().map(Message::Key),
            window::events().filter_map(window_event),
            menu,
            dialog,
            live,
        ])
    }

    fn close_window(&mut self, id: window::Id) -> Task<Message> {
        if self.preview_window == Some(id) {
            return window::close(id);
        }
        self.quit()
    }

    fn quit(&mut self) -> Task<Message> {
        self.menu_open = false;
        if self.is_saved() {
            return iced::exit();
        }
        self.quitting = true;
        Task::none()
    }

    fn answer(&mut self, answer: unsaved::Answer) -> Task<Message> {
        self.quitting = false;
        match answer {
            unsaved::Answer::Save => {
                self.save();
                if self.is_saved() {
                    iced::exit()
                } else {
                    Task::none()
                }
            }
            unsaved::Answer::Discard => iced::exit(),
            unsaved::Answer::Cancel => Task::none(),
        }
    }

    fn choose(&mut self, item: menu::Item) -> Task<Message> {
        match item {
            menu::Item::OpenFolder => self.open_folder(),
            menu::Item::Save => {
                self.save();
                Task::none()
            }
            menu::Item::Connect => {
                self.server_dialog.open();
                Task::none()
            }
            menu::Item::Typst => self.update(Message::Toolchains(toolchain_panel::Message::Toggle)),
            menu::Item::Logs => self.update(Message::Logs(log_panel::Message::Toggle)),
            menu::Item::Quit => self.quit(),
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
        if self.chat.is_open()
            && let Some(chat) = self.live.as_ref().and_then(LiveSession::chat)
        {
            content = content.push(hairline()).push(
                self.chat
                    .view(chat.name(), chat.messages(), &self.localizer)
                    .map(Message::Chat),
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
        if self.server_dialog.is_open() {
            layers = layers.push(
                self.server_dialog
                    .view(&self.localizer)
                    .map(Message::Server),
            );
        }
        if let Some(project) = self.project.as_ref().filter(|_| self.quitting) {
            let file = project.manifest().main.display().to_string();
            layers = layers.push(unsaved::view(&self.localizer, &file));
        }
        chrome::window(self.frame(self.main_window), layers)
    }

    fn top_bar(&self) -> Element<'_, Message> {
        let content =
            responsive(move |available| self.bar_content(available.width < self.full_bar_width()))
                .height(size::TOP_BAR);
        chrome::bar(self.frame(self.main_window), &self.localizer, content)
    }

    fn full_bar_width(&self) -> f32 {
        let brand = 2.0 * gap::S
            + MARK_SIZE
            + label_width(&self.title(), font_size::BODY)
            + size::ICON_SMALL
            + 2.0 * (gap::XS + gap::XXS);
        let tabs: f32 = Workspace::ALL
            .into_iter()
            .map(|workspace| {
                2.0 * (gap::S + gap::XXS)
                    + size::ICON
                    + gap::XS
                    + 2.0 * gap::XXS
                    + label_width(&self.localizer.text(workspace.text_id()), font_size::BODY)
            })
            .sum();
        let open = 2.0 * gap::S
            + size::ICON_SMALL
            + gap::XS
            + label_width(&self.localizer.text("open-folder"), font_size::SMALL);
        brand + tabs + open + 4.0 * gap::S + gap::XS + DRAG_SPACE
    }

    fn bar_content(&self, compact: bool) -> Element<'_, Message> {
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
            .map(|workspace| self.workspace_tab(workspace, compact)))
        .spacing(gap::XXS);
        let words = self.localizer.text("open-folder");
        let mut label = row![icon(Icon::FolderOpen).size(size::ICON_SMALL)]
            .spacing(gap::XS)
            .align_y(Vertical::Center);
        if !compact {
            label = label.push(text(words.clone()).size(font_size::SMALL));
        }
        let open = button(label)
            .padding([gap::XS, gap::S])
            .style(style::ghost)
            .on_press(Message::OpenFolder);
        let open = if compact {
            chrome::tip(open, words, tooltip::Position::Bottom)
        } else {
            open.into()
        };
        row![
            brand,
            space().width(gap::XS),
            tabs,
            space().width(Fill),
            open
        ]
        .spacing(gap::S)
        .height(Fill)
        .align_y(Vertical::Center)
        .into()
    }

    fn workspace_tab(&self, workspace: Workspace, compact: bool) -> Element<'_, Message> {
        let name = self.localizer.text(workspace.text_id());
        let mut content = row![icon(workspace.icon())]
            .spacing(gap::XS + gap::XXS)
            .align_y(Vertical::Center);
        if !compact {
            content = content.push(text(name.clone()).font(font::UI_MEDIUM));
        }
        let tab = button(content)
            .padding([gap::XS, gap::S + gap::XXS])
            .style(style::tab(workspace == self.workspace))
            .on_press(Message::SelectWorkspace(workspace));
        if compact {
            chrome::tip(tab, name, tooltip::Position::Bottom)
        } else {
            tab.into()
        }
    }

    fn main_area(&self) -> Element<'_, Message> {
        match &self.notice {
            Some(notice) => column![notice_bar(notice), self.workspace_area()].into(),
            None => self.workspace_area(),
        }
    }

    fn workspace_area(&self) -> Element<'_, Message> {
        let workspace = self.workspace;
        if workspace == Workspace::Plan
            && let Some(tasks) = &self.tasks
        {
            return tasks.view(&self.localizer).map(Message::Tasks);
        }
        if workspace == Workspace::Research
            && let Some(library) = &self.library
        {
            return library.view(&self.localizer).map(Message::Library);
        }
        if workspace == Workspace::Write {
            let colors = ui_theme::colors(&self.theme());
            let editor = presswork_editor::view(&self.editor, colors, Message::Editor)
                .id(EDITOR)
                .autofocus(true)
                .marks(self.marks())
                .on_hover(Message::Hover)
                .tooltip(self.tooltip.as_ref())
                .on_complete(Message::Complete)
                .completions(self.completions.as_ref())
                .on_definition(Message::Definition);
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
        if let Some(live) = &self.live
            && let (Some(document), Some(state)) = (live.document(), live.state())
        {
            let mut args = FluentArgs::new();
            args.set("path", document.path.clone());
            let id = match state {
                live_session::State::Connecting => "status-live-connecting",
                live_session::State::Live => "status-live",
                live_session::State::Offline => "status-live-offline",
            };
            return self.localizer.format(id, &args);
        }
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
        if self.live.as_ref().and_then(LiveSession::chat).is_some() {
            indicators = indicators.push(self.chat_button());
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

fn label_width(words: &str, size: f32) -> f32 {
    let characters = u16::try_from(words.chars().count()).unwrap_or(u16::MAX);
    f32::from(characters) * LABEL_ADVANCE * size
}

fn server_key(event: iced::Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed { key: pressed, .. }) = event else {
        return None;
    };
    (pressed == keyboard::Key::Named(keyboard::key::Named::Escape))
        .then_some(Message::Server(server_dialog::Message::Close))
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
    use presswork_api::live::{ClientMessage, ServerMessage};
    use presswork_log::LogEntry;
    use presswork_sync::TextDocument;
    use presswork_text::{ChangeSet, Edit};
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

    const TWO_PAGES: &str = "First page.\n#pagebreak()\nSecond page.";

    fn compiled_app(text: &str) -> App {
        let mut app = app();
        app.editor = Editor::new(text);
        app.compile_now();
        app
    }

    fn page_point(app: &App, offset: usize) -> PagePoint {
        let point = app
            .pages_analysis
            .as_ref()
            .unwrap()
            .jump_from_cursor(offset)[0];
        PagePoint {
            x: point.x + 0.5,
            ..point
        }
    }

    fn click(point: PagePoint) -> Message {
        Message::Preview(presswork_preview::Action::Click(point))
    }

    fn move_to(offset: usize) -> Message {
        Message::Editor(presswork_editor::Action::MoveTo {
            position: Position::new(offset),
            extend: false,
            goal: None,
        })
    }

    #[test]
    fn a_click_on_the_pages_moves_the_cursor_to_the_text_and_focuses_the_editor() {
        let mut app = compiled_app(TWO_PAGES);
        let offset = TWO_PAGES.find("Second").unwrap() + 2;
        let point = page_point(&app, offset);

        let task = app.update(click(point));

        assert_eq!(app.cursor(), Position::new(offset));
        assert!(task.units() > 0);
    }

    #[test]
    fn a_click_on_text_of_another_file_names_the_file() {
        let folder = project_folder(&[
            ("presswork.toml", "[project]\nname = \"The Book\"\n"),
            ("main.typ", "#include \"part.typ\""),
            ("part.typ", "Included words."),
        ]);
        let mut app = app();
        open(&mut app, &folder);
        app.compile_now();

        send(
            &mut app,
            click(PagePoint {
                page: 0,
                x: 72.0,
                y: 75.0,
            }),
        );

        assert!(
            app.notice
                .as_deref()
                .is_some_and(|notice| notice.contains("part.typ")),
            "{:?}",
            app.notice
        );
    }

    #[test]
    fn a_click_without_the_built_in_engine_tells_why_nothing_happens() {
        let mut app = app();
        app.compiler.set_backend(Backend::Program(Program::new(
            PathBuf::from("typst"),
            Version::new(0, 14, 2),
        )));
        let output = Output {
            pages: Some(vec![presswork_typst::PageImage {
                width: 20,
                height: 30,
                rgba: vec![255; 20 * 30 * 4],
            }]),
            ..Output::default()
        };
        let generation = app.compiler.generation();
        let revision = app.editor.revision();
        send(
            &mut app,
            Message::Compiled {
                generation,
                revision,
                output: Arc::new(output),
            },
        );

        send(
            &mut app,
            click(PagePoint {
                page: 0,
                x: 5.0,
                y: 5.0,
            }),
        );

        assert_eq!(
            app.notice.as_deref(),
            Some(app.localizer.text("preview-jump-needs-engine").as_str())
        );
    }

    #[test]
    fn moving_the_cursor_marks_its_place_in_the_preview() {
        let mut app = compiled_app(TWO_PAGES);

        send(&mut app, move_to(TWO_PAGES.find("Second").unwrap()));

        assert_eq!(app.preview.marked().map(|point| point.page), Some(1));
    }

    #[test]
    fn typing_does_not_mark_the_preview() {
        let mut app = compiled_app(TWO_PAGES);

        send(&mut app, insert("x"));

        assert_eq!(app.preview.marked(), None);
    }

    const LENGTH: &str = "#let gap = 10pt";

    fn hover_at(offset: usize) -> Message {
        Message::Hover(Some(Position::new(offset)))
    }

    fn found(app: &App, offset: usize, tooltip: presswork_typst::Tooltip) -> Message {
        Message::TooltipFound {
            position: Position::new(offset),
            revision: app.editor.revision(),
            tooltip: Some(tooltip),
        }
    }

    fn code(text: &str) -> presswork_typst::Tooltip {
        presswork_typst::Tooltip::Code(text.to_owned())
    }

    #[test]
    fn resting_on_text_asks_for_its_tooltip() {
        let mut app = compiled_app(LENGTH);

        let task = app.update(hover_at(12));

        assert!(task.units() > 0);
    }

    #[test]
    fn a_found_tooltip_shows_at_the_hovered_text() {
        let mut app = compiled_app(LENGTH);
        send(&mut app, hover_at(12));

        let message = found(&app, 12, code("10pt = 3.53mm"));
        send(&mut app, message);

        assert_eq!(
            app.tooltip,
            Some(presswork_editor::Tooltip {
                position: Position::new(12),
                text: "10pt = 3.53mm".to_owned(),
                code: true,
            })
        );
    }

    #[test]
    fn a_tooltip_for_another_position_or_other_text_is_dropped() {
        let mut app = compiled_app(LENGTH);
        send(&mut app, hover_at(12));

        let other_position = found(&app, 5, code("10pt"));
        send(&mut app, other_position);
        let other_text = Message::TooltipFound {
            position: Position::new(12),
            revision: app.editor.revision() + 1,
            tooltip: Some(code("10pt")),
        };
        send(&mut app, other_text);

        assert_eq!(app.tooltip, None);
    }

    fn typed_app(text: &str) -> App {
        let mut app = compiled_app(text);
        send(&mut app, move_to(text.len()));
        app
    }

    fn completion(
        label: &str,
        text: &str,
        select: std::ops::Range<usize>,
    ) -> presswork_typst::Completion {
        presswork_typst::Completion {
            kind: presswork_typst::CompletionKind::Function,
            label: label.to_owned(),
            detail: None,
            text: text.to_owned(),
            select,
        }
    }

    fn completions_found(app: &App, from: usize) -> Message {
        Message::CompletionsFound {
            revision: app.editor.revision(),
            offset: app.cursor().offset(),
            explicit: false,
            completions: Some(presswork_typst::Completions {
                from,
                items: vec![
                    completion("text", "text()", 5..5),
                    completion("table", "table()", 6..6),
                    completion("rgb", "rgb()", 4..4),
                ],
            }),
        }
    }

    fn labels(app: &App) -> Vec<String> {
        app.completions.as_ref().map_or_else(Vec::new, |list| {
            list.items.iter().map(|item| item.label.clone()).collect()
        })
    }

    fn open_completions(text: &str) -> App {
        let mut app = typed_app(text);
        let found = completions_found(&app, 1);
        send(&mut app, found);
        app
    }

    #[test]
    fn found_completions_show_the_items_that_match_the_typed_text() {
        let app = open_completions("#t");

        assert_eq!(labels(&app), ["text", "table"]);
    }

    #[test]
    fn completions_for_other_text_are_dropped() {
        let mut app = typed_app("#t");
        let mut found = completions_found(&app, 1);
        if let Message::CompletionsFound { revision, .. } = &mut found {
            *revision += 1;
        }

        send(&mut app, found);

        assert_eq!(app.completions, None);
    }

    #[test]
    fn accepting_the_next_item_puts_it_into_the_text() {
        let mut app = open_completions("#t");

        send(&mut app, Message::Complete(CompletionEvent::Next));
        send(&mut app, Message::Complete(CompletionEvent::Accept));

        assert_eq!(app.editor.document().buffer().to_string(), "#table()");
        assert_eq!(app.cursor(), Position::new(7));
        assert_eq!(app.completions, None);
    }

    #[test]
    fn a_chosen_item_goes_into_the_text() {
        let mut app = open_completions("#t");

        send(&mut app, Message::Complete(CompletionEvent::Choose(0)));

        assert_eq!(app.editor.document().buffer().to_string(), "#text()");
        assert_eq!(app.cursor(), Position::new(6));
    }

    #[test]
    fn escape_and_cursor_moves_close_the_list() {
        let mut app = open_completions("#t");
        send(&mut app, Message::Complete(CompletionEvent::Close));
        assert_eq!(app.completions, None);

        let mut app = open_completions("#t");
        send(&mut app, move_to(0));
        assert_eq!(app.completions, None);
    }

    #[test]
    fn typing_and_control_space_ask_for_completions() {
        let mut app = typed_app("#t");

        assert!(
            app.update(Message::Complete(CompletionEvent::Request))
                .units()
                > 0
        );
        assert!(app.update(insert("e")).units() > 1);
    }

    const BINDING: &str = "#let gap = 1pt\n#gap";

    fn definition_found(app: &App, definition: presswork_typst::Definition) -> Message {
        Message::DefinitionFound {
            revision: app.editor.revision(),
            definition: Some(definition),
        }
    }

    #[test]
    fn a_definition_request_asks_the_engine() {
        let mut app = compiled_app(BINDING);

        let task = app.update(Message::Definition(Position::new(17)));

        assert!(task.units() > 0);
    }

    #[test]
    fn a_definition_in_the_main_file_selects_its_name() {
        let mut app = typed_app(BINDING);

        let found = definition_found(&app, presswork_typst::Definition::Main(5..8));
        send(&mut app, found);

        let selection = app.editor.document().selection().primary();
        assert_eq!(
            (selection.anchor, selection.head),
            (Position::new(5), Position::new(8))
        );
    }

    #[test]
    fn a_definition_in_another_file_names_the_file() {
        let mut app = typed_app(BINDING);

        let found = definition_found(
            &app,
            presswork_typst::Definition::File(PathBuf::from("part.typ"), None),
        );
        send(&mut app, found);

        assert!(
            app.notice
                .as_deref()
                .is_some_and(|notice| notice.contains("part.typ")),
            "{:?}",
            app.notice
        );
    }

    #[test]
    fn a_definition_in_the_library_tells_its_name() {
        let mut app = typed_app(BINDING);

        let found = definition_found(
            &app,
            presswork_typst::Definition::Library("text".to_owned()),
        );
        send(&mut app, found);

        assert!(
            app.notice
                .as_deref()
                .is_some_and(|notice| notice.contains("text")),
            "{:?}",
            app.notice
        );
    }

    #[test]
    fn a_definition_for_other_text_is_dropped() {
        let mut app = typed_app(BINDING);
        let head = app.cursor();
        let revision = app.editor.revision() + 1;

        send(
            &mut app,
            Message::DefinitionFound {
                revision,
                definition: Some(presswork_typst::Definition::Main(5..8)),
            },
        );

        assert_eq!(app.cursor(), head);
    }

    #[test]
    fn the_end_of_a_hover_hides_the_tooltip() {
        let mut app = compiled_app(LENGTH);
        send(&mut app, hover_at(12));
        let message = found(&app, 12, code("10pt"));
        send(&mut app, message);

        send(&mut app, Message::Hover(None));

        assert_eq!(app.tooltip, None);
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
        let mut ui = sized(app.view(), 1280.0);

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
        let mut ui = sized(app.view(), 1280.0);

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

    fn sized(element: Element<'_, Message>, width: f32) -> iced_test::Simulator<'_, Message> {
        iced_test::Simulator::with_size(iced::Settings::default(), (width, 800.0), element)
    }

    #[test]
    fn a_narrow_window_shows_only_the_icons_of_the_tabs() {
        let app = app();
        let research = Workspace::Research.icon().unicode().to_string();

        let mut wide = sized(app.view(), 1280.0);
        assert!(wide.find("Research").is_ok());
        assert!(wide.find("Open folder").is_ok());

        let mut narrow = sized(app.view(), 760.0);
        assert!(narrow.find("Research").is_err());
        assert!(narrow.find("Open folder").is_err());
        assert!(narrow.find(research).is_ok());
    }

    #[test]
    fn the_top_bar_centers_its_content_vertically() {
        let app = app();
        let mut ui = sized(app.view(), 1280.0);

        let bar = ui
            .find(iced_test::selector::id(chrome::TOP_BAR))
            .unwrap()
            .bounds();
        for label in ["Write", "Open folder"] {
            let bounds = ui.find(label).unwrap().bounds();
            assert!(
                (bounds.center_y() - bar.center_y()).abs() < 0.5,
                "{label}: {bounds:?} in {bar:?}"
            );
        }
    }

    #[test]
    fn a_tab_without_its_label_still_selects_its_workspace() {
        let mut app = app();
        let mut ui = sized(app.view(), 760.0);

        ui.click(Workspace::Plan.icon().unicode().to_string())
            .expect("the icon of the Plan tab is in the top bar");
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        assert_eq!(app.workspace, Workspace::Plan);
    }

    const SERVER_DOCUMENT: uuid::Uuid = uuid::Uuid::from_u128(4);

    fn signed_in(app: &mut App) {
        send(app, Message::MenuItem(menu::Item::Connect));
        let connection = server_dialog::Connection {
            api: presswork_client::Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
        };
        send(
            app,
            Message::Server(server_dialog::Message::SignedIn(Ok(connection))),
        );
    }

    fn open_server_document(
        app: &mut App,
    ) -> futures::channel::mpsc::UnboundedReceiver<ClientMessage> {
        signed_in(app);
        let document = presswork_api::Document {
            id: SERVER_DOCUMENT,
            project_id: uuid::Uuid::from_u128(2),
            path: "main.typ".to_owned(),
        };
        send(app, Message::Server(server_dialog::Message::Open(document)));
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        send(app, Message::Live(live_session::Event::Ready(sender)));
        send(app, Message::Live(live_session::Event::Connected));
        receiver
    }

    fn server_with(text: &str) -> TextDocument {
        let mut server = TextDocument::new();
        server
            .apply(&ChangeSet::new(0, [Edit::insert(Position::new(0), text)]).unwrap())
            .unwrap();
        server
    }

    fn subscribed(app: &mut App, server: &TextDocument) {
        send(
            app,
            Message::Live(live_session::Event::Received(ServerMessage::Subscribed {
                document: SERVER_DOCUMENT,
                version: server.version(),
                data: server
                    .updates_since(&TextDocument::new().version())
                    .unwrap(),
            })),
        );
    }

    fn drained(
        receiver: &mut futures::channel::mpsc::UnboundedReceiver<ClientMessage>,
    ) -> Vec<ClientMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    #[test]
    fn connect_in_the_menu_opens_the_server_dialog() {
        let mut app = app();

        send(&mut app, Message::MenuItem(menu::Item::Connect));

        assert!(simulator(app.view()).find("Connect to a server").is_ok());
    }

    #[test]
    fn a_server_document_opens_live_in_the_editor() {
        let mut app = app();
        let mut receiver = open_server_document(&mut app);
        assert!(simulator(app.view()).find("main.typ · Connecting").is_ok());
        assert!(matches!(
            drained(&mut receiver).as_slice(),
            [ClientMessage::Subscribe {
                document: SERVER_DOCUMENT,
                ..
            }]
        ));

        let mut server = server_with("The press stopped.");
        subscribed(&mut app, &server);
        assert_eq!(
            app.editor.document().buffer().to_string(),
            "The press stopped."
        );
        assert!(simulator(app.view()).find("main.typ · Live").is_ok());

        send(&mut app, insert("Late: "));
        let sent = drained(&mut receiver);
        let [ClientMessage::Updates { data, .. }] = sent.as_slice() else {
            panic!("the edit did not go to the server: {sent:?}");
        };
        server.merge(data).unwrap();
        assert_eq!(server.text(), "Late: The press stopped.");
    }

    #[test]
    fn remote_changes_appear_in_the_editor_and_keep_the_cursor() {
        let mut app = app();
        let _receiver = open_server_document(&mut app);
        let mut server = server_with("Proof");
        subscribed(&mut app, &server);
        send(
            &mut app,
            Message::Editor(presswork_editor::Action::MoveTo {
                position: Position::new(5),
                extend: false,
                goal: None,
            }),
        );

        let data = server
            .apply(&ChangeSet::new(5, [Edit::insert(Position::new(0), "The ")]).unwrap())
            .unwrap();
        send(
            &mut app,
            Message::Live(live_session::Event::Received(ServerMessage::Updates {
                document: SERVER_DOCUMENT,
                data,
            })),
        );

        assert_eq!(app.editor.document().buffer().to_string(), "The Proof");
        assert_eq!(
            app.editor.document().selection().primary().head,
            Position::new(9)
        );
    }

    const PRESS_ORGANIZATION: uuid::Uuid = uuid::Uuid::from_u128(1);

    fn open_with_chat(app: &mut App) -> futures::channel::mpsc::UnboundedReceiver<ClientMessage> {
        signed_in(app);
        let project = uuid::Uuid::from_u128(2);
        let document = presswork_api::Document {
            id: SERVER_DOCUMENT,
            project_id: project,
            path: "main.typ".to_owned(),
        };
        let shelf = server_dialog::Shelf {
            organization: "Field Press".to_owned(),
            project: presswork_api::Project {
                id: project,
                organization_id: PRESS_ORGANIZATION,
                name: "Field Notes".to_owned(),
            },
            documents: vec![document.clone()],
        };
        send(
            app,
            Message::Server(server_dialog::Message::Loaded(Ok(vec![shelf]))),
        );
        send(app, Message::Server(server_dialog::Message::Open(document)));
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        send(app, Message::Live(live_session::Event::Ready(sender)));
        send(app, Message::Live(live_session::Event::Connected));
        receiver
    }

    #[test]
    fn the_chat_of_the_organization_opens_from_the_status_bar() {
        let mut app = app();
        let mut receiver = open_with_chat(&mut app);
        assert!(drained(&mut receiver).contains(&ClientMessage::Join {
            organization: PRESS_ORGANIZATION
        }));
        let message = presswork_api::live::ChatMessage {
            id: uuid::Uuid::from_u128(7),
            organization: PRESS_ORGANIZATION,
            author: uuid::Uuid::nil(),
            author_name: "Bob".to_owned(),
            text: "Are the proofs ready?".to_owned(),
            sent_at: jiff::Timestamp::UNIX_EPOCH,
        };
        send(
            &mut app,
            Message::Live(live_session::Event::Received(ServerMessage::Joined {
                organization: PRESS_ORGANIZATION,
                messages: vec![message],
            })),
        );

        let mut ui = simulator(app.view());
        ui.click("Chat")
            .expect("the status bar has the chat button");
        for message in ui.into_messages() {
            send(&mut app, message);
        }

        assert!(simulator(app.view()).find("Are the proofs ready?").is_ok());
    }

    #[test]
    fn send_in_the_chat_sends_the_text_and_clears_the_draft() {
        let mut app = app();
        let mut receiver = open_with_chat(&mut app);
        drained(&mut receiver);
        send(&mut app, Message::Chat(chat_panel::Message::Toggle));
        send(
            &mut app,
            Message::Chat(chat_panel::Message::Draft("Ready.".to_owned())),
        );

        send(&mut app, Message::Chat(chat_panel::Message::Send));

        assert_eq!(
            drained(&mut receiver),
            [ClientMessage::Say {
                organization: PRESS_ORGANIZATION,
                text: "Ready.".to_owned(),
            }]
        );
        assert!(simulator(app.view()).find("Ready.").is_err());
    }

    fn field_notes_hit() -> presswork_api::SearchHit {
        presswork_api::SearchHit {
            work_id: uuid::Uuid::from_u128(10),
            edition_id: uuid::Uuid::from_u128(20),
            key: "field-notes".to_owned(),
            title: "Field Notes".to_owned(),
            page: 3,
            label: "iii".to_owned(),
            snippet: "The fine grain of the paper".to_owned(),
            standard: true,
        }
    }

    fn field_notes_work() -> presswork_api::Work {
        presswork_api::Work {
            id: uuid::Uuid::from_u128(10),
            organization_id: PRESS_ORGANIZATION,
            key: "field-notes".to_owned(),
            title: "Field Notes".to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            editions: vec![presswork_api::Edition {
                id: uuid::Uuid::from_u128(20),
                year: Some(1455),
                publisher: String::new(),
                language: String::new(),
                translators: Vec::new(),
                standard: true,
                corrections: Vec::new(),
                file: None,
            }],
        }
    }

    #[test]
    fn the_research_workspace_shows_the_library_of_the_organization() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);

        send(&mut app, Message::SelectWorkspace(Workspace::Research));

        assert!(simulator(app.view()).find("Library · Field Press").is_ok());
    }

    #[test]
    fn cite_puts_the_citation_and_the_bibliography_into_the_document() {
        let mut app = app();
        let mut receiver = open_with_chat(&mut app);
        subscribed(&mut app, &server_with("Paper matters."));
        send(
            &mut app,
            Message::Editor(presswork_editor::Action::Move {
                motion: presswork_editor::Motion::DocumentEnd,
                extend: false,
            }),
        );
        send(&mut app, Message::SelectWorkspace(Workspace::Research));
        let _ = drained(&mut receiver);

        send(
            &mut app,
            Message::Library(library_view::Message::Cite(field_notes_hit())),
        );

        assert_eq!(app.workspace, Workspace::Write);
        assert_eq!(
            app.editor.document().buffer().to_string(),
            "Paper matters.@field-notes[p. iii]\n#bibliography(\"library.yml\")\n"
        );
        assert!(
            drained(&mut receiver)
                .iter()
                .any(|message| matches!(message, ClientMessage::Updates { .. }))
        );

        send(
            &mut app,
            Message::Library(library_view::Message::Cite(field_notes_hit())),
        );
        assert_eq!(
            app.editor
                .document()
                .buffer()
                .to_string()
                .matches("#bibliography")
                .count(),
            1
        );
    }

    #[test]
    fn the_works_of_the_library_become_the_bibliography_of_the_document() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);
        subscribed(
            &mut app,
            &server_with(
                "#set page(width: 200pt, height: auto)\n\
                 See @field-notes[p. 3].\n\
                 #bibliography(\"library.yml\")\n",
            ),
        );

        send(
            &mut app,
            Message::Library(library_view::Message::Loaded(Ok(vec![field_notes_work()]))),
        );
        app.compile_now();

        let errors: Vec<&str> = app
            .compiled
            .as_ref()
            .unwrap()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert_eq!(errors, Vec::<&str>::new());
        assert_eq!(app.preview.page_count(), 1);
    }

    #[test]
    fn a_live_change_of_a_work_reaches_the_library_and_the_bibliography() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);
        subscribed(
            &mut app,
            &server_with(
                "#set page(width: 200pt, height: auto)\n\
                 See @field-notes[p. 3].\n\
                 #bibliography(\"library.yml\")\n",
            ),
        );
        send(
            &mut app,
            Message::Library(library_view::Message::Loaded(Ok(Vec::new()))),
        );

        send(
            &mut app,
            Message::Live(live_session::Event::Received(ServerMessage::WorkChanged {
                work: Box::new(field_notes_work()),
            })),
        );
        app.compile_now();

        assert_eq!(app.library.as_ref().unwrap().works().unwrap().len(), 1);
        let errors = app
            .compiled
            .as_ref()
            .unwrap()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        assert_eq!(errors, 0);
    }

    #[test]
    fn the_plan_workspace_shows_the_tasks_of_the_server_project() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);

        send(&mut app, Message::SelectWorkspace(Workspace::Plan));
        assert!(simulator(app.view()).find("Tasks · Field Notes").is_ok());
        assert!(
            simulator(app.view())
                .find("Presswork gets the tasks.")
                .is_ok()
        );
        let task = presswork_api::Task {
            id: uuid::Uuid::from_u128(5),
            project_id: uuid::Uuid::from_u128(2),
            title: "Proof the cover".to_owned(),
            description: String::new(),
            state: presswork_api::TaskState::ToDo,
            assignee_id: None,
            due_date: None,
            created_by: None,
            created_at: jiff::Timestamp::UNIX_EPOCH,
            updated_at: jiff::Timestamp::UNIX_EPOCH,
        };
        send(
            &mut app,
            Message::Tasks(task_list::Message::Loaded(Ok(vec![task]))),
        );

        assert!(simulator(app.view()).find("Proof the cover").is_ok());
    }

    #[test]
    fn a_local_project_takes_the_place_of_the_server_document_but_keeps_the_chat() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);
        let folder = book();

        open(&mut app, &folder);

        let live = app.live.as_ref().unwrap();
        assert!(live.document().is_none());
        assert!(live.chat().is_some());
        assert!(app.tasks.is_none());
        assert!(app.library.is_none());
        assert!(simulator(app.view()).find("The Book · main.typ").is_ok());
    }

    #[test]
    fn the_chat_and_the_log_panel_do_not_open_together() {
        let mut app = app();
        let _receiver = open_with_chat(&mut app);
        send(&mut app, Message::Chat(chat_panel::Message::Toggle));

        send(&mut app, Message::Logs(log_panel::Message::Toggle));
        assert!(!app.chat.is_open());

        send(&mut app, Message::Chat(chat_panel::Message::Toggle));
        assert!(app.chat.is_open());
        assert!(!app.logs.is_open());
    }

    #[test]
    fn a_lost_connection_shows_offline() {
        let mut app = app();
        let _receiver = open_server_document(&mut app);

        send(&mut app, Message::Live(live_session::Event::Disconnected));

        assert!(simulator(app.view()).find("main.typ · Offline").is_ok());
    }

    const UNSAVED_QUESTION: &str = "Save the changes to main.typ?";

    fn changed_book(app: &mut App) -> tempfile::TempDir {
        let folder = book();
        open(app, &folder);
        send(app, insert("x"));
        folder
    }

    fn control_key(letter: char) -> Message {
        let pressed = keyboard::Key::Character(letter.to_string().into());
        Message::Key(keyboard::Event::KeyPressed {
            key: pressed.clone(),
            modified_key: pressed,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::COMMAND,
            text: None,
            repeat: false,
        })
    }

    fn close_main(app: &mut App) {
        let main = app.main_window;
        send(app, Message::Window(main, chrome::Action::Close));
    }

    #[test]
    fn closing_with_changes_that_are_not_saved_asks_first() {
        let mut app = app();
        let _folder = changed_book(&mut app);

        close_main(&mut app);

        assert!(simulator(app.view()).find(UNSAVED_QUESTION).is_ok());
    }

    #[test]
    fn closing_without_changes_does_not_ask() {
        let folder = book();
        let mut app = app();
        open(&mut app, &folder);

        close_main(&mut app);

        assert!(simulator(app.view()).find(UNSAVED_QUESTION).is_err());
    }

    #[test]
    fn quit_and_control_q_ask_when_there_are_changes() {
        for message in [Message::MenuItem(menu::Item::Quit), control_key('q')] {
            let mut app = app();
            let _folder = changed_book(&mut app);

            send(&mut app, message);

            assert!(simulator(app.view()).find(UNSAVED_QUESTION).is_ok());
        }
    }

    #[test]
    fn cancel_keeps_presswork_open_with_the_changes() {
        let mut app = app();
        let _folder = changed_book(&mut app);
        close_main(&mut app);

        send(&mut app, Message::Unsaved(unsaved::Answer::Cancel));

        assert!(simulator(app.view()).find(UNSAVED_QUESTION).is_err());
        assert!(!app.is_saved());
        assert!(app.editor.document().buffer().to_string().starts_with('x'));
    }

    #[test]
    fn save_in_the_dialog_saves_the_file() {
        let mut app = app();
        let folder = changed_book(&mut app);
        close_main(&mut app);

        send(&mut app, Message::Unsaved(unsaved::Answer::Save));

        assert!(app.is_saved());
        let text = fs::read_to_string(folder.path().join("main.typ")).unwrap();
        assert_eq!(text, "x= Book");
    }

    #[test]
    fn do_not_save_leaves_the_file_as_it_was() {
        let mut app = app();
        let folder = changed_book(&mut app);
        close_main(&mut app);

        send(&mut app, Message::Unsaved(unsaved::Answer::Discard));

        let text = fs::read_to_string(folder.path().join("main.typ")).unwrap();
        assert_eq!(text, "= Book");
    }

    #[test]
    fn a_failed_save_keeps_presswork_open_and_shows_the_problem() {
        let mut app = app();
        let folder = changed_book(&mut app);
        close_main(&mut app);
        fs::remove_dir_all(folder.path()).unwrap();

        send(&mut app, Message::Unsaved(unsaved::Answer::Save));

        assert!(!app.is_saved());
        let mut ui = simulator(app.view());
        assert!(ui.find(UNSAVED_QUESTION).is_err());
        assert!(app.notice.is_some());
    }

    #[test]
    fn the_editor_ignores_typing_while_the_dialog_is_open() {
        let mut app = app();
        let _folder = changed_book(&mut app);
        close_main(&mut app);

        send(&mut app, insert("y"));

        assert!(app.editor.document().buffer().to_string().starts_with("x="));
    }

    #[test]
    fn the_preview_window_closes_without_a_question() {
        let mut app = app();
        let _folder = changed_book(&mut app);
        let preview = detach(&mut app);

        send(&mut app, Message::Window(preview, chrome::Action::Close));

        assert!(simulator(app.view()).find(UNSAVED_QUESTION).is_err());
    }

    #[test]
    fn only_a_window_without_system_buttons_shows_window_buttons() {
        let app = app();
        let mut ui = simulator(app.view());

        let found = ui.find(iced_test::selector::id("window-close")).is_ok();

        assert_eq!(found, Controls::CURRENT == Controls::Custom);
    }
}
