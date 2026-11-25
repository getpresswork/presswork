use std::path::{Path, PathBuf};

use iced::alignment::Vertical;
use iced::keyboard::{self, key};
use iced::widget::{
    button, column, container, opaque, row, scrollable, space, text, text_input, tooltip,
};
use iced::{Element, Event, Fill, Theme, event, padding, window};
use presswork_project::MANIFEST;
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::chrome;
use crate::files::display_path;
use crate::i18n::{FluentArgs, Localizer};

pub(crate) const TOGGLE: &str = "files-toggle";
pub(crate) const NAME_INPUT: &str = "files-name";
const NEW: &str = "files-new-button";
const RENAME: &str = "files-rename-button";
const DELETE: &str = "files-delete-button";
const INDENT: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Toggle,
    Open(PathBuf),
    New,
    Rename(PathBuf),
    Delete(PathBuf),
    Name(String),
    Submit,
    ConfirmDelete,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Editing {
    New,
    Rename(PathBuf),
}

#[derive(Debug)]
pub(crate) struct FilePanel {
    open: bool,
    editing: Option<Editing>,
    name: String,
    error: Option<String>,
    deleting: Option<PathBuf>,
}

impl Default for FilePanel {
    fn default() -> Self {
        Self {
            open: true,
            editing: None,
            name: String::new(),
            error: None,
            deleting: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Entry<'a> {
    Folder {
        name: String,
        depth: usize,
    },
    File {
        path: &'a Path,
        name: String,
        depth: usize,
    },
}

impl FilePanel {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn toggle(&mut self) {
        self.open = !self.open;
        self.cancel();
    }

    pub(crate) fn start(&mut self, editing: Editing, name: String) {
        self.open = true;
        self.editing = Some(editing);
        self.name = name;
        self.error = None;
        self.deleting = None;
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = name;
        self.error = None;
    }

    pub(crate) fn editing(&self) -> Option<(&Editing, &str)> {
        self.editing
            .as_ref()
            .map(|editing| (editing, self.name.as_str()))
    }

    pub(crate) fn fail(&mut self, error: String) {
        self.error = Some(error);
    }

    pub(crate) fn ask_delete(&mut self, path: PathBuf) {
        self.cancel();
        self.deleting = Some(path);
    }

    pub(crate) fn deleting(&self) -> Option<&Path> {
        self.deleting.as_deref()
    }

    pub(crate) fn take_deleting(&mut self) -> Option<PathBuf> {
        self.deleting.take()
    }

    pub(crate) fn cancel(&mut self) {
        self.editing = None;
        self.name.clear();
        self.error = None;
        self.deleting = None;
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.editing.is_some() || self.deleting.is_some()
    }

    pub(crate) fn toggle_button(&self, localizer: &Localizer) -> Element<'_, Message> {
        let content = row![
            icon(Icon::Files).size(size::ICON_SMALL),
            text(localizer.text("files-toggle")).size(font_size::SMALL)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center);
        container(
            button(content)
                .padding([gap::XXS, gap::S])
                .style(style::tab(self.open))
                .on_press(Message::Toggle),
        )
        .id(TOGGLE)
        .into()
    }

    pub(crate) fn view<'a>(
        &'a self,
        localizer: &Localizer,
        files: &'a [PathBuf],
        main: &Path,
        active: Option<&Path>,
        changed: &[PathBuf],
    ) -> Element<'a, Message> {
        let rows = tree(files).into_iter().map(|entry| match entry {
            Entry::Folder { name, depth } => folder_row(name, depth),
            Entry::File { path, name, depth } => {
                let active = active == Some(path);
                let actions = (active && !is_fixed(path, main)).then(|| actions(localizer, path));
                file_row(
                    path,
                    name,
                    depth,
                    active,
                    changed.iter().any(|changed| changed == path),
                    actions,
                )
            }
        });
        let header = row![
            text(localizer.text("files-title")).font(font::UI_SEMIBOLD),
            space::horizontal(),
            icon_button(
                Icon::FilePlus,
                localizer.text("files-new"),
                Message::New,
                NEW
            ),
        ]
        .align_y(Vertical::Center);
        let mut content = column![header].spacing(gap::S);
        if self.editing.is_some() {
            content = content.push(self.name_field(localizer));
        }
        container(content.push(scrollable(column(rows).spacing(gap::XXS)).height(Fill)))
            .padding([gap::S, gap::S])
            .width(size::PANEL)
            .height(Fill)
            .style(style::chrome)
            .into()
    }

    fn name_field(&self, localizer: &Localizer) -> Element<'_, Message> {
        let field = text_input(&localizer.text("files-name-placeholder"), &self.name)
            .id(NAME_INPUT)
            .on_input(Message::Name)
            .on_submit(Message::Submit)
            .size(font_size::SMALL)
            .padding([gap::XS, gap::S])
            .style(style::field);
        match &self.error {
            Some(error) => column![
                field,
                text(error.clone())
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Danger))
            ]
            .spacing(gap::XS)
            .into(),
            None => field.into(),
        }
    }
}

pub(crate) fn is_fixed(path: &Path, main: &Path) -> bool {
    path == main || path == Path::new(MANIFEST)
}

pub(crate) fn delete_view<'a>(localizer: &Localizer, path: &Path) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("file", display_path(path));
    let buttons = row![
        space::horizontal(),
        dialog_button(
            localizer.text("files-cancel"),
            Message::Cancel,
            style::secondary
        ),
        dialog_button(
            localizer.text("files-delete-confirm"),
            Message::ConfirmDelete,
            style::primary
        ),
    ]
    .spacing(gap::S);
    let card = container(
        column![
            text(localizer.format("files-delete-title", &args))
                .font(font::UI_SEMIBOLD)
                .size(font_size::LARGE),
            text(localizer.text("files-delete-body")).style(style::muted),
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

pub(crate) fn panel_key(
    event: Event,
    _status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key: pressed, .. }) = event else {
        return None;
    };
    (pressed == keyboard::Key::Named(key::Named::Escape)).then_some(Message::Cancel)
}

fn actions<'a>(localizer: &Localizer, path: &Path) -> Element<'a, Message> {
    row![
        icon_button(
            Icon::Pencil,
            localizer.text("files-rename"),
            Message::Rename(path.to_path_buf()),
            RENAME
        ),
        icon_button(
            Icon::Trash2,
            localizer.text("files-delete"),
            Message::Delete(path.to_path_buf()),
            DELETE
        ),
    ]
    .spacing(gap::XXS)
    .into()
}

fn icon_button<'a>(
    shape: Icon,
    words: String,
    message: Message,
    id: &'static str,
) -> Element<'a, Message> {
    let button = button(icon(shape).size(size::ICON_SMALL))
        .padding(gap::XXS)
        .style(style::ghost)
        .on_press(message);
    container(chrome::tip(button, words, tooltip::Position::Bottom))
        .id(id)
        .into()
}

fn dialog_button<'a>(
    words: String,
    message: Message,
    look: fn(&Theme, button::Status) -> button::Style,
) -> Element<'a, Message> {
    button(text(words))
        .padding([gap::XS + gap::XXS, gap::M])
        .style(look)
        .on_press(message)
        .into()
}

fn tree(files: &[PathBuf]) -> Vec<Entry<'_>> {
    let mut entries = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    for path in files {
        let parts: Vec<String> = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let Some((name, parents)) = parts.split_last() else {
            continue;
        };
        let common = folders
            .iter()
            .zip(parents)
            .take_while(|(open, parent)| open == parent)
            .count();
        folders.truncate(common);
        for (depth, folder) in parents.iter().enumerate().skip(common) {
            entries.push(Entry::Folder {
                name: folder.clone(),
                depth,
            });
            folders.push(folder.clone());
        }
        entries.push(Entry::File {
            path,
            name: name.clone(),
            depth: parents.len(),
        });
    }
    entries
}

fn indent(depth: usize) -> f32 {
    f32::from(u16::try_from(depth).unwrap_or(u16::MAX)) * INDENT
}

fn folder_row<'a>(name: String, depth: usize) -> Element<'a, Message> {
    container(
        row![
            icon(Icon::Folder)
                .size(size::ICON_SMALL)
                .style(style::muted),
            text(name).size(font_size::BODY).style(style::muted)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center),
    )
    .padding(
        padding::left(indent(depth) + gap::XS)
            .top(gap::XXS)
            .bottom(gap::XXS),
    )
    .into()
}

fn file_row<'a>(
    path: &Path,
    name: String,
    depth: usize,
    active: bool,
    changed: bool,
    actions: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut content = row![
        icon(Icon::FileText).size(size::ICON_SMALL),
        text(name)
            .size(font_size::BODY)
            .wrapping(text::Wrapping::None)
    ]
    .spacing(gap::XS)
    .align_y(Vertical::Center);
    if changed {
        content = content.push(text("•").size(font_size::BODY).style(style::muted));
    }
    if let Some(actions) = actions {
        content = content.push(space::horizontal()).push(actions);
    }
    button(content)
        .width(Fill)
        .padding(
            padding::left(indent(depth) + gap::XS)
                .right(gap::XS)
                .top(gap::XXS)
                .bottom(gap::XXS),
        )
        .style(style::list_item(active))
        .on_press(Message::Open(path.to_path_buf()))
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key;
    use iced_test::{Simulator, selector, simulator};

    use super::*;
    use crate::testing::key_press;

    fn files() -> Vec<PathBuf> {
        [
            "chapters/one.typ",
            "chapters/two.typ",
            "main.typ",
            "presswork.toml",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect()
    }

    fn panel_view<'a>(
        panel: &'a FilePanel,
        files: &'a [PathBuf],
        active: &str,
    ) -> Element<'a, Message> {
        panel.view(
            &Localizer::english(),
            files,
            Path::new("main.typ"),
            Some(Path::new(active)),
            &[],
        )
    }

    fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
        ui.into_messages().collect()
    }

    #[test]
    fn the_tree_puts_each_file_under_its_folder() {
        let files = files();

        let names: Vec<(String, usize)> = tree(&files)
            .into_iter()
            .map(|entry| match entry {
                Entry::Folder { name, depth } => (format!("{name}/"), depth),
                Entry::File { name, depth, .. } => (name, depth),
            })
            .collect();

        assert_eq!(
            names,
            [
                ("chapters/".to_owned(), 0),
                ("one.typ".to_owned(), 1),
                ("two.typ".to_owned(), 1),
                ("main.typ".to_owned(), 0),
                ("presswork.toml".to_owned(), 0),
            ]
        );
    }

    #[test]
    fn a_click_on_a_file_opens_it() {
        let files = files();
        let panel = FilePanel::default();
        let mut ui = simulator::simulator(panel_view(&panel, &files, "main.typ"));

        assert!(ui.find("chapters").is_ok());
        ui.click("one.typ").unwrap();

        assert_eq!(
            messages(ui),
            [Message::Open(PathBuf::from("chapters/one.typ"))]
        );
    }

    #[test]
    fn the_new_file_button_starts_a_new_file() {
        let files = files();
        let panel = FilePanel::default();
        let mut ui = simulator::simulator(panel_view(&panel, &files, "main.typ"));

        ui.click(selector::id(NEW)).unwrap();

        assert_eq!(messages(ui), [Message::New]);
    }

    #[test]
    fn the_open_file_has_buttons_to_rename_and_delete_it() {
        let files = files();
        let panel = FilePanel::default();
        let path = PathBuf::from("chapters/one.typ");

        for (button, message) in [
            (RENAME, Message::Rename(path.clone())),
            (DELETE, Message::Delete(path.clone())),
        ] {
            let mut ui = simulator::simulator(panel_view(&panel, &files, "chapters/one.typ"));

            ui.click(selector::id(button)).unwrap();

            assert_eq!(messages(ui), [message]);
        }
    }

    #[test]
    fn the_main_file_and_the_manifest_have_no_buttons_to_rename_or_delete() {
        let files = files();
        let panel = FilePanel::default();

        for active in ["main.typ", "presswork.toml"] {
            let mut ui = simulator::simulator(panel_view(&panel, &files, active));

            assert!(ui.find(selector::id(RENAME)).is_err(), "{active}");
            assert!(ui.find(selector::id(DELETE)).is_err(), "{active}");
        }
    }

    #[test]
    fn the_name_field_shows_with_its_problem() {
        let files = files();
        let mut panel = FilePanel::default();
        panel.start(Editing::New, "chapters/".to_owned());
        panel.fail("A problem.".to_owned());

        let mut ui = simulator::simulator(panel_view(&panel, &files, "main.typ"));

        assert!(ui.find(selector::id(NAME_INPUT)).is_ok());
        assert!(ui.find("A problem.").is_ok());
    }

    #[test]
    fn a_new_name_removes_the_problem() {
        let mut panel = FilePanel::default();
        panel.start(Editing::New, String::new());
        panel.fail("A problem.".to_owned());

        panel.set_name("two.typ".to_owned());

        assert_eq!(panel.error, None);
        assert_eq!(panel.editing(), Some((&Editing::New, "two.typ")));
    }

    #[test]
    fn closing_the_panel_stops_the_name_field() {
        let mut panel = FilePanel::default();
        panel.start(Editing::New, String::new());

        panel.toggle();

        assert!(!panel.is_busy());
    }

    #[test]
    fn the_delete_question_names_the_file_and_sends_the_answers() {
        for (label, message) in [
            ("Delete", Message::ConfirmDelete),
            ("Cancel", Message::Cancel),
        ] {
            let mut ui = simulator::simulator(delete_view(
                &Localizer::english(),
                Path::new("chapters/one.typ"),
            ));

            assert!(ui.find("Delete chapters/one.typ?").is_ok());
            ui.click(label).unwrap();

            assert_eq!(messages(ui), [message], "{label}");
        }
    }

    #[test]
    fn escape_cancels_the_name_field_and_the_question() {
        let id = window::Id::unique();

        assert_eq!(
            panel_key(key_press(key::Named::Escape), event::Status::Captured, id),
            Some(Message::Cancel)
        );
        assert_eq!(
            panel_key(key_press(key::Named::Enter), event::Status::Ignored, id),
            None
        );
    }
}
