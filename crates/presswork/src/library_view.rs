use std::sync::Arc;

use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Fill, Task};
use presswork_api::{FileState, NewWork, SearchHit, Work};
use presswork_library::{parse_key, propose_key};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, space as gap};
use uuid::Uuid;

use crate::i18n::{FluentArgs, Localizer};
use crate::server_dialog::{Connection, Problem, problem, problem_text};

pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const LIST_WIDTH: f32 = 720.0;
const YEAR_WIDTH: f32 = 96.0;

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 23] = [
    "library-form-creators-example",
    "library-page-count",
    "library-reading",
    "library-failed",
    "library-title",
    "library-add",
    "library-search-placeholder",
    "library-search",
    "library-loading",
    "library-empty",
    "library-no-hits",
    "library-hit",
    "library-cite",
    "library-pages",
    "library-no-file",
    "library-form-title",
    "library-form-creators",
    "library-form-year",
    "library-form-key",
    "library-form-add",
    "library-form-cancel",
    "library-uploading",
    "library-too-large",
];

#[derive(Debug, Clone)]
pub(crate) struct PickedFile {
    name: String,
    bytes: Arc<Vec<u8>>,
}

impl PickedFile {
    #[cfg(test)]
    pub(crate) fn new(name: &str, bytes: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            bytes: Arc::new(bytes),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Loaded(Result<Vec<Work>, Problem>),
    Query(String),
    Search,
    Found(Result<Vec<SearchHit>, Problem>),
    Pick,
    Picked(Option<PickedFile>),
    Title(String),
    Creators(String),
    Year(String),
    Key(String),
    Upload,
    Created(Result<Work, Problem>),
    Uploaded(Result<Work, Problem>),
    Cancel,
    Cite(SearchHit),
}

#[derive(Debug, Clone)]
enum Trouble {
    Server(Problem),
    TooLarge,
}

#[derive(Debug)]
struct Draft {
    file: PickedFile,
    title: String,
    creators: String,
    year: String,
    key: String,
    key_changed: bool,
    work: Option<Uuid>,
    sending: bool,
}

impl Draft {
    fn new(file: PickedFile) -> Self {
        let title = title_of(&file.name);
        Self {
            key: propose_key(&title),
            file,
            title,
            creators: String::new(),
            year: String::new(),
            key_changed: false,
            work: None,
            sending: false,
        }
    }

    fn new_work(&self) -> Option<NewWork> {
        let title = self.title.trim();
        if title.is_empty() {
            return None;
        }
        let key = parse_key(&self.key).ok()?.to_owned();
        let year = match self.year.trim() {
            "" => None,
            year => Some(year.parse().ok()?),
        };
        let creators = self
            .creators
            .split(';')
            .map(str::trim)
            .filter(|creator| !creator.is_empty())
            .map(str::to_owned)
            .collect();
        Some(NewWork {
            key,
            title: title.to_owned(),
            creators,
            year,
        })
    }
}

fn title_of(file_name: &str) -> String {
    let stem = file_name
        .strip_suffix(".pdf")
        .or_else(|| file_name.strip_suffix(".PDF"))
        .unwrap_or(file_name);
    stem.replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug)]
pub(crate) struct LibraryView {
    organization: Uuid,
    name: String,
    works: Option<Vec<Work>>,
    query: String,
    hits: Option<Vec<SearchHit>>,
    draft: Option<Draft>,
    trouble: Option<Trouble>,
}

impl LibraryView {
    pub(crate) fn new(organization: Uuid, name: String) -> Self {
        Self {
            organization,
            name,
            works: None,
            query: String::new(),
            hits: None,
            draft: None,
            trouble: None,
        }
    }

    pub(crate) fn works(&self) -> Option<&[Work]> {
        self.works.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn proposal(&self) -> Option<NewWork> {
        self.draft.as_ref()?.new_work()
    }

    pub(crate) fn load(&self, connection: &Connection) -> Task<Message> {
        let Connection { api, token } = connection.clone();
        let organization = self.organization;
        Task::perform(
            async move { api.works(&token, organization).await.map_err(problem) },
            Message::Loaded,
        )
    }

    pub(crate) fn update(&mut self, message: Message, connection: &Connection) -> Task<Message> {
        match message {
            Message::Loaded(Ok(works)) => self.works = Some(works),
            Message::Loaded(Err(problem)) | Message::Found(Err(problem)) => {
                self.trouble = Some(Trouble::Server(problem));
            }
            Message::Query(query) => self.query = query,
            Message::Search => return self.search(connection),
            Message::Found(Ok(hits)) => self.hits = Some(hits),
            Message::Pick => return pick(),
            Message::Picked(Some(file)) => self.take(file),
            Message::Picked(None) | Message::Cite(_) => {}
            Message::Title(title) => {
                if let Some(draft) = &mut self.draft {
                    if !draft.key_changed {
                        draft.key = propose_key(&title);
                    }
                    draft.title = title;
                }
            }
            Message::Creators(creators) => {
                if let Some(draft) = &mut self.draft {
                    draft.creators = creators;
                }
            }
            Message::Year(year) => {
                if let Some(draft) = &mut self.draft {
                    draft.year = year;
                }
            }
            Message::Key(key) => {
                if let Some(draft) = &mut self.draft {
                    draft.key = key;
                    draft.key_changed = true;
                }
            }
            Message::Upload => return self.upload(connection),
            Message::Created(Ok(work)) => {
                if let Some(draft) = &mut self.draft {
                    draft.work = Some(work.id);
                }
                self.keep(work);
                return self.send_file(connection);
            }
            Message::Created(Err(problem)) | Message::Uploaded(Err(problem)) => {
                if let Some(draft) = &mut self.draft {
                    draft.sending = false;
                }
                self.trouble = Some(Trouble::Server(problem));
            }
            Message::Uploaded(Ok(work)) => {
                self.draft = None;
                self.keep(work);
            }
            Message::Cancel => {
                self.draft = None;
                self.trouble = None;
            }
        }
        Task::none()
    }

    fn take(&mut self, file: PickedFile) {
        if file.bytes.len() > MAX_FILE_BYTES {
            self.draft = None;
            self.trouble = Some(Trouble::TooLarge);
        } else {
            self.draft = Some(Draft::new(file));
            self.trouble = None;
        }
    }

    pub(crate) fn changed(&mut self, work: Work) -> bool {
        if work.organization_id != self.organization {
            return false;
        }
        self.keep(work);
        true
    }

    fn keep(&mut self, work: Work) {
        let works = self.works.get_or_insert_with(Vec::new);
        match works.iter_mut().find(|known| known.id == work.id) {
            Some(known) => *known = work,
            None => works.push(work),
        }
        works.sort_by(|left, right| {
            left.title
                .to_lowercase()
                .cmp(&right.title.to_lowercase())
                .then_with(|| left.key.cmp(&right.key))
        });
    }

    fn search(&mut self, connection: &Connection) -> Task<Message> {
        let words = self.query.trim().to_owned();
        if words.is_empty() {
            self.hits = None;
            return Task::none();
        }
        self.trouble = None;
        let Connection { api, token } = connection.clone();
        let organization = self.organization;
        Task::perform(
            async move {
                api.search(&token, organization, &words)
                    .await
                    .map_err(problem)
            },
            Message::Found,
        )
    }

    fn upload(&mut self, connection: &Connection) -> Task<Message> {
        let Some(draft) = &mut self.draft else {
            return Task::none();
        };
        if draft.sending {
            return Task::none();
        }
        if draft.work.is_some() {
            return self.send_file(connection);
        }
        let Some(new) = draft.new_work() else {
            return Task::none();
        };
        draft.sending = true;
        self.trouble = None;
        let Connection { api, token } = connection.clone();
        let organization = self.organization;
        Task::perform(
            async move {
                api.create_work(&token, organization, &new)
                    .await
                    .map_err(problem)
            },
            Message::Created,
        )
    }

    fn send_file(&mut self, connection: &Connection) -> Task<Message> {
        let Some(draft) = &mut self.draft else {
            return Task::none();
        };
        let Some(work) = draft.work else {
            return Task::none();
        };
        draft.sending = true;
        self.trouble = None;
        let Connection { api, token } = connection.clone();
        let bytes = Arc::clone(&draft.file.bytes);
        Task::perform(
            async move {
                api.upload_file(&token, work, bytes.as_ref().clone())
                    .await
                    .map_err(problem)
            },
            Message::Uploaded,
        )
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        let mut args = FluentArgs::new();
        args.set("organization", self.name.clone());
        let header = row![
            text(localizer.format("library-title", &args))
                .font(font::UI_SEMIBOLD)
                .size(font_size::TITLE),
            space::horizontal(),
            button(text(localizer.text("library-add")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::secondary)
                .on_press_maybe(self.draft.is_none().then_some(Message::Pick)),
        ]
        .align_y(Vertical::Center);
        let ready = !self.query.trim().is_empty();
        let search = row![
            text_input(&localizer.text("library-search-placeholder"), &self.query)
                .on_input(Message::Query)
                .on_submit(Message::Search)
                .padding([gap::XS + gap::XXS, gap::S])
                .style(style::field),
            button(text(localizer.text("library-search")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe(ready.then_some(Message::Search)),
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center);
        let mut content = column![header, search].spacing(gap::M);
        if let Some(trouble) = &self.trouble {
            let words = match trouble {
                Trouble::Server(problem) => problem_text(problem, localizer),
                Trouble::TooLarge => localizer.text("library-too-large"),
            };
            content = content.push(
                text(words)
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Danger)),
            );
        }
        if let Some(draft) = &self.draft {
            content = content.push(draft_form(draft, localizer));
        }
        let mut lists = column![].spacing(gap::L);
        if let Some(hits) = &self.hits {
            lists = lists.push(hit_list(hits, localizer));
        }
        lists = lists.push(self.work_list(localizer));
        container(
            content
                .push(scrollable(lists).height(Fill))
                .width(LIST_WIDTH),
        )
        .center_x(Fill)
        .padding(gap::XL)
        .height(Fill)
        .style(style::page)
        .into()
    }

    fn work_list<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        match &self.works {
            None => text(localizer.text("library-loading"))
                .style(style::muted)
                .into(),
            Some(works) if works.is_empty() => text(localizer.text("library-empty"))
                .style(style::muted)
                .into(),
            Some(works) => column(works.iter().map(|work| work_line(work, localizer)))
                .spacing(gap::S)
                .into(),
        }
    }
}

fn pick() -> Task<Message> {
    Task::perform(
        async {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("PDF", &["pdf"])
                .pick_file()
                .await?;
            let bytes = file.read().await;
            Some(PickedFile {
                name: file.file_name(),
                bytes: Arc::new(bytes),
            })
        },
        Message::Picked,
    )
}

fn draft_form<'a>(draft: &'a Draft, localizer: &Localizer) -> Element<'a, Message> {
    let field =
        |label: &str, placeholder: &str, value: &'a str, on_input: fn(String) -> Message| {
            column![
                text(localizer.text(label))
                    .size(font_size::SMALL)
                    .style(style::muted),
                text_input(placeholder, value)
                    .on_input(on_input)
                    .padding([gap::XS + gap::XXS, gap::S])
                    .style(style::field),
            ]
            .spacing(gap::XXS)
        };
    let example = localizer.text("library-form-creators-example");
    let actions: Element<'a, Message> = if draft.sending {
        text(localizer.text("library-uploading"))
            .style(style::muted)
            .into()
    } else {
        row![
            button(text(localizer.text("library-form-add")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe(draft.new_work().map(|_| Message::Upload)),
            button(text(localizer.text("library-form-cancel")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::secondary)
                .on_press(Message::Cancel),
        ]
        .spacing(gap::S)
        .into()
    };
    container(
        column![
            text(draft.file.name.as_str()).font(font::UI_SEMIBOLD),
            field("library-form-title", "", &draft.title, Message::Title),
            row![
                field(
                    "library-form-creators",
                    &example,
                    &draft.creators,
                    Message::Creators
                ),
                field("library-form-year", "", &draft.year, Message::Year).width(YEAR_WIDTH),
            ]
            .spacing(gap::S),
            field("library-form-key", "", &draft.key, Message::Key),
            actions,
        ]
        .spacing(gap::S),
    )
    .padding(gap::M)
    .style(style::outline)
    .into()
}

fn hit_list<'a>(hits: &'a [SearchHit], localizer: &Localizer) -> Element<'a, Message> {
    if hits.is_empty() {
        return text(localizer.text("library-no-hits"))
            .style(style::muted)
            .into();
    }
    column(hits.iter().map(|hit| {
        let mut args = FluentArgs::new();
        args.set("title", hit.title.clone());
        args.set("page", hit.page);
        row![
            column![
                text(localizer.format("library-hit", &args)).font(font::UI_SEMIBOLD),
                text(hit.snippet.as_str())
                    .size(font_size::SMALL)
                    .style(style::muted),
            ]
            .spacing(gap::XXS)
            .width(Fill),
            button(text(localizer.text("library-cite")))
                .padding([gap::XS, gap::M])
                .style(style::secondary)
                .on_press(Message::Cite(hit.clone())),
        ]
        .spacing(gap::M)
        .align_y(Vertical::Center)
        .into()
    }))
    .spacing(gap::S)
    .into()
}

fn work_line<'a>(work: &'a Work, localizer: &Localizer) -> Element<'a, Message> {
    let mut parts = Vec::new();
    if !work.creators.is_empty() {
        parts.push(work.creators.join("; "));
    }
    if let Some(year) = work.year {
        parts.push(year.to_string());
    }
    parts.push(format!("@{}", work.key));
    let mut state = None;
    match &work.file {
        Some(file) => {
            let mut args = FluentArgs::new();
            args.set("pages", file.pages);
            args.set("text", file.pages_with_text);
            match file.state {
                FileState::Ready => parts.push(localizer.format("library-pages", &args)),
                FileState::Reading => {
                    parts.push(localizer.format("library-page-count", &args));
                    state = Some(
                        text(localizer.text("library-reading"))
                            .size(font_size::SMALL)
                            .style(style::tone(Tone::Info)),
                    );
                }
                FileState::Failed => {
                    parts.push(localizer.format("library-page-count", &args));
                    let problem = file
                        .problem
                        .clone()
                        .unwrap_or_else(|| localizer.text("library-failed"));
                    state = Some(
                        text(problem)
                            .size(font_size::SMALL)
                            .style(style::tone(Tone::Danger)),
                    );
                }
            }
        }
        None => parts.push(localizer.text("library-no-file")),
    }
    let mut line = column![
        text(work.title.as_str()).font(font::UI_SEMIBOLD),
        text(parts.join(" · "))
            .size(font_size::SMALL)
            .style(style::muted),
    ]
    .spacing(gap::XXS);
    if let Some(state) = state {
        line = line.push(state);
    }
    line.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_api::{FileState, LibraryFile};
    use presswork_client::Api;

    use super::*;

    const PRESS: Uuid = Uuid::from_u128(1);
    const NOTES: Uuid = Uuid::from_u128(10);

    fn connection() -> Connection {
        Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
        }
    }

    fn work(id: Uuid, key: &str, title: &str) -> Work {
        Work {
            id,
            organization_id: PRESS,
            key: key.to_owned(),
            title: title.to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            year: Some(1455),
            file: Some(LibraryFile {
                size: 12_470,
                pages: 2,
                pages_with_text: 2,
                state: FileState::Ready,
                problem: None,
            }),
        }
    }

    fn in_state(state: FileState, problem: Option<&str>) -> Work {
        let mut work = work(NOTES, "field-notes", "Field Notes");
        if let Some(file) = &mut work.file {
            file.state = state;
            file.pages_with_text = 0;
            file.problem = problem.map(str::to_owned);
        }
        work
    }

    #[test]
    fn a_file_that_the_server_reads_shows_its_state() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());

        let _ = view.update(
            Message::Loaded(Ok(vec![in_state(FileState::Reading, None)])),
            &connection(),
        );

        assert!(shows(
            &view,
            "Gutenberg, Johannes · 1455 · @field-notes · 2 pages"
        ));
        assert!(shows(&view, "The server reads the text of the pages."));
    }

    #[test]
    fn a_failed_file_shows_the_problem() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let problem = "Presswork cannot read the pages of the file. Upload the file again.";

        let _ = view.update(
            Message::Loaded(Ok(vec![
                in_state(FileState::Failed, Some(problem)),
                Work {
                    id: Uuid::from_u128(11),
                    key: "atlas".to_owned(),
                    title: "Atlas".to_owned(),
                    ..in_state(FileState::Failed, None)
                },
            ])),
            &connection(),
        );

        assert!(shows(&view, problem));
        assert!(shows(&view, "The server cannot read the text of the file."));
    }

    #[test]
    fn a_changed_work_takes_the_place_of_the_old_one() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let _ = view.update(
            Message::Loaded(Ok(vec![in_state(FileState::Reading, None)])),
            &connection(),
        );

        assert!(view.changed(work(NOTES, "field-notes", "Field Notes")));
        assert!(shows(
            &view,
            "Gutenberg, Johannes · 1455 · @field-notes · 2 pages, 2 with text"
        ));
        assert!(!shows(&view, "The server reads the text of the pages."));
        let elsewhere = Work {
            organization_id: Uuid::from_u128(99),
            ..work(Uuid::from_u128(12), "atlas", "Atlas")
        };
        assert!(!view.changed(elsewhere));
        assert_eq!(view.works().unwrap().len(), 1);
    }

    fn loaded() -> LibraryView {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let _ = view.update(
            Message::Loaded(Ok(vec![work(NOTES, "field-notes", "Field Notes")])),
            &connection(),
        );
        view
    }

    fn picked(name: &str, size: usize) -> Message {
        Message::Picked(Some(PickedFile {
            name: name.to_owned(),
            bytes: Arc::new(vec![0; size]),
        }))
    }

    fn shows(view: &LibraryView, words: &str) -> bool {
        let localizer = Localizer::english();
        simulator(view.view(&localizer)).find(words).is_ok()
    }

    fn clicks(view: &LibraryView, label: &str) -> Vec<Message> {
        let localizer = Localizer::english();
        let mut ui = simulator(view.view(&localizer));
        ui.click(label).unwrap();
        ui.into_messages().collect()
    }

    fn hit() -> SearchHit {
        SearchHit {
            work_id: NOTES,
            key: "field-notes".to_owned(),
            title: "Field Notes".to_owned(),
            page: 3,
            snippet: "The fine grain of the paper".to_owned(),
        }
    }

    #[test]
    fn each_text_of_the_view_is_in_the_english_messages() {
        let localizer = Localizer::english();

        for id in TEXT_IDS {
            assert_ne!(localizer.text(id), id, "{id} has no English text");
        }
    }

    #[test]
    fn the_view_shows_the_works_of_the_library() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        assert!(shows(&view, "Presswork gets the works of the library."));
        let _ = view.update(Message::Loaded(Ok(Vec::new())), &connection());
        assert!(shows(
            &view,
            "The library has no works. Add the first PDF file."
        ));

        let view = loaded();

        for words in [
            "Library · Field Press",
            "Field Notes",
            "Gutenberg, Johannes · 1455 · @field-notes · 2 pages, 2 with text",
        ] {
            assert!(shows(&view, words), "{words} is not in the view");
        }
    }

    #[test]
    fn a_picked_file_gives_a_form_with_a_title_and_a_key() {
        let mut view = loaded();
        assert!(matches!(
            clicks(&view, "Add PDF…").as_slice(),
            [Message::Pick]
        ));

        let _ = view.update(picked("field_notes-2019.pdf", 10), &connection());

        assert!(shows(&view, "field_notes-2019.pdf"));
        assert_eq!(
            view.proposal(),
            Some(NewWork {
                key: "field-notes-2019".to_owned(),
                title: "field notes 2019".to_owned(),
                creators: Vec::new(),
                year: None,
            })
        );
    }

    #[test]
    fn the_key_follows_the_title_until_the_key_changes() {
        let mut view = loaded();
        let _ = view.update(picked("scan.pdf", 10), &connection());

        let _ = view.update(Message::Title("War and Peace".to_owned()), &connection());
        assert_eq!(view.proposal().unwrap().key, "war-and-peace");

        let _ = view.update(Message::Key("tolstoy".to_owned()), &connection());
        let _ = view.update(
            Message::Title("War and Peace, Volume 1".to_owned()),
            &connection(),
        );
        assert_eq!(view.proposal().unwrap().key, "tolstoy");
    }

    #[test]
    fn the_form_reads_the_creators_and_the_year() {
        let mut view = loaded();
        let _ = view.update(picked("scan.pdf", 10), &connection());

        let _ = view.update(
            Message::Creators(" Tolstoy, Leo; Maude, Louise ;".to_owned()),
            &connection(),
        );
        let _ = view.update(Message::Year(" 1869 ".to_owned()), &connection());

        let proposal = view.proposal().unwrap();
        assert_eq!(proposal.creators, ["Tolstoy, Leo", "Maude, Louise"]);
        assert_eq!(proposal.year, Some(1869));
    }

    #[test]
    fn add_waits_for_a_valid_form() {
        let mut view = loaded();
        let _ = view.update(picked("scan.pdf", 10), &connection());

        for message in [
            Message::Year("soon".to_owned()),
            Message::Key("War".to_owned()),
            Message::Title("  ".to_owned()),
        ] {
            let mut copy = loaded();
            let _ = copy.update(picked("scan.pdf", 10), &connection());
            let _ = copy.update(message, &connection());
            assert_eq!(copy.proposal(), None);
            assert!(clicks(&copy, "Add").is_empty());
        }
        assert!(matches!(clicks(&view, "Add").as_slice(), [Message::Upload]));
    }

    #[test]
    fn a_file_larger_than_the_limit_is_refused_at_once() {
        let mut view = loaded();

        let _ = view.update(picked("big.pdf", MAX_FILE_BYTES + 1), &connection());

        assert_eq!(view.proposal(), None);
        assert!(shows(
            &view,
            "The file is larger than 64 MiB. Select a smaller file."
        ));
    }

    #[test]
    fn an_added_work_joins_the_list_and_closes_the_form() {
        let mut view = loaded();
        let _ = view.update(picked("atlas.pdf", 10), &connection());

        let _ = view.update(Message::Upload, &connection());
        assert!(shows(
            &view,
            "Presswork uploads the file and reads its pages."
        ));
        let atlas = work(Uuid::from_u128(11), "atlas", "Atlas");
        let _ = view.update(Message::Created(Ok(atlas.clone())), &connection());
        let _ = view.update(Message::Uploaded(Ok(atlas)), &connection());

        assert_eq!(view.proposal(), None);
        assert!(shows(&view, "Atlas"));
        assert_eq!(view.works().unwrap().len(), 2);
    }

    #[test]
    fn a_failed_upload_keeps_the_form_and_shows_the_problem() {
        let mut view = loaded();
        let _ = view.update(picked("atlas.pdf", 10), &connection());
        let _ = view.update(Message::Upload, &connection());
        let atlas = work(Uuid::from_u128(11), "atlas", "Atlas");
        let _ = view.update(Message::Created(Ok(atlas)), &connection());

        let _ = view.update(
            Message::Uploaded(Err(Problem::Refused(
                "The file is not a PDF file that Presswork can read.".to_owned(),
            ))),
            &connection(),
        );

        assert!(shows(
            &view,
            "The file is not a PDF file that Presswork can read."
        ));
        assert!(view.proposal().is_some());
        assert!(matches!(clicks(&view, "Add").as_slice(), [Message::Upload]));
    }

    #[test]
    fn a_search_shows_the_pages_and_cite_gives_the_page() {
        let mut view = loaded();
        let _ = view.update(Message::Query("fine grain".to_owned()), &connection());
        assert!(matches!(
            clicks(&view, "Find").as_slice(),
            [Message::Search]
        ));

        let _ = view.update(Message::Found(Ok(vec![hit()])), &connection());

        assert!(shows(&view, "Field Notes · p. 3"));
        assert!(shows(&view, "The fine grain of the paper"));
        let messages = clicks(&view, "Cite");
        assert!(
            matches!(messages.as_slice(), [Message::Cite(cited)] if *cited == hit()),
            "{messages:?}"
        );
        let _ = view.update(Message::Found(Ok(Vec::new())), &connection());
        assert!(shows(&view, "No page has all these words."));
    }
}
