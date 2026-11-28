use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, scrollable, text, tooltip};
use iced::{Element, Fill};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::app::Message;
use crate::chrome;
use crate::i18n::Localizer;
use crate::menu;
use crate::recent::RecentProject;

const CONTENT_WIDTH: f32 = 880.0;
const START_WIDTH: f32 = 240.0;

pub(crate) fn view<'a>(
    localizer: &Localizer,
    projects: &'a [RecentProject],
) -> Element<'a, Message> {
    let start = column![
        heading(localizer.text("home-start")),
        action(
            localizer,
            Icon::FolderOpen,
            "home-open-folder",
            Message::MenuItem(menu::Item::OpenFolder)
        ),
        action(
            localizer,
            Icon::FolderPlus,
            "home-new-project",
            Message::MenuItem(menu::Item::NewProject)
        ),
        action(
            localizer,
            Icon::Server,
            "home-connect",
            Message::MenuItem(menu::Item::Connect)
        ),
        action(
            localizer,
            Icon::NotebookPen,
            "home-scratchpad",
            Message::OpenScratchpad
        ),
    ]
    .spacing(gap::XS)
    .width(START_WIDTH);
    let mut recent = column![heading(localizer.text("home-recent"))].spacing(gap::XS);
    if projects.is_empty() {
        recent = recent.push(text(localizer.text("home-no-recent")).style(style::muted));
    }
    for (index, project) in projects.iter().enumerate() {
        recent = recent.push(project_row(localizer, index, project));
    }
    let content =
        container(row![start, recent.width(Fill)].spacing(gap::XL)).max_width(CONTENT_WIDTH);
    scrollable(
        container(content)
            .padding(gap::XL)
            .width(Fill)
            .align_x(Horizontal::Center),
    )
    .height(Fill)
    .into()
}

fn heading<'a>(words: String) -> Element<'a, Message> {
    text(words)
        .font(font::UI_SEMIBOLD)
        .size(font_size::LARGE)
        .into()
}

fn action<'a>(
    localizer: &Localizer,
    shape: Icon,
    text_id: &str,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![
            icon(shape).size(size::ICON_SMALL),
            text(localizer.text(text_id))
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center),
    )
    .width(Fill)
    .padding([gap::XS + gap::XXS, gap::S])
    .style(style::secondary)
    .on_press(message)
    .into()
}

fn project_row<'a>(
    localizer: &Localizer,
    index: usize,
    project: &'a RecentProject,
) -> Element<'a, Message> {
    let mut details = column![
        text(&project.name).font(font::UI_SEMIBOLD),
        text(project.folder.display().to_string())
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::XXS);
    if !project.folder.is_dir() {
        details = details.push(
            text(localizer.text("home-missing"))
                .size(font_size::SMALL)
                .style(style::tone(Tone::Danger)),
        );
    }
    let open = button(details)
        .width(Fill)
        .padding([gap::XS, gap::S])
        .style(style::list_item(false))
        .on_press(Message::OpenRecent(project.folder.clone()));
    let remove = container(chrome::tip(
        button(icon(Icon::X).size(size::ICON_SMALL))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press(Message::ForgetRecent(project.folder.clone())),
        localizer.text("home-remove"),
        tooltip::Position::Left,
    ))
    .id(format!("recent-remove-{index}"));
    row![open, remove]
        .spacing(gap::XS)
        .align_y(Vertical::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator::simulator;
    use iced_test::{Simulator, selector};

    use super::*;

    fn projects() -> Vec<RecentProject> {
        vec![
            RecentProject {
                name: "Field Notes".to_owned(),
                folder: PathBuf::from("books/notes"),
            },
            RecentProject {
                name: "The Press".to_owned(),
                folder: PathBuf::from("books/press"),
            },
        ]
    }

    fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
        ui.into_messages().collect()
    }

    #[test]
    fn a_click_on_a_recent_project_opens_it() {
        let projects = projects();
        let mut ui = simulator(view(&Localizer::english(), &projects));

        assert!(ui.find("Recent projects").is_ok());
        ui.click("The Press").unwrap();

        assert!(
            matches!(messages(ui).as_slice(), [Message::OpenRecent(folder)] if folder == &PathBuf::from("books/press"))
        );
    }

    #[test]
    fn a_recent_project_can_leave_the_list() {
        let projects = projects();
        let mut ui = simulator(view(&Localizer::english(), &projects));

        ui.click(selector::id("recent-remove-0")).unwrap();

        assert!(
            matches!(messages(ui).as_slice(), [Message::ForgetRecent(folder)] if folder == &PathBuf::from("books/notes"))
        );
    }

    #[test]
    fn a_folder_shows_as_the_system_writes_it() {
        let folder = std::env::temp_dir().join("books").join("notes");
        let projects = [RecentProject {
            name: "Field Notes".to_owned(),
            folder: folder.clone(),
        }];
        let mut ui = simulator(view(&Localizer::english(), &projects));

        assert!(ui.find(folder.display().to_string()).is_ok());
    }

    #[test]
    fn a_missing_folder_has_a_note() {
        let projects = projects();
        let mut ui = simulator(view(&Localizer::english(), &projects));

        assert!(ui.find("The folder is not there.").is_ok());
    }

    #[test]
    fn the_start_buttons_send_the_commands_of_the_menu() {
        for (label, item) in [
            ("Open folder…", menu::Item::OpenFolder),
            ("New project…", menu::Item::NewProject),
            ("Connect to a server…", menu::Item::Connect),
        ] {
            let mut ui = simulator(view(&Localizer::english(), &[]));

            ui.click(label).unwrap();

            assert!(
                matches!(messages(ui).as_slice(), [Message::MenuItem(sent)] if *sent == item),
                "{label}"
            );
        }
    }

    #[test]
    fn the_scratchpad_button_opens_the_scratchpad() {
        let mut ui = simulator(view(&Localizer::english(), &[]));

        ui.click("Scratchpad").unwrap();

        assert!(matches!(messages(ui).as_slice(), [Message::OpenScratchpad]));
    }

    #[test]
    fn without_recent_projects_a_note_tells_what_shows_here() {
        let mut ui = simulator(view(&Localizer::english(), &[]));

        assert!(ui.find("The projects that you open show here.").is_ok());
    }
}
