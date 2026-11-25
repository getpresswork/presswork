use std::path::{Path, PathBuf};

use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, padding};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font, font_size, size, space as gap, style};

use crate::i18n::Localizer;

pub(crate) const TOGGLE: &str = "files-toggle";
const INDENT: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Toggle,
    Open(PathBuf),
}

#[derive(Debug)]
pub(crate) struct FilePanel {
    open: bool,
}

impl Default for FilePanel {
    fn default() -> Self {
        Self { open: true }
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
}

pub(crate) fn view<'a>(
    localizer: &Localizer,
    files: &'a [PathBuf],
    active: Option<&Path>,
    changed: &[PathBuf],
) -> Element<'a, Message> {
    let rows = tree(files).into_iter().map(|entry| match entry {
        Entry::Folder { name, depth } => folder_row(name, depth),
        Entry::File { path, name, depth } => file_row(
            path,
            name,
            depth,
            active == Some(path),
            changed.iter().any(|changed| changed == path),
        ),
    });
    container(
        column![
            text(localizer.text("files-title")).font(font::UI_SEMIBOLD),
            scrollable(column(rows).spacing(gap::XXS)).height(Fill)
        ]
        .spacing(gap::S),
    )
    .padding([gap::S, gap::S])
    .width(size::PANEL)
    .height(Fill)
    .style(style::chrome)
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
    use iced_test::simulator;

    use super::*;

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
        let mut ui = simulator::simulator(view(
            &Localizer::english(),
            &files,
            Some(Path::new("main.typ")),
            &[],
        ));

        assert!(ui.find("chapters").is_ok());
        ui.click("one.typ").unwrap();

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, [Message::Open(PathBuf::from("chapters/one.typ"))]);
    }
}
