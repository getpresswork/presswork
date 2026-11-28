use std::sync::Arc;

use iced::alignment::Vertical;
use iced::widget::{button, column, container, markdown, row, rule, scrollable, text, text_input};
use iced::{Element, Fill, Task, Theme};
use presswork_toolchain::{PackageEntry, PackageIndex, Packages};
use presswork_ui::{font, font_size, space as gap, style};

use crate::i18n::{FluentArgs, Localizer};

pub(crate) const SEARCH_INPUT: &str = "packages-search";
const NAMESPACE: &str = "preview";
const SHOWN: usize = 100;
const LIST_WIDTH: f32 = 240.0;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Query(String),
    Loaded(Result<Arc<PackageIndex>, String>),
    Retry,
    Select(String),
    Readme {
        name: String,
        version: String,
        result: Result<Option<String>, String>,
    },
    Insert,
    Copy,
    Link(markdown::Uri),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    Insert(String),
    Copy(String),
    Open(String),
}

#[derive(Debug, Default)]
pub(crate) struct PackagesView {
    index: Index,
    query: String,
    selected: Option<String>,
    readme: Readme,
}

#[derive(Debug, Default)]
enum Index {
    #[default]
    Empty,
    Loading,
    Loaded(Arc<PackageIndex>),
    Unavailable,
    Failed(String),
}

#[derive(Debug, Default)]
enum Readme {
    #[default]
    Empty,
    Loading(String),
    Loaded {
        key: String,
        content: markdown::Content,
    },
    Missing(String),
    Failed {
        key: String,
        error: String,
    },
}

impl Readme {
    fn key(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Loading(key)
            | Self::Missing(key)
            | Self::Loaded { key, .. }
            | Self::Failed { key, .. } => Some(key),
        }
    }
}

impl PackagesView {
    #[cfg(test)]
    pub(crate) fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub(crate) fn open(&mut self, packages: Option<&Packages>) -> Task<Message> {
        match self.index {
            Index::Empty => self.load(packages),
            _ => Task::none(),
        }
    }

    pub(crate) fn show(&mut self, name: &str, packages: Option<&Packages>) -> Task<Message> {
        name.clone_into(&mut self.query);
        let load = self.open(packages);
        Task::batch([load, self.select(name.to_owned(), packages)])
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        packages: Option<&Packages>,
    ) -> (Task<Message>, Option<Request>) {
        let task = match message {
            Message::Query(query) => {
                self.query = query;
                Task::none()
            }
            Message::Loaded(Ok(index)) => {
                self.index = Index::Loaded(index);
                self.readme_task(packages)
            }
            Message::Loaded(Err(error)) => {
                self.index = Index::Failed(error);
                Task::none()
            }
            Message::Retry => self.load(packages),
            Message::Select(name) => self.select(name, packages),
            Message::Readme {
                name,
                version,
                result,
            } => {
                self.readme_found(format!("{name}:{version}"), result);
                Task::none()
            }
            Message::Insert => return (Task::none(), self.import_line().map(Request::Insert)),
            Message::Copy => return (Task::none(), self.import_line().map(Request::Copy)),
            Message::Link(url) => return (Task::none(), Some(Request::Open(url))),
        };
        (task, None)
    }

    fn load(&mut self, packages: Option<&Packages>) -> Task<Message> {
        let Some(packages) = packages.cloned() else {
            self.index = Index::Unavailable;
            return Task::none();
        };
        self.index = Index::Loading;
        Task::perform(
            async move {
                packages
                    .index(NAMESPACE)
                    .await
                    .map(|infos| Arc::new(PackageIndex::new(infos)))
                    .map_err(|error| error.to_string())
            },
            Message::Loaded,
        )
    }

    fn select(&mut self, name: String, packages: Option<&Packages>) -> Task<Message> {
        self.selected = Some(name);
        self.readme_task(packages)
    }

    fn selected_entry(&self) -> Option<&PackageEntry> {
        let Index::Loaded(index) = &self.index else {
            return None;
        };
        index.entry(self.selected.as_deref()?)
    }

    fn selected_key(&self) -> Option<String> {
        self.selected_entry()
            .map(|entry| format!("{}:{}", entry.info.name, entry.info.version))
    }

    fn readme_task(&mut self, packages: Option<&Packages>) -> Task<Message> {
        let Some(entry) = self.selected_entry() else {
            return Task::none();
        };
        let (name, version) = (entry.info.name.clone(), entry.info.version.clone());
        let key = format!("{name}:{version}");
        if self.readme.key() == Some(key.as_str()) {
            return Task::none();
        }
        let Some(packages) = packages.cloned() else {
            self.readme = Readme::Empty;
            return Task::none();
        };
        self.readme = Readme::Loading(key);
        Task::perform(
            async move {
                let result = packages
                    .readme(NAMESPACE, &name, &version)
                    .await
                    .map_err(|error| error.to_string());
                (name, version, result)
            },
            |(name, version, result)| Message::Readme {
                name,
                version,
                result,
            },
        )
    }

    fn readme_found(&mut self, key: String, result: Result<Option<String>, String>) {
        if self.selected_key().as_deref() != Some(key.as_str()) {
            return;
        }
        self.readme = match result {
            Ok(Some(text)) => Readme::Loaded {
                content: markdown::Content::parse(&text),
                key,
            },
            Ok(None) => Readme::Missing(key),
            Err(error) => Readme::Failed { key, error },
        };
    }

    fn import_line(&self) -> Option<String> {
        self.selected_entry().map(|entry| {
            format!(
                "#import \"@{NAMESPACE}/{}:{}\"",
                entry.info.name, entry.info.version
            )
        })
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer, theme: &Theme) -> Element<'a, Message> {
        let search = text_input(&localizer.text("packages-search"), &self.query)
            .id(SEARCH_INPUT)
            .on_input(Message::Query)
            .size(font_size::SMALL)
            .padding([gap::XS, gap::S])
            .style(style::field);
        let body = match &self.index {
            Index::Empty | Index::Loading => note(localizer.text("packages-loading")),
            Index::Unavailable => note(localizer.text("packages-unavailable")),
            Index::Failed(error) => {
                let mut args = FluentArgs::new();
                args.set("error", error.clone());
                column![
                    note(localizer.format("packages-failed", &args)),
                    container(
                        button(text(localizer.text("packages-retry")))
                            .style(style::secondary)
                            .on_press(Message::Retry)
                    )
                    .padding([0.0, gap::S])
                ]
                .into()
            }
            Index::Loaded(index) => self.browser(index, localizer, theme),
        };
        column![container(search).padding(gap::S), hairline(), body]
            .height(Fill)
            .into()
    }

    fn browser<'a>(
        &'a self,
        index: &'a PackageIndex,
        localizer: &Localizer,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let found = index.search(&self.query);
        let mut args = FluentArgs::new();
        args.set("found", found.len());
        args.set("total", index.len());
        args.set("shown", SHOWN);
        let mut list = column![
            text(localizer.format("packages-count", &args))
                .size(font_size::SMALL)
                .style(style::muted)
        ]
        .spacing(gap::XXS)
        .padding(gap::S);
        if found.len() > SHOWN {
            list = list.push(
                text(localizer.format("packages-more", &args))
                    .size(font_size::SMALL)
                    .style(style::muted),
            );
        }
        for entry in found.into_iter().take(SHOWN) {
            let selected = self.selected.as_deref() == Some(entry.info.name.as_str());
            list = list.push(package_row(entry, selected));
        }
        let details = match self.selected_entry() {
            Some(entry) => self.details(entry, localizer, theme),
            None => note(localizer.text("packages-pick")),
        };
        row![
            scrollable(list).width(LIST_WIDTH).height(Fill),
            rule::vertical(1).style(style::hairline),
            scrollable(container(details).padding(gap::M).width(Fill))
                .width(Fill)
                .height(Fill),
        ]
        .into()
    }

    fn details<'a>(
        &'a self,
        entry: &'a PackageEntry,
        localizer: &Localizer,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let info = &entry.info;
        let mut content = column![
            row![
                text(&info.name)
                    .font(font::UI_SEMIBOLD)
                    .size(font_size::TITLE),
                text(&info.version).style(style::muted)
            ]
            .spacing(gap::S)
            .align_y(Vertical::Center)
        ]
        .spacing(gap::S);
        if let Some(description) = &info.description {
            content = content.push(text(description));
        }
        let mut args = FluentArgs::new();
        let mut facts = Vec::new();
        if !info.authors.is_empty() {
            args.set("authors", info.authors.join(", "));
            facts.push(localizer.format("packages-by", &args));
        }
        if let Some(license) = &info.license {
            args.set("license", license.clone());
            facts.push(localizer.format("packages-license", &args));
        }
        if let Some(compiler) = &info.compiler {
            args.set("version", compiler.clone());
            facts.push(localizer.format("packages-compiler", &args));
        }
        args.set("versions", entry.versions.join(", "));
        facts.push(localizer.format("packages-versions", &args));
        for fact in facts {
            content = content.push(text(fact).size(font_size::SMALL).style(style::muted));
        }
        let mut actions = row![
            button(text(localizer.text("packages-insert")))
                .style(style::primary)
                .on_press(Message::Insert),
            button(text(localizer.text("packages-copy")))
                .style(style::secondary)
                .on_press(Message::Copy),
        ]
        .spacing(gap::S);
        if let Some(url) = info.repository.as_ref().or(info.homepage.as_ref()) {
            actions = actions.push(
                button(text(localizer.text("packages-repository")))
                    .style(style::ghost)
                    .on_press(Message::Link(url.clone())),
            );
        }
        content
            .push(actions.wrap().vertical_spacing(gap::S))
            .push(hairline())
            .push(self.readme_view(localizer, theme))
            .into()
    }

    fn readme_view<'a>(&'a self, localizer: &Localizer, theme: &Theme) -> Element<'a, Message> {
        match &self.readme {
            Readme::Loaded { content, .. } => {
                crate::markdown::view(content.items(), theme).map(Message::Link)
            }
            Readme::Loading(_) => note(localizer.text("packages-readme-loading")),
            Readme::Missing(_) => note(localizer.text("packages-readme-missing")),
            Readme::Failed { error, .. } => {
                let mut args = FluentArgs::new();
                args.set("error", error.clone());
                note(localizer.format("packages-readme-failed", &args))
            }
            Readme::Empty => column![].into(),
        }
    }
}

fn package_row(entry: &PackageEntry, selected: bool) -> Element<'_, Message> {
    let info = &entry.info;
    let mut content = column![
        row![
            text(&info.name)
                .font(font::UI_SEMIBOLD)
                .wrapping(text::Wrapping::None),
            text(&info.version)
                .size(font_size::SMALL)
                .style(style::muted)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center)
    ]
    .spacing(gap::XXS);
    if let Some(description) = &info.description {
        content = content.push(text(description).size(font_size::SMALL).style(style::muted));
    }
    button(content)
        .width(Fill)
        .padding([gap::XS, gap::S])
        .style(style::list_item(selected))
        .on_press(Message::Select(info.name.clone()))
        .into()
}

fn note<'a>(words: String) -> Element<'a, Message> {
    container(text(words).size(font_size::SMALL).style(style::muted))
        .padding(gap::S)
        .into()
}

fn hairline<'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::hairline).into()
}

#[cfg(test)]
mod tests {
    use iced_test::{Simulator, simulator};
    use presswork_toolchain::{Client, PackageInfo};
    use presswork_ui::theme as ui_theme;

    use super::*;
    use crate::testing::{Route, run_task, serve};

    fn info(name: &str, version: &str, description: &str, authors: &[&str]) -> PackageInfo {
        PackageInfo {
            name: name.to_owned(),
            version: version.to_owned(),
            description: Some(description.to_owned()),
            authors: authors.iter().map(|author| (*author).to_owned()).collect(),
            ..PackageInfo::default()
        }
    }

    fn index() -> Arc<PackageIndex> {
        Arc::new(PackageIndex::new(vec![
            info("cetz", "0.3.4", "Drawing with Typst.", &["Ann", "Bob"]),
            info("cetz", "0.3.3", "Drawing with Typst.", &["Ann", "Bob"]),
            info("cetz-plot", "0.1.0", "Plots.", &[]),
            info("tablex", "0.0.9", "More tables.", &[]),
        ]))
    }

    fn loaded() -> PackagesView {
        let mut view = PackagesView::default();
        let _ = view.update(Message::Loaded(Ok(index())), None);
        view
    }

    fn selected(name: &str) -> PackagesView {
        let mut view = loaded();
        let _ = view.update(Message::Select(name.to_owned()), None);
        view
    }

    fn simulate(view: &PackagesView) -> Simulator<'_, Message> {
        simulator::simulator(view.view(&Localizer::english(), &ui_theme::light()))
    }

    fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
        ui.into_messages().collect()
    }

    fn packages() -> Packages {
        Packages::new(
            Client::new("Presswork tests").unwrap(),
            std::env::temp_dir(),
        )
        .with_server("http://127.0.0.1:9")
    }

    fn readme(name: &str, version: &str, text: &str) -> Message {
        Message::Readme {
            name: name.to_owned(),
            version: version.to_owned(),
            result: Ok(Some(text.to_owned())),
        }
    }

    #[test]
    fn the_list_loads_once_when_the_pane_opens() {
        let mut view = PackagesView::default();
        let packages = packages();

        assert!(view.open(Some(&packages)).units() > 0);
        assert_eq!(view.open(Some(&packages)).units(), 0);
    }

    #[test]
    fn the_search_shows_the_matching_packages() {
        let mut view = loaded();

        let _ = view.update(Message::Query("cetz".to_owned()), None);

        let mut ui = simulate(&view);
        assert!(ui.find("cetz").is_ok());
        assert!(ui.find("cetz-plot").is_ok());
        assert!(ui.find("tablex").is_err());
        assert!(ui.find("2 of 3 packages").is_ok());
    }

    #[test]
    fn a_click_on_a_package_selects_it() {
        let view = loaded();
        let mut ui = simulate(&view);

        ui.click("tablex").unwrap();

        assert!(matches!(messages(ui).as_slice(), [Message::Select(name)] if name == "tablex"));
    }

    #[test]
    fn a_selected_package_shows_its_details() {
        let view = selected("cetz");

        let mut ui = simulate(&view);

        assert!(ui.find("Drawing with Typst.").is_ok());
        assert!(ui.find("By Ann, Bob").is_ok());
        assert!(ui.find("Versions: 0.3.4, 0.3.3").is_ok());
    }

    #[test]
    fn selecting_a_package_asks_for_its_readme() {
        let mut view = loaded();
        let packages = packages();

        let (task, _) = view.update(Message::Select("cetz".to_owned()), Some(&packages));

        assert!(task.units() > 0);
    }

    #[test]
    fn only_the_readme_of_the_selected_package_shows() {
        let mut view = selected("cetz");

        let _ = view.update(readme("tablex", "0.0.9", "Other."), None);
        assert!(!matches!(view.readme, Readme::Loaded { .. }));
        let _ = view.update(readme("cetz", "0.3.4", "# Usage\nDraw things."), None);

        assert!(matches!(&view.readme, Readme::Loaded { key, .. } if key == "cetz:0.3.4"));
    }

    #[test]
    fn the_buttons_give_the_import_line() {
        let mut view = selected("cetz");
        let line = "#import \"@preview/cetz:0.3.4\"".to_owned();

        assert_eq!(
            view.update(Message::Insert, None).1,
            Some(Request::Insert(line.clone()))
        );
        assert_eq!(
            view.update(Message::Copy, None).1,
            Some(Request::Copy(line))
        );
        let mut ui = simulate(&view);
        ui.click("Insert import").unwrap();
        assert!(matches!(messages(ui).as_slice(), [Message::Insert]));
    }

    #[test]
    fn a_failed_list_offers_another_try() {
        let mut view = PackagesView::default();
        let _ = view.update(Message::Loaded(Err("offline".to_owned())), None);

        let mut ui = simulate(&view);

        assert!(
            ui.find("Presswork cannot get the list of packages: offline")
                .is_ok()
        );
        ui.click("Try again").unwrap();
        assert!(matches!(messages(ui).as_slice(), [Message::Retry]));
    }

    #[test]
    fn a_link_asks_to_open_it() {
        let mut view = selected("cetz");

        let (_, request) = view.update(Message::Link("https://typst.app".to_owned()), None);

        assert_eq!(request, Some(Request::Open("https://typst.app".to_owned())));
    }

    #[test]
    fn show_selects_a_package_and_searches_for_it() {
        let mut view = loaded();

        let _ = view.show("cetz-plot", None);

        assert_eq!(view.selected.as_deref(), Some("cetz-plot"));
        assert_eq!(view.query, "cetz-plot");
    }

    const INDEX: &str = r#"[
        {
            "name": "greeting",
            "version": "0.1.5",
            "description": "Says hello.",
            "authors": ["Ann"],
            "license": "MIT",
            "repository": "https://example.com/greeting",
            "compiler": "0.13.0"
        },
        { "name": "plain", "version": "1.0.0" }
    ]"#;

    fn served(root: &std::path::Path) -> Packages {
        let readme = root.join("preview").join("greeting").join("0.1.5");
        std::fs::create_dir_all(&readme).unwrap();
        std::fs::write(readme.join("README.md"), "Say hello to the reader.").unwrap();
        let base = serve(vec![Route::new("GET /preview/index.json", INDEX)]);
        Packages::new(Client::new("Presswork tests").unwrap(), root.to_path_buf())
            .with_server(&base)
    }

    #[test]
    fn the_list_and_the_readme_come_from_the_package_server() {
        let root = tempfile::tempdir().unwrap();
        let packages = served(root.path());
        let mut view = PackagesView::default();

        let loaded = run_task(view.open(Some(&packages)));
        let [Message::Loaded(Ok(index))] = loaded.as_slice() else {
            panic!("{loaded:?}");
        };
        let _ = view.update(Message::Loaded(Ok(Arc::clone(index))), Some(&packages));
        let (task, _) = view.update(Message::Select("greeting".to_owned()), Some(&packages));
        let found = run_task(task);
        let [found] = <[Message; 1]>::try_from(found).unwrap();
        let _ = view.update(found, Some(&packages));

        assert!(matches!(&view.readme, Readme::Loaded { key, .. } if key == "greeting:0.1.5"));
        let mut ui = simulate(&view);
        for words in ["License: MIT", "Needs Typst 0.13.0 or later", "Repository"] {
            assert!(ui.find(words).is_ok(), "{words} is not in the view");
        }
        drop(ui);
        assert!(matches!(
            view.update(Message::Link("https://example.com/greeting".to_owned()), None).1,
            Some(Request::Open(url)) if url == "https://example.com/greeting"
        ));
    }

    #[test]
    fn a_package_without_a_readme_file_says_so() {
        let root = tempfile::tempdir().unwrap();
        let packages = served(root.path());
        let mut view = PackagesView::default();
        let loaded = run_task(view.open(Some(&packages)));
        let [loaded] = <[Message; 1]>::try_from(loaded).unwrap();
        let _ = view.update(loaded, Some(&packages));

        let (task, _) = view.update(Message::Select("plain".to_owned()), Some(&packages));
        assert!(
            simulate(&view)
                .find("Presswork gets the README of the package.")
                .is_ok()
        );
        let found = run_task(task);

        assert!(
            matches!(found.as_slice(), [Message::Readme { result: Err(_), .. }]),
            "{found:?}"
        );
    }

    #[test]
    fn a_missing_or_failed_readme_shows_a_note() {
        let mut view = selected("tablex");

        let _ = view.update(
            Message::Readme {
                name: "tablex".to_owned(),
                version: "0.0.9".to_owned(),
                result: Ok(None),
            },
            None,
        );
        assert!(
            simulate(&view)
                .find("The package has no README file.")
                .is_ok()
        );
        let _ = view.update(
            Message::Readme {
                name: "tablex".to_owned(),
                version: "0.0.9".to_owned(),
                result: Err("the server answered with the HTTP status 404".to_owned()),
            },
            None,
        );

        assert!(
            simulate(&view)
                .find(
                    "Presswork cannot get the README: the server answered with the HTTP status 404"
                )
                .is_ok()
        );
    }

    #[test]
    fn retry_loads_the_list_again() {
        let mut view = PackagesView::default();
        let packages = packages();
        let _ = view.update(Message::Loaded(Err("timeout".to_owned())), Some(&packages));

        let (task, _) = view.update(Message::Retry, Some(&packages));

        assert!(task.units() > 0);
        assert!(
            simulate(&view)
                .find("Presswork gets the list of packages from Typst Universe.")
                .is_ok()
        );
    }

    #[test]
    fn without_a_package_folder_the_list_is_not_available() {
        let mut view = PackagesView::default();

        let task = view.open(None);
        let (readme, _) = view.update(Message::Select("cetz".to_owned()), None);

        assert_eq!(task.units(), 0);
        assert_eq!(readme.units(), 0);
        assert!(
            simulate(&view)
                .find("The list of packages is not available.")
                .is_ok()
        );
    }

    #[test]
    fn a_long_list_shows_only_the_first_packages() {
        let mut view = PackagesView::default();
        let infos = (0..=SHOWN)
            .map(|number| info(&format!("package-{number}"), "1.0.0", "A package.", &[]))
            .collect();

        let _ = view.update(
            Message::Loaded(Ok(Arc::new(PackageIndex::new(infos)))),
            None,
        );

        assert!(
            simulate(&view)
                .find("The list shows the first 100. Type more words to find a package.")
                .is_ok()
        );
    }
}
