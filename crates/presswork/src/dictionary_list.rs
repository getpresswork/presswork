use std::collections::BTreeMap;
use std::path::PathBuf;

use futures::SinkExt;
use futures::channel::mpsc;
use iced::alignment::Vertical;
use iced::futures::Stream;
use iced::widget::{button, column, progress_bar, row, scrollable, space, text};
use iced::{Element, Fill, Task};
use presswork_toolchain::{Client, Dictionaries, DictionarySource};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::i18n::{FluentArgs, Localizer};

const ROW_HEIGHT: f32 = 28.0;
const PROGRESS_LENGTH: f32 = 60.0;
const PROGRESS_GIRTH: f32 = 4.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Install(String),
    Progress(String, u8),
    Installed(String, Result<(), String>),
    Remove(String),
}

impl Message {
    pub(crate) fn changes_dictionaries(&self) -> bool {
        matches!(self, Self::Installed(_, Ok(())) | Self::Remove(_))
    }
}

#[derive(Debug)]
pub(crate) struct DictionaryList {
    dictionaries: Option<Dictionaries>,
    root: Option<PathBuf>,
    sources: Vec<DictionarySource>,
    installed: Vec<String>,
    installing: BTreeMap<String, u8>,
    problem: Option<Problem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    Install(String, String),
    Remove(String, String),
}

impl DictionaryList {
    pub(crate) fn new(root: Option<PathBuf>) -> Self {
        let catalog = presswork_toolchain::catalog();
        if let Err(error) = &catalog {
            tracing::error!(%error, "The list of dictionaries is not valid");
        }
        let (sources, dictionaries) = match (catalog, &root) {
            (Ok(catalog), Some(root)) => {
                let dictionaries = Dictionaries::new(root, &catalog);
                (catalog.sources, Some(dictionaries))
            }
            (Ok(catalog), None) => (catalog.sources, None),
            (Err(_), _) => (Vec::new(), None),
        };
        Self::with(dictionaries, root, sources)
    }

    fn with(
        dictionaries: Option<Dictionaries>,
        root: Option<PathBuf>,
        sources: Vec<DictionarySource>,
    ) -> Self {
        let mut list = Self {
            dictionaries,
            root,
            sources,
            installed: Vec::new(),
            installing: BTreeMap::new(),
            problem: None,
        };
        list.refresh();
        list
    }

    pub(crate) fn root(&self) -> Option<PathBuf> {
        self.root.clone()
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        client: Result<Client, String>,
    ) -> Task<Message> {
        match message {
            Message::Install(tag) => return self.install(tag, client),
            Message::Progress(tag, percent) => {
                self.installing.insert(tag, percent);
            }
            Message::Installed(tag, result) => {
                self.installing.remove(&tag);
                if let Err(error) = result {
                    self.problem = Some(Problem::Install(tag, error));
                }
                self.refresh();
            }
            Message::Remove(tag) => {
                self.problem = None;
                if let Some(dictionaries) = &self.dictionaries
                    && let Err(error) = dictionaries.remove(&tag)
                {
                    tracing::warn!(%error, %tag, "The dictionary was not removed");
                    self.problem = Some(Problem::Remove(tag, error.to_string()));
                }
                self.refresh();
            }
        }
        Task::none()
    }

    fn install(&mut self, tag: String, client: Result<Client, String>) -> Task<Message> {
        self.problem = None;
        let (Some(dictionaries), Some(source)) = (
            self.dictionaries.clone(),
            self.sources
                .iter()
                .find(|source| source.tag == tag)
                .cloned(),
        ) else {
            return Task::none();
        };
        let client = match client {
            Ok(client) => client,
            Err(error) => {
                self.problem = Some(Problem::Install(tag, error));
                return Task::none();
            }
        };
        self.installing.insert(tag, 0);
        Task::run(install(dictionaries, client, source), |message| message)
    }

    fn refresh(&mut self) {
        self.installed = self
            .dictionaries
            .as_ref()
            .map_or_else(Vec::new, Dictionaries::installed);
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        let rows = self
            .sources
            .iter()
            .map(|source| self.row(localizer, source));
        let mut content = column![
            text(localizer.text("dictionaries-title"))
                .size(font_size::CAPTION)
                .font(font::UI_MEDIUM)
                .style(style::muted)
        ]
        .spacing(gap::S);
        if let Some(problem) = &self.problem {
            content = content.push(
                text(problem_text(localizer, problem))
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Danger)),
            );
        }
        content
            .push(scrollable(column(rows).spacing(gap::XS)).height(Fill))
            .into()
    }

    fn row<'a>(
        &'a self,
        localizer: &Localizer,
        source: &'a DictionarySource,
    ) -> Element<'a, Message> {
        let state: Element<'a, Message> = if let Some(percent) = self.installing.get(&source.tag) {
            row![
                progress_bar(0.0..=100.0, f32::from(*percent))
                    .length(PROGRESS_LENGTH)
                    .girth(PROGRESS_GIRTH),
            ]
            .into()
        } else if self.installed.contains(&source.tag) {
            action(
                Icon::Trash2,
                localizer.text("dictionaries-remove"),
                (self.dictionaries.is_some()).then(|| Message::Remove(source.tag.clone())),
            )
        } else {
            action(
                Icon::Download,
                localizer.text("dictionaries-install"),
                (self.dictionaries.is_some()).then(|| Message::Install(source.tag.clone())),
            )
        };
        row![
            column![
                text(localizer.text(&format!("dictionary-{}", source.tag))).size(font_size::SMALL),
                text(source.license.clone())
                    .size(font_size::CAPTION)
                    .style(style::muted),
            ],
            space().width(Fill),
            state
        ]
        .spacing(gap::S)
        .height(ROW_HEIGHT)
        .align_y(Vertical::Center)
        .into()
    }
}

fn problem_text(localizer: &Localizer, problem: &Problem) -> String {
    let (id, tag, error) = match problem {
        Problem::Install(tag, error) => ("dictionaries-install-error", tag, error),
        Problem::Remove(tag, error) => ("dictionaries-remove-error", tag, error),
    };
    let mut args = FluentArgs::new();
    args.set("language", localizer.text(&format!("dictionary-{tag}")));
    args.set("error", crate::toolchain_panel::sentence(error));
    localizer.format(id, &args)
}

fn action<'a>(symbol: Icon, label: String, message: Option<Message>) -> Element<'a, Message> {
    button(
        row![
            icon(symbol).size(size::ICON_SMALL),
            text(label).size(font_size::SMALL)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center),
    )
    .padding([gap::XXS, gap::S])
    .style(style::ghost)
    .on_press_maybe(message)
    .into()
}

fn install(
    dictionaries: Dictionaries,
    client: Client,
    source: DictionarySource,
) -> impl Stream<Item = Message> {
    iced::stream::channel(16, async move |mut output: mpsc::Sender<Message>| {
        let tag = source.tag.clone();
        tracing::info!(%tag, "Dictionary installation started");
        let mut progress_output = output.clone();
        let progress_tag = tag.clone();
        let mut last = None;
        let result = dictionaries
            .install(&client, &source, move |progress| {
                let percent = crate::toolchain_panel::percent(progress);
                if last != Some(percent) {
                    last = Some(percent);
                    let _ =
                        progress_output.try_send(Message::Progress(progress_tag.clone(), percent));
                }
            })
            .await;
        match &result {
            Ok(()) => tracing::info!(%tag, "Dictionary installed"),
            Err(error) => tracing::warn!(%error, %tag, "Dictionary not installed"),
        }
        let _ = output
            .send(Message::Installed(
                tag,
                result.map_err(|error| error.to_string()),
            ))
            .await;
    })
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_toolchain::{Digest, SourceFile};

    use super::*;
    use crate::testing::{Route, collect, serve};

    const FILES: [(&str, &[u8]); 3] = [
        ("xx/xx.aff", b"SET UTF-8\n"),
        ("xx/xx.dic", b"1\nword\n"),
        ("xx/README.txt", b"The license."),
    ];

    fn source_file(path: &str, bytes: &[u8]) -> SourceFile {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("file");
        std::fs::write(&file, bytes).unwrap();
        SourceFile {
            path: path.to_owned(),
            size: bytes.len() as u64,
            digest: Digest::of_file(&file).unwrap(),
        }
    }

    fn source(dic: &[u8]) -> DictionarySource {
        DictionarySource {
            tag: "xx-XX".to_owned(),
            license: "MIT".to_owned(),
            aff: source_file(FILES[0].0, FILES[0].1),
            dic: source_file(FILES[1].0, dic),
            notice: source_file(FILES[2].0, FILES[2].1),
        }
    }

    fn server() -> String {
        serve(
            FILES
                .iter()
                .map(|(path, bytes)| Route::new(&format!("GET /{path}"), *bytes))
                .collect(),
        )
    }

    fn client() -> Client {
        Client::new("presswork-test").unwrap()
    }

    fn installed(root: &std::path::Path, tag: &str) {
        let folder = root.join(tag);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("dictionary.aff"), "SET UTF-8\n").unwrap();
        std::fs::write(folder.join("dictionary.dic"), "0\n").unwrap();
    }

    #[test]
    fn the_list_shows_each_dictionary_with_its_license() {
        let list = DictionaryList::new(None);
        let localizer = Localizer::english();

        let mut ui = simulator(list.view(&localizer));

        assert!(ui.find("English (United States)").is_ok());
        assert!(ui.find("SCOWL (BSD-style)").is_ok());
        assert!(ui.find("German (Germany)").is_ok());
    }

    #[test]
    fn an_installed_dictionary_can_be_removed() {
        let root = tempfile::tempdir().unwrap();
        installed(root.path(), "en-US");
        let mut list = DictionaryList::new(Some(root.path().to_path_buf()));

        let _ = list.update(Message::Remove("en-US".to_owned()), Err(String::new()));

        assert_eq!(list.installed, Vec::<String>::new());
        assert!(!root.path().join("en-US").exists());
    }

    #[test]
    fn an_installation_shows_its_progress() {
        let root = tempfile::tempdir().unwrap();
        let mut list = DictionaryList::new(Some(root.path().to_path_buf()));

        let _ = list.update(
            Message::Progress("de-DE".to_owned(), 40),
            Err(String::new()),
        );

        assert_eq!(list.installing.get("de-DE"), Some(&40));
    }

    #[test]
    fn a_failed_installation_tells_the_language() {
        let mut list = DictionaryList::new(None);
        let localizer = Localizer::english();

        let _ = list.update(
            Message::Installed(
                "de-DE".to_owned(),
                Err("the server answered with the HTTP status 404".to_owned()),
            ),
            Err(String::new()),
        );

        let mut ui = simulator(list.view(&localizer));
        assert!(
            ui.find("Presswork cannot download the German (Germany) dictionary. The server answered with the HTTP status 404.")
                .is_ok()
        );
    }

    #[test]
    fn each_dictionary_has_an_english_name() {
        let localizer = Localizer::english();
        let list = DictionaryList::new(None);

        assert_eq!(list.sources.len(), 14);
        for source in &list.sources {
            let id = format!("dictionary-{}", source.tag);
            assert_ne!(localizer.text(&id), id);
        }
        for id in [
            "dictionaries-title",
            "dictionaries-install",
            "dictionaries-remove",
            "dictionaries-install-error",
            "dictionaries-remove-error",
        ] {
            assert_ne!(localizer.text(id), id);
        }
    }

    #[test]
    fn only_an_installation_and_a_removal_change_the_dictionaries() {
        let tag = || "de-DE".to_owned();

        assert!(Message::Installed(tag(), Ok(())).changes_dictionaries());
        assert!(Message::Remove(tag()).changes_dictionaries());
        assert!(!Message::Installed(tag(), Err(String::new())).changes_dictionaries());
        assert!(!Message::Install(tag()).changes_dictionaries());
        assert!(!Message::Progress(tag(), 5).changes_dictionaries());
    }

    #[test]
    fn the_list_keeps_its_folder() {
        let root = tempfile::tempdir().unwrap();

        let list = DictionaryList::new(Some(root.path().to_path_buf()));

        assert_eq!(list.root(), Some(root.path().to_path_buf()));
    }

    #[test]
    fn an_installation_without_a_client_shows_the_problem() {
        let root = tempfile::tempdir().unwrap();
        let mut list = DictionaryList::new(Some(root.path().to_path_buf()));

        let _ = list.update(
            Message::Install("de-DE".to_owned()),
            Err("no connection".to_owned()),
        );

        assert_eq!(
            list.problem,
            Some(Problem::Install(
                "de-DE".to_owned(),
                "no connection".to_owned()
            ))
        );
        assert!(list.installing.is_empty());
    }

    #[test]
    fn an_installation_starts_at_zero_percent() {
        let root = tempfile::tempdir().unwrap();
        let mut list = DictionaryList::new(Some(root.path().to_path_buf()));

        let _ = list.update(Message::Install("de-DE".to_owned()), Ok(client()));

        assert_eq!(list.installing.get("de-DE"), Some(&0));
        assert_eq!(list.problem, None);
    }

    #[test]
    fn an_installation_needs_a_folder_and_a_known_language() {
        let root = tempfile::tempdir().unwrap();
        let mut without_folder = DictionaryList::new(None);
        let mut with_folder = DictionaryList::new(Some(root.path().to_path_buf()));

        let _ = without_folder.update(Message::Install("de-DE".to_owned()), Ok(client()));
        let _ = with_folder.update(Message::Install("xx-XX".to_owned()), Ok(client()));

        assert!(without_folder.installing.is_empty());
        assert!(with_folder.installing.is_empty());
    }

    #[test]
    fn the_installation_downloads_the_files_and_tells_the_progress() {
        let root = tempfile::tempdir().unwrap();
        let dictionaries = Dictionaries::with_base(root.path(), &server());

        let messages = collect(install(dictionaries.clone(), client(), source(FILES[1].1)));

        assert_eq!(
            messages.last(),
            Some(&Message::Installed("xx-XX".to_owned(), Ok(())))
        );
        assert!(messages.contains(&Message::Progress("xx-XX".to_owned(), 100)));
        assert_eq!(dictionaries.installed(), ["xx-XX"]);
    }

    #[test]
    fn a_file_with_a_different_digest_stops_the_installation() {
        let root = tempfile::tempdir().unwrap();
        let dictionaries = Dictionaries::with_base(root.path(), &server());

        let messages = collect(install(
            dictionaries.clone(),
            client(),
            source(b"other words"),
        ));

        assert!(matches!(
            messages.last(),
            Some(Message::Installed(tag, Err(_))) if tag == "xx-XX"
        ));
        assert_eq!(dictionaries.installed(), Vec::<String>::new());
    }

    #[test]
    fn the_list_shows_the_installed_and_the_installing_dictionaries() {
        let root = tempfile::tempdir().unwrap();
        installed(root.path(), "xx-XX");
        let dictionaries = Dictionaries::with_base(root.path(), "http://127.0.0.1:9");
        let mut list = DictionaryList::with(
            Some(dictionaries),
            Some(root.path().to_path_buf()),
            vec![source(FILES[1].1), {
                let mut other = source(FILES[1].1);
                other.tag = "de-DE".to_owned();
                other
            }],
        );
        let localizer = Localizer::english();
        let _ = list.update(
            Message::Progress("de-DE".to_owned(), 40),
            Err(String::new()),
        );

        let mut ui = simulator(list.view(&localizer));

        assert!(ui.find("Remove").is_ok());
        assert!(ui.find("Download").is_err());
    }

    #[test]
    fn a_failed_removal_tells_the_language() {
        let localizer = Localizer::english();

        let text = problem_text(
            &localizer,
            &Problem::Remove("de-DE".to_owned(), "the folder is in use".to_owned()),
        );

        assert_eq!(
            text,
            "Presswork cannot remove the German (Germany) dictionary. The folder is in use."
        );
    }
}
