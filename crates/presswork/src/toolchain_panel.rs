use std::collections::BTreeMap;
use std::path::PathBuf;

use directories::ProjectDirs;
use futures::SinkExt;
use futures::channel::mpsc;
use iced::alignment::Vertical;
use iced::futures::Stream;
use iced::widget::{button, column, container, progress_bar, row, rule, scrollable, space, text};
use iced::{Element, Fill, FillPortion, Task};
use presswork_toolchain::{
    Client, Engine, Platform, Progress, Release, Toolchains, Version, VersionReq,
};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::dictionary_list::{self, DictionaryList};
use crate::i18n::{FluentArgs, Localizer};

const DICTIONARIES: &str = "dictionaries";
const VERSION_WIDTH: f32 = 96.0;
const SIZE_WIDTH: f32 = 64.0;
const ROW_HEIGHT: f32 = 28.0;
const PROGRESS_LENGTH: f32 = 80.0;
const PROGRESS_GIRTH: f32 = 4.0;
const MEGABYTE: u64 = 1_000_000;

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 26] = [
    "status-typst",
    "status-typst-missing",
    "status-typst-invalid",
    "toolchains-title",
    "toolchains-installed",
    "toolchains-available",
    "toolchains-built-in",
    "toolchains-in-use",
    "toolchains-use",
    "toolchains-remove",
    "toolchains-install",
    "toolchains-installing",
    "toolchains-loading",
    "toolchains-load-error",
    "toolchains-retry",
    "toolchains-all-installed",
    "toolchains-size",
    "toolchains-project-none",
    "toolchains-project-built-in",
    "toolchains-project-managed",
    "toolchains-project-missing",
    "toolchains-project-invalid",
    "toolchains-install-error",
    "toolchains-remove-error",
    "toolchains-pin-error",
    "toolchains-no-folder",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Choice {
    Engine(Engine),
    InvalidPin(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Releases {
    NotLoaded,
    Loading,
    Loaded(Vec<Release>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    Install(Version, String),
    Remove(Version, String),
    NoFolder,
}

#[derive(Debug)]
pub(crate) struct ToolchainPanel {
    toolchains: Option<Toolchains>,
    client: Option<Client>,
    embedded: Version,
    open: bool,
    installed: Vec<Version>,
    releases: Releases,
    installing: BTreeMap<Version, u8>,
    problem: Option<Problem>,
    dictionaries: DictionaryList,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Toggle,
    Close,
    Reload,
    Loaded(Result<Vec<Release>, String>),
    Install(Version),
    Progress(Version, u8),
    Installed(Version, Result<(), String>),
    Remove(Version),
    Use(Version),
    Dictionaries(dictionary_list::Message),
}

impl ToolchainPanel {
    pub(crate) fn new(root: Option<PathBuf>, embedded: Version) -> Self {
        let dictionaries = root
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|data| data.join(DICTIONARIES));
        let mut panel = Self {
            toolchains: root.map(Toolchains::new),
            client: None,
            embedded,
            open: false,
            installed: Vec::new(),
            releases: Releases::NotLoaded,
            installing: BTreeMap::new(),
            problem: None,
            dictionaries: DictionaryList::new(dictionaries),
        };
        panel.refresh();
        panel
    }

    pub(crate) fn dictionary_root(&self) -> Option<PathBuf> {
        self.dictionaries.root()
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    pub(crate) fn embedded(&self) -> &Version {
        &self.embedded
    }

    pub(crate) fn installed(&self) -> &[Version] {
        &self.installed
    }

    pub(crate) fn executable(&self, version: &Version) -> Option<PathBuf> {
        self.toolchains
            .as_ref()
            .map(|toolchains| toolchains.executable(version))
    }

    pub(crate) fn available(&self, pin: Option<&VersionReq>) -> Vec<&Release> {
        let Releases::Loaded(releases) = &self.releases else {
            return Vec::new();
        };
        let platform = Platform::current();
        let stable_agrees = pin.is_some_and(|pin| {
            releases
                .iter()
                .any(|release| release.is_stable() && pin.matches(&release.version))
        });
        releases
            .iter()
            .filter(|release| {
                release
                    .file(&platform)
                    .is_some_and(|file| file.digest.is_some())
            })
            .filter(|release| {
                release.version != self.embedded && !self.installed.contains(&release.version)
            })
            .filter(|release| {
                release.is_stable()
                    || pin.is_some_and(|pin| !stable_agrees && pin.matches(&release.version))
            })
            .collect()
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Toggle => {
                self.open = !self.open;
                if self.open && self.releases == Releases::NotLoaded {
                    return self.load();
                }
            }
            Message::Close => self.open = false,
            Message::Reload => return self.load(),
            Message::Loaded(Ok(releases)) => self.releases = Releases::Loaded(releases),
            Message::Loaded(Err(error)) => {
                tracing::warn!(%error, "The list of Typst versions is not available");
                self.releases = Releases::Failed(error);
            }
            Message::Install(version) => return self.install(version),
            Message::Progress(version, percent) => {
                self.installing.insert(version, percent);
            }
            Message::Installed(version, result) => {
                self.installing.remove(&version);
                if let Err(error) = result {
                    self.problem = Some(Problem::Install(version, error));
                }
                self.refresh();
            }
            Message::Remove(version) => self.remove(&version),
            Message::Use(_) => {}
            Message::Dictionaries(message) => {
                let client = self.client();
                return self
                    .dictionaries
                    .update(message, client)
                    .map(Message::Dictionaries);
            }
        }
        Task::none()
    }

    fn load(&mut self) -> Task<Message> {
        let client = match self.client() {
            Ok(client) => client,
            Err(error) => {
                self.releases = Releases::Failed(error);
                return Task::none();
            }
        };
        self.releases = Releases::Loading;
        Task::perform(
            async move { client.releases().await.map_err(|error| error.to_string()) },
            Message::Loaded,
        )
    }

    fn client(&mut self) -> Result<Client, String> {
        if let Some(client) = &self.client {
            return Ok(client.clone());
        }
        let client = Client::new(&user_agent()).map_err(|error| error.to_string())?;
        self.client = Some(client.clone());
        Ok(client)
    }

    fn install(&mut self, version: Version) -> Task<Message> {
        self.problem = None;
        let Some(toolchains) = self.toolchains.clone() else {
            self.problem = Some(Problem::NoFolder);
            return Task::none();
        };
        let Some(release) = self.release(&version).cloned() else {
            return Task::none();
        };
        let client = match self.client() {
            Ok(client) => client,
            Err(error) => {
                self.problem = Some(Problem::Install(version, error));
                return Task::none();
            }
        };
        self.installing.insert(version, 0);
        Task::run(install(toolchains, client, release), |message| message)
    }

    fn remove(&mut self, version: &Version) {
        self.problem = None;
        let Some(toolchains) = &self.toolchains else {
            self.problem = Some(Problem::NoFolder);
            return;
        };
        match toolchains.remove(version) {
            Ok(()) => tracing::info!(%version, "Typst version removed"),
            Err(error) => {
                tracing::warn!(%error, %version, "Typst version not removed");
                self.problem = Some(Problem::Remove(version.clone(), error.to_string()));
            }
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        self.installed = match self.toolchains.as_ref().map(Toolchains::installed) {
            Some(Ok(versions)) => versions,
            Some(Err(error)) => {
                tracing::warn!(%error, "The installed Typst versions are not available");
                Vec::new()
            }
            None => Vec::new(),
        };
    }

    fn release(&self, version: &Version) -> Option<&Release> {
        let Releases::Loaded(releases) = &self.releases else {
            return None;
        };
        releases.iter().find(|release| release.version == *version)
    }

    pub(crate) fn view(
        &self,
        localizer: &Localizer,
        choice: Option<&Choice>,
    ) -> Element<'_, Message> {
        let close = button(icon(Icon::X))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press(Message::Close);
        let header = row![
            text(localizer.text("toolchains-title")).font(font::UI_SEMIBOLD),
            text(self.project_text(localizer, choice))
                .size(font_size::SMALL)
                .style(style::muted),
            space().width(Fill),
            close
        ]
        .spacing(gap::M)
        .align_y(Vertical::Center);
        let pin = match choice {
            Some(Choice::Engine(Engine::Missing(pin))) => Some(pin),
            _ => None,
        };
        let lists = row![
            container(self.installed_list(localizer, choice)).width(FillPortion(1)),
            rule::vertical(1).style(style::hairline),
            container(self.available_list(localizer, pin)).width(FillPortion(1)),
            rule::vertical(1).style(style::hairline),
            container(self.dictionaries.view(localizer).map(Message::Dictionaries))
                .width(FillPortion(1)),
        ]
        .spacing(gap::L);
        let mut content = column![header].spacing(gap::S);
        if let Some(problem) = &self.problem {
            content = content.push(
                text(problem_text(localizer, problem))
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Danger)),
            );
        }
        container(content.push(lists))
            .padding([gap::S, gap::L])
            .width(Fill)
            .height(size::PANEL)
            .style(style::chrome)
            .into()
    }

    fn project_text(&self, localizer: &Localizer, choice: Option<&Choice>) -> String {
        let mut args = FluentArgs::new();
        let id = match choice {
            None => "toolchains-project-none",
            Some(Choice::Engine(Engine::Embedded)) => {
                args.set("version", self.embedded.to_string());
                "toolchains-project-built-in"
            }
            Some(Choice::Engine(Engine::Managed(version))) => {
                args.set("version", version.to_string());
                args.set("embedded", self.embedded.to_string());
                "toolchains-project-managed"
            }
            Some(Choice::Engine(Engine::Missing(pin))) => {
                args.set("pin", pin.to_string());
                "toolchains-project-missing"
            }
            Some(Choice::InvalidPin(pin)) => {
                args.set("pin", pin.clone());
                "toolchains-project-invalid"
            }
        };
        localizer.format(id, &args)
    }

    fn installed_list(
        &self,
        localizer: &Localizer,
        choice: Option<&Choice>,
    ) -> Element<'_, Message> {
        let in_use = |version: &Version| match choice {
            Some(Choice::Engine(Engine::Embedded)) => *version == self.embedded,
            Some(Choice::Engine(Engine::Managed(managed))) => version == managed,
            _ => false,
        };
        let project_open = choice.is_some();
        let mut rows = vec![version_row(
            localizer,
            &self.embedded,
            true,
            in_use(&self.embedded),
            project_open,
        )];
        rows.extend(
            self.installed.iter().map(|version| {
                version_row(localizer, version, false, in_use(version), project_open)
            }),
        );
        column![
            heading(localizer, "toolchains-installed"),
            scrollable(column(rows).spacing(gap::XS)).height(Fill)
        ]
        .spacing(gap::S)
        .into()
    }

    fn available_list(
        &self,
        localizer: &Localizer,
        pin: Option<&VersionReq>,
    ) -> Element<'_, Message> {
        let body: Element<'_, Message> = match &self.releases {
            Releases::NotLoaded | Releases::Loading => muted(localizer.text("toolchains-loading")),
            Releases::Failed(error) => {
                let mut args = FluentArgs::new();
                args.set("error", sentence(error));
                column![
                    text(localizer.format("toolchains-load-error", &args))
                        .size(font_size::SMALL)
                        .style(style::tone(Tone::Danger)),
                    action(
                        Icon::RefreshCw,
                        localizer.text("toolchains-retry"),
                        Message::Reload
                    ),
                ]
                .spacing(gap::S)
                .into()
            }
            Releases::Loaded(_) => {
                let releases = self.available(pin);
                if releases.is_empty() {
                    muted(localizer.text("toolchains-all-installed"))
                } else {
                    let rows = releases
                        .into_iter()
                        .map(|release| self.release_row(localizer, release));
                    scrollable(column(rows).spacing(gap::XS))
                        .height(Fill)
                        .into()
                }
            }
        };
        column![heading(localizer, "toolchains-available"), body]
            .spacing(gap::S)
            .into()
    }

    fn release_row(&self, localizer: &Localizer, release: &Release) -> Element<'_, Message> {
        let mut args = FluentArgs::new();
        let size = release
            .file(&Platform::current())
            .map_or(0, |file| (file.size + MEGABYTE / 2) / MEGABYTE);
        args.set("megabytes", size);
        let state: Element<'_, Message> = match self.installing.get(&release.version) {
            Some(percent) => {
                let mut args = FluentArgs::new();
                args.set("percent", *percent);
                row![
                    progress_bar(0.0..=100.0, f32::from(*percent))
                        .length(PROGRESS_LENGTH)
                        .girth(PROGRESS_GIRTH),
                    muted(localizer.format("toolchains-installing", &args)),
                ]
                .spacing(gap::S)
                .align_y(Vertical::Center)
                .into()
            }
            None => action(
                Icon::Download,
                localizer.text("toolchains-install"),
                Message::Install(release.version.clone()),
            ),
        };
        row![
            text(release.version.to_string())
                .font(font::MONO)
                .size(font_size::SMALL)
                .width(VERSION_WIDTH),
            text(localizer.format("toolchains-size", &args))
                .font(font::MONO)
                .size(font_size::SMALL)
                .style(style::muted)
                .width(SIZE_WIDTH),
            space().width(Fill),
            state
        ]
        .spacing(gap::S)
        .height(ROW_HEIGHT)
        .align_y(Vertical::Center)
        .into()
    }
}

fn version_row<'a>(
    localizer: &Localizer,
    version: &Version,
    built_in: bool,
    in_use: bool,
    project_open: bool,
) -> Element<'a, Message> {
    let mut content = row![
        text(version.to_string())
            .font(font::MONO)
            .size(font_size::SMALL)
            .width(VERSION_WIDTH)
    ]
    .spacing(gap::S)
    .height(ROW_HEIGHT)
    .align_y(Vertical::Center);
    if built_in {
        content = content.push(badge(localizer.text("toolchains-built-in"), Tone::Neutral));
    }
    content = content.push(space().width(Fill));
    if in_use {
        content = content.push(badge(localizer.text("toolchains-in-use"), Tone::Success));
    } else if project_open {
        content = content.push(action(
            Icon::Pin,
            localizer.text("toolchains-use"),
            Message::Use(version.clone()),
        ));
    }
    if !built_in {
        content = content.push(action(
            Icon::Trash2,
            localizer.text("toolchains-remove"),
            Message::Remove(version.clone()),
        ));
    }
    content.into()
}

fn heading<'a>(localizer: &Localizer, id: &str) -> Element<'a, Message> {
    text(localizer.text(id))
        .size(font_size::CAPTION)
        .font(font::UI_MEDIUM)
        .style(style::muted)
        .into()
}

fn muted<'a>(content: String) -> Element<'a, Message> {
    text(content)
        .size(font_size::SMALL)
        .style(style::muted)
        .into()
}

fn badge<'a>(content: String, tone: Tone) -> Element<'a, Message> {
    container(text(content).size(font_size::CAPTION))
        .padding([0.0, gap::XS])
        .style(style::badge(tone))
        .into()
}

fn action<'a>(symbol: Icon, label: String, message: Message) -> Element<'a, Message> {
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
    .on_press(message)
    .into()
}

fn problem_text(localizer: &Localizer, problem: &Problem) -> String {
    let mut args = FluentArgs::new();
    let id = match problem {
        Problem::Install(version, error) => {
            args.set("version", version.to_string());
            args.set("error", sentence(error));
            "toolchains-install-error"
        }
        Problem::Remove(version, error) => {
            args.set("version", version.to_string());
            args.set("error", sentence(error));
            "toolchains-remove-error"
        }
        Problem::NoFolder => "toolchains-no-folder",
    };
    localizer.format(id, &args)
}

pub(crate) fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    let mut sentence: String = first.to_uppercase().chain(characters).collect();
    if !sentence.ends_with('.') {
        sentence.push('.');
    }
    sentence
}

pub(crate) fn percent(progress: Progress) -> u8 {
    let Some(total) = progress.total.filter(|total| *total > 0) else {
        return 0;
    };
    u8::try_from(progress.received.min(total) * 100 / total).unwrap_or(100)
}

fn install(
    toolchains: Toolchains,
    client: Client,
    release: Release,
) -> impl Stream<Item = Message> {
    iced::stream::channel(16, async move |mut output: mpsc::Sender<Message>| {
        let version = release.version.clone();
        tracing::info!(%version, "Typst installation started");
        let mut progress_output = output.clone();
        let progress_version = version.clone();
        let mut last = None;
        let result = toolchains
            .install(&client, &release, move |progress| {
                let percent = percent(progress);
                if last != Some(percent) {
                    last = Some(percent);
                    let _ = progress_output
                        .try_send(Message::Progress(progress_version.clone(), percent));
                }
            })
            .await;
        match &result {
            Ok(()) => tracing::info!(%version, "Typst installed"),
            Err(error) => tracing::warn!(%error, %version, "Typst not installed"),
        }
        let _ = output
            .send(Message::Installed(
                version,
                result.map_err(|error| error.to_string()),
            ))
            .await;
    })
}

pub(crate) fn user_agent() -> String {
    format!(
        "Presswork/{} (https://github.com/getpresswork/presswork)",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn toolchain_directory() -> Option<PathBuf> {
    ProjectDirs::from("io.github", "getpresswork", "Presswork")
        .map(|directories| directories.data_local_dir().join("toolchains"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use iced_test::simulator;
    use presswork_toolchain::{Digest, Platform, ReleaseFile};

    use super::*;
    use crate::testing::{Route, collect, serve};

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn release(text: &str) -> Release {
        Release {
            version: version(text),
            files: vec![ReleaseFile {
                name: Platform::current().release_file().unwrap(),
                url: format!("https://example.com/{text}"),
                size: 22_000_000,
                digest: Digest::from_hex(&"a".repeat(64)),
            }],
        }
    }

    fn panel(root: &Path) -> ToolchainPanel {
        ToolchainPanel::new(Some(root.to_path_buf()), version("0.15.1"))
    }

    fn install_folder(root: &Path, text: &str) {
        let folder = root.join("typst").join(text);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(Platform::current().executable()), "binary").unwrap();
    }

    fn load(panel: &mut ToolchainPanel, releases: Vec<Release>) {
        let _ = panel.update(Message::Loaded(Ok(releases)));
    }

    fn available(panel: &ToolchainPanel, pin: Option<&str>) -> Vec<String> {
        let pin = pin.map(|pin| VersionReq::parse(pin).unwrap());
        panel
            .available(pin.as_ref())
            .iter()
            .map(|release| release.version.to_string())
            .collect()
    }

    #[test]
    fn starts_closed_with_the_installed_versions() {
        let root = tempfile::tempdir().unwrap();
        install_folder(root.path(), "0.14.2");

        let panel = panel(root.path());

        assert!(!panel.is_open());
        assert_eq!(panel.installed(), [version("0.14.2")]);
    }

    #[test]
    fn toggle_opens_the_panel_and_starts_to_load_the_list() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());

        let _ = panel.update(Message::Toggle);

        assert!(panel.is_open());
        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(ui.find("Presswork gets the list of versions.").is_ok());
    }

    #[test]
    fn available_versions_are_stable_and_not_installed_or_built_in() {
        let root = tempfile::tempdir().unwrap();
        install_folder(root.path(), "0.14.2");
        let mut panel = panel(root.path());

        load(
            &mut panel,
            ["0.15.1", "0.15.0", "0.15.0-rc.1", "0.14.2", "0.14.1"]
                .map(release)
                .to_vec(),
        );

        assert_eq!(available(&panel, None), ["0.15.0", "0.14.1"]);
    }

    #[test]
    fn a_pin_that_agrees_only_with_a_candidate_shows_the_candidate() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());

        load(&mut panel, ["0.16.0-rc.1", "0.15.0"].map(release).to_vec());

        assert_eq!(
            available(&panel, Some("=0.16.0-rc.1")),
            ["0.16.0-rc.1", "0.15.0"]
        );
        assert_eq!(available(&panel, Some("0.15")), ["0.15.0"]);
    }

    #[test]
    fn a_release_without_a_digest_or_a_file_is_not_available() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        let mut no_digest = release("0.13.1");
        no_digest.files[0].digest = None;
        let mut no_file = release("0.13.0");
        no_file.files.clear();

        load(&mut panel, vec![no_digest, no_file, release("0.12.0")]);

        assert_eq!(available(&panel, None), ["0.12.0"]);
    }

    #[test]
    fn a_failed_list_shows_the_problem_and_a_new_attempt() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());

        let _ = panel.update(Message::Loaded(Err(
            "the network request failed: timeout".to_owned()
        )));

        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(
            ui.find(
                "Presswork cannot get the list of versions. The network request failed: timeout."
            )
            .is_ok()
        );
        assert!(ui.find("Try again").is_ok());
    }

    #[test]
    fn the_size_of_a_release_is_rounded_to_megabytes() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        let mut small = release("0.15.0");
        small.files[0].size = 22_310_112;
        let mut large = release("0.14.0");
        large.files[0].size = 22_500_000;

        load(&mut panel, vec![small, large]);

        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(ui.find("22 MB").is_ok());
        assert!(ui.find("23 MB").is_ok());
    }

    #[test]
    fn progress_shows_the_percentage_of_an_installation() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        load(&mut panel, vec![release("0.15.0")]);

        let _ = panel.update(Message::Progress(version("0.15.0"), 45));

        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(ui.find("Installing 45%").is_ok());
        assert!(ui.find("Install").is_err());
    }

    #[test]
    fn a_finished_installation_moves_the_version_to_the_installed_list() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        load(&mut panel, vec![release("0.15.0")]);
        let _ = panel.update(Message::Progress(version("0.15.0"), 100));
        install_folder(root.path(), "0.15.0");

        let _ = panel.update(Message::Installed(version("0.15.0"), Ok(())));

        assert_eq!(panel.installed(), [version("0.15.0")]);
        assert_eq!(available(&panel, None), Vec::<String>::new());
        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(ui.find("Installing 100%").is_err());
    }

    #[test]
    fn a_failed_installation_shows_the_problem() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());

        let _ = panel.update(Message::Installed(
            version("0.15.0"),
            Err("the archive is damaged: bad header".to_owned()),
        ));

        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(
            ui.find("Typst 0.15.0 is not installed. The archive is damaged: bad header.")
                .is_ok()
        );
    }

    #[test]
    fn remove_deletes_an_installed_version() {
        let root = tempfile::tempdir().unwrap();
        install_folder(root.path(), "0.14.2");
        let mut panel = panel(root.path());

        let _ = panel.update(Message::Remove(version("0.14.2")));

        assert_eq!(panel.installed(), Vec::<Version>::new());
    }

    #[test]
    fn the_view_marks_the_version_in_use() {
        let root = tempfile::tempdir().unwrap();
        install_folder(root.path(), "0.14.2");
        let panel = panel(root.path());
        let choice = Choice::Engine(Engine::Managed(version("0.14.2")));

        let mut ui = simulator(panel.view(&Localizer::english(), Some(&choice)));

        assert!(ui.find("In use").is_ok());
        assert!(ui.find("Use for this project").is_ok());
        assert!(
            ui.find("This project uses Typst 0.14.2 for the preview and the exports. Completion and other editor help need the built-in Typst 0.15.1.")
                .is_ok()
        );
    }

    #[test]
    fn a_click_on_use_asks_to_pin_the_version() {
        let root = tempfile::tempdir().unwrap();
        let panel = panel(root.path());
        let choice = Choice::Engine(Engine::Missing(VersionReq::parse("=0.13.1").unwrap()));

        let mut ui = simulator(panel.view(&Localizer::english(), Some(&choice)));
        ui.click("Use for this project").unwrap();

        assert!(matches!(
            ui.into_messages().collect::<Vec<_>>().as_slice(),
            [Message::Use(used)] if *used == version("0.15.1")
        ));
    }

    #[test]
    fn without_a_project_no_version_can_be_used() {
        let root = tempfile::tempdir().unwrap();
        let panel = panel(root.path());

        let mut ui = simulator(panel.view(&Localizer::english(), None));

        assert!(ui.find("Use for this project").is_err());
        assert!(
            ui.find("Open a project to select its Typst version.")
                .is_ok()
        );
    }

    #[test]
    fn each_text_of_the_panel_has_english_text() {
        let localizer = Localizer::english();

        for id in TEXT_IDS {
            assert_ne!(localizer.text(id), id);
        }
    }

    #[test]
    fn the_dictionaries_are_next_to_the_typst_versions() {
        let root = tempfile::tempdir().unwrap();
        let toolchains = root.path().join("toolchains");

        let panel = panel(&toolchains);

        assert_eq!(
            panel.dictionary_root(),
            Some(root.path().join(DICTIONARIES))
        );
    }

    #[test]
    fn close_closes_the_panel_and_use_changes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        let _ = panel.update(Message::Toggle);

        let _ = panel.update(Message::Use(version("0.15.1")));
        assert!(panel.is_open());
        let _ = panel.update(Message::Close);

        assert!(!panel.is_open());
    }

    #[test]
    fn reload_gets_the_list_again() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        let _ = panel.update(Message::Loaded(Err("timeout".to_owned())));

        let _ = panel.update(Message::Reload);

        assert_eq!(panel.releases, Releases::Loading);
        assert_eq!(panel.available(None).len(), 0);
    }

    #[test]
    fn without_a_folder_no_version_installs_or_goes_away() {
        let mut panel = ToolchainPanel::new(None, version("0.15.1"));
        load(&mut panel, vec![release("0.15.0")]);

        let _ = panel.update(Message::Install(version("0.15.0")));
        assert_eq!(panel.problem, Some(Problem::NoFolder));
        panel.problem = None;
        let _ = panel.update(Message::Remove(version("0.15.0")));

        assert_eq!(panel.problem, Some(Problem::NoFolder));
        assert!(panel.installing.is_empty());
        assert_eq!(panel.executable(&version("0.15.0")), None);
        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(
            ui.find("Presswork cannot find a folder for the Typst versions.")
                .is_ok()
        );
    }

    #[test]
    fn an_installation_starts_only_for_a_version_of_the_list() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());
        load(&mut panel, vec![release("0.15.0")]);

        let _ = panel.update(Message::Install(version("0.14.0")));
        assert!(panel.installing.is_empty());
        let _ = panel.update(Message::Install(version("0.15.0")));

        assert_eq!(panel.installing.get(&version("0.15.0")), Some(&0));
        assert!(panel.client.is_some());
    }

    #[test]
    fn a_failed_removal_shows_the_problem() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(root.path());

        let _ = panel.update(Message::Remove(version("0.14.2")));

        assert!(matches!(
            &panel.problem,
            Some(Problem::Remove(removed, _)) if *removed == version("0.14.2")
        ));
        let text = problem_text(
            &Localizer::english(),
            &Problem::Remove(version("0.14.2"), "the folder is in use".to_owned()),
        );
        assert_eq!(text, "Typst 0.14.2 is not removed. The folder is in use.");
    }

    #[test]
    fn dictionary_messages_go_to_the_dictionary_list() {
        let root = tempfile::tempdir().unwrap();
        let mut panel = panel(&root.path().join("toolchains"));

        let _ = panel.update(Message::Dictionaries(dictionary_list::Message::Installed(
            "de-DE".to_owned(),
            Err("the server answered with the HTTP status 404".to_owned()),
        )));

        let mut ui = simulator(panel.view(&Localizer::english(), None));
        assert!(
            ui.find("Presswork cannot download the German (Germany) dictionary. The server answered with the HTTP status 404.")
                .is_ok()
        );
        assert!(panel.client.is_some());
    }

    #[test]
    fn the_panel_tells_about_the_built_in_version_and_a_bad_pin() {
        let root = tempfile::tempdir().unwrap();
        let panel = panel(root.path());
        let localizer = Localizer::english();

        let built_in = panel.project_text(&localizer, Some(&Choice::Engine(Engine::Embedded)));
        let invalid = panel.project_text(&localizer, Some(&Choice::InvalidPin("x".to_owned())));

        assert_eq!(built_in, "This project uses the built-in Typst 0.15.1.");
        assert_eq!(
            invalid,
            "The Typst pin \"x\" in presswork.toml is not valid."
        );
        let mut ui = simulator(panel.view(&localizer, Some(&Choice::Engine(Engine::Embedded))));
        assert!(ui.find("In use").is_ok());
    }

    #[test]
    fn a_sentence_starts_with_a_capital_and_ends_with_a_period() {
        assert_eq!(sentence("the file is missing"), "The file is missing.");
        assert_eq!(sentence("Done."), "Done.");
        assert_eq!(sentence(""), "");
    }

    #[test]
    fn the_percent_comes_from_the_received_bytes() {
        let progress = |received, total| Progress { received, total };

        assert_eq!(percent(progress(50, Some(200))), 25);
        assert_eq!(percent(progress(300, Some(200))), 100);
        assert_eq!(percent(progress(50, Some(0))), 0);
        assert_eq!(percent(progress(50, None)), 0);
    }

    #[test]
    fn an_archive_with_a_different_digest_is_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let base = serve(vec![Route::new("GET /typst", vec![7; 2000])]);
        let mut wrong = release("0.15.0");
        wrong.files[0].url = format!("{base}/typst");
        wrong.files[0].size = 2000;

        let messages = collect(install(
            Toolchains::new(root.path()),
            Client::new("presswork-test").unwrap(),
            wrong,
        ));

        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Progress(installing, 100) if *installing == version("0.15.0")
        )));
        assert!(matches!(
            messages.last(),
            Some(Message::Installed(installed, Err(_))) if *installed == version("0.15.0")
        ));
        assert_eq!(
            Toolchains::new(root.path()).installed().unwrap(),
            Vec::<Version>::new()
        );
    }
}
