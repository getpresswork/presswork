use std::sync::Arc;

use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Fill, Task};
use presswork_api::{
    Edition, FileState, LabelStyle, NewEdition, NewWork, PageRange, SearchHit, Work,
};
use presswork_library::{page_labels, parse_key, propose_key};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, space as gap};
use uuid::Uuid;

use crate::i18n::{FluentArgs, Localizer};
use crate::server_dialog::{Connection, Problem, problem, problem_text};

pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const LIST_WIDTH: f32 = 720.0;
const YEAR_WIDTH: f32 = 96.0;
const LANGUAGE_WIDTH: f32 = 120.0;

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 45] = [
    "library-page-numbers",
    "library-page-map-title",
    "library-page-map-empty",
    "library-correction",
    "library-correction-page",
    "library-correction-start",
    "library-style-arabic",
    "library-style-lower-roman",
    "library-style-upper-roman",
    "library-correction-add",
    "library-correction-remove",
    "library-page-map-save",
    "library-title",
    "library-add",
    "library-add-edition",
    "library-search-placeholder",
    "library-search",
    "library-loading",
    "library-empty",
    "library-no-hits",
    "library-hit",
    "library-cite",
    "library-other-edition",
    "library-standard",
    "library-make-standard",
    "library-translators",
    "library-pages",
    "library-page-count",
    "library-reading",
    "library-failed",
    "library-no-file",
    "library-form-title",
    "library-form-creators",
    "library-form-creators-example",
    "library-form-year",
    "library-form-publisher",
    "library-form-language",
    "library-form-translators",
    "library-form-translators-example",
    "library-form-key",
    "library-form-new-edition",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Work,
    Edition(Uuid),
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Loaded(Result<Vec<Work>, Problem>),
    Query(String),
    Search,
    Found(Result<Vec<SearchHit>, Problem>),
    Pick(Target),
    Picked(Target, Option<PickedFile>),
    Title(String),
    Creators(String),
    Key(String),
    Year(String),
    Publisher(String),
    Language(String),
    Translators(String),
    Upload,
    Created(Result<Work, Problem>),
    EditionAdded(Result<Edition, Problem>),
    Uploaded(Result<Work, Problem>),
    MakeStandard(Uuid),
    Changed(Result<Work, Problem>),
    EditPageMap(Uuid),
    CorrectionPage(String),
    CorrectionStart(String),
    CorrectionStyle(LabelStyle),
    AddCorrection,
    RemoveCorrection(usize),
    SavePageMap,
    PageMapSaved(Result<Work, Problem>),
    Cancel,
    Cite(SearchHit),
}

#[derive(Debug)]
struct PageForm {
    edition: Uuid,
    title: String,
    corrections: Vec<PageRange>,
    page: String,
    start: String,
    style: LabelStyle,
    sending: bool,
}

impl PageForm {
    fn correction(&self) -> Option<PageRange> {
        let first_page: u32 = self.page.trim().parse().ok().filter(|page| *page >= 1)?;
        let start: u32 = self.start.trim().parse().ok().filter(|start| *start >= 1)?;
        Some(PageRange {
            first_page,
            style: self.style,
            prefix: String::new(),
            start,
        })
    }
}

fn printed(correction: &PageRange) -> String {
    page_labels(&[], std::slice::from_ref(correction), correction.first_page)
        .pop()
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
enum Trouble {
    Server(Problem),
    TooLarge,
}

#[derive(Debug)]
struct Draft {
    target: Target,
    file: PickedFile,
    title: String,
    creators: String,
    key: String,
    key_changed: bool,
    year: String,
    publisher: String,
    language: String,
    translators: String,
    edition: Option<Uuid>,
    sending: bool,
}

impl Draft {
    fn new(target: Target, file: PickedFile) -> Self {
        let title = title_of(&file.name);
        Self {
            target,
            key: propose_key(&title),
            file,
            title,
            creators: String::new(),
            key_changed: false,
            year: String::new(),
            publisher: String::new(),
            language: String::new(),
            translators: String::new(),
            edition: None,
            sending: false,
        }
    }

    fn new_edition(&self) -> Option<NewEdition> {
        let year = match self.year.trim() {
            "" => None,
            year => Some(year.parse().ok()?),
        };
        Some(NewEdition {
            year,
            publisher: self.publisher.trim().to_owned(),
            language: self.language.trim().to_owned(),
            translators: names(&self.translators),
        })
    }

    fn new_work(&self) -> Option<NewWork> {
        let title = self.title.trim();
        if title.is_empty() {
            return None;
        }
        Some(NewWork {
            key: parse_key(&self.key).ok()?.to_owned(),
            title: title.to_owned(),
            creators: names(&self.creators),
            edition: self.new_edition()?,
        })
    }

    fn ready(&self) -> bool {
        match self.target {
            Target::Work => self.new_work().is_some(),
            Target::Edition(_) => self.new_edition().is_some(),
        }
    }
}

fn names(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
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
    page_form: Option<PageForm>,
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
            page_form: None,
            trouble: None,
        }
    }

    fn free(&self) -> bool {
        self.draft.is_none() && self.page_form.is_none()
    }

    pub(crate) fn works(&self) -> Option<&[Work]> {
        self.works.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn proposal(&self) -> Option<NewWork> {
        self.draft.as_ref()?.new_work()
    }

    #[cfg(test)]
    pub(crate) fn edition_proposal(&self) -> Option<NewEdition> {
        self.draft.as_ref()?.new_edition()
    }

    pub(crate) fn load(&self, connection: &Connection) -> Task<Message> {
        let Connection { api, token, .. } = connection.clone();
        let organization = self.organization;
        Task::perform(
            async move { api.works(&token, organization).await.map_err(problem) },
            Message::Loaded,
        )
    }

    pub(crate) fn changed(&mut self, work: Work) -> bool {
        if work.organization_id != self.organization {
            return false;
        }
        self.keep(work);
        true
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
            Message::Pick(target) => return pick(target),
            Message::Picked(target, Some(file)) => self.take(target, file),
            Message::Picked(_, None) | Message::Cite(_) => {}
            Message::Upload => return self.upload(connection),
            Message::Created(Ok(work)) => {
                if let Some(draft) = &mut self.draft {
                    draft.edition = work.editions.first().map(|edition| edition.id);
                }
                self.keep(work);
                return self.send_file(connection);
            }
            Message::EditionAdded(Ok(edition)) => {
                if let Some(draft) = &mut self.draft {
                    draft.edition = Some(edition.id);
                    if let Target::Edition(work) = draft.target {
                        self.add_edition_to(work, edition);
                    }
                }
                return self.send_file(connection);
            }
            Message::Created(Err(problem))
            | Message::EditionAdded(Err(problem))
            | Message::Uploaded(Err(problem))
            | Message::Changed(Err(problem)) => {
                if let Some(draft) = &mut self.draft {
                    draft.sending = false;
                }
                self.trouble = Some(Trouble::Server(problem));
            }
            Message::Uploaded(Ok(work)) => {
                self.draft = None;
                self.keep(work);
            }
            Message::MakeStandard(edition) => return self.make_standard(edition, connection),
            Message::Changed(Ok(work)) => self.keep(work),
            Message::EditPageMap(edition) => self.open_page_form(edition),
            Message::SavePageMap => return self.save_page_map(connection),
            Message::PageMapSaved(Ok(work)) => {
                self.page_form = None;
                self.keep(work);
            }
            Message::PageMapSaved(Err(problem)) => {
                if let Some(form) = &mut self.page_form {
                    form.sending = false;
                }
                self.trouble = Some(Trouble::Server(problem));
            }
            Message::Cancel => {
                self.draft = None;
                self.page_form = None;
                self.trouble = None;
            }
            Message::CorrectionPage(_)
            | Message::CorrectionStart(_)
            | Message::CorrectionStyle(_)
            | Message::AddCorrection
            | Message::RemoveCorrection(_) => self.edit_page_form(message),
            field => self.edit_draft(field),
        }
        Task::none()
    }

    fn open_page_form(&mut self, edition: Uuid) {
        let found = self.works.as_ref().and_then(|works| {
            works.iter().find_map(|work| {
                work.editions
                    .iter()
                    .find(|known| known.id == edition)
                    .map(|known| (work.title.clone(), known.corrections.clone()))
            })
        });
        let Some((title, corrections)) = found else {
            return;
        };
        self.draft = None;
        self.trouble = None;
        self.page_form = Some(PageForm {
            edition,
            title,
            corrections,
            page: String::new(),
            start: "1".to_owned(),
            style: LabelStyle::Arabic,
            sending: false,
        });
    }

    fn edit_page_form(&mut self, message: Message) {
        let Some(form) = &mut self.page_form else {
            return;
        };
        match message {
            Message::CorrectionPage(page) => form.page = page,
            Message::CorrectionStart(start) => form.start = start,
            Message::CorrectionStyle(style) => form.style = style,
            Message::AddCorrection => {
                if let Some(correction) = form.correction() {
                    form.corrections
                        .retain(|known| known.first_page != correction.first_page);
                    form.corrections.push(correction);
                    form.corrections.sort_by_key(|known| known.first_page);
                    form.page.clear();
                }
            }
            Message::RemoveCorrection(index) if index < form.corrections.len() => {
                form.corrections.remove(index);
            }
            _ => {}
        }
    }

    fn save_page_map(&mut self, connection: &Connection) -> Task<Message> {
        let Some(form) = &mut self.page_form else {
            return Task::none();
        };
        form.sending = true;
        self.trouble = None;
        let Connection { api, token, .. } = connection.clone();
        let edition = form.edition;
        let corrections = form.corrections.clone();
        Task::perform(
            async move {
                api.set_page_map(&token, edition, corrections)
                    .await
                    .map_err(problem)
            },
            Message::PageMapSaved,
        )
    }

    fn edit_draft(&mut self, message: Message) {
        let Some(draft) = &mut self.draft else {
            return;
        };
        match message {
            Message::Title(title) => {
                if !draft.key_changed {
                    draft.key = propose_key(&title);
                }
                draft.title = title;
            }
            Message::Key(key) => {
                draft.key = key;
                draft.key_changed = true;
            }
            Message::Creators(creators) => draft.creators = creators,
            Message::Year(year) => draft.year = year,
            Message::Publisher(publisher) => draft.publisher = publisher,
            Message::Language(language) => draft.language = language,
            Message::Translators(translators) => draft.translators = translators,
            _ => {}
        }
    }

    fn take(&mut self, target: Target, file: PickedFile) {
        if file.bytes.len() > MAX_FILE_BYTES {
            self.draft = None;
            self.trouble = Some(Trouble::TooLarge);
        } else {
            self.draft = Some(Draft::new(target, file));
            self.trouble = None;
        }
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

    fn add_edition_to(&mut self, work: Uuid, edition: Edition) {
        if let Some(known) = self
            .works
            .as_mut()
            .and_then(|works| works.iter_mut().find(|known| known.id == work))
        {
            known.editions.push(edition);
        }
    }

    fn search(&mut self, connection: &Connection) -> Task<Message> {
        let words = self.query.trim().to_owned();
        if words.is_empty() {
            self.hits = None;
            return Task::none();
        }
        self.trouble = None;
        let Connection { api, token, .. } = connection.clone();
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
        if draft.edition.is_some() {
            return self.send_file(connection);
        }
        let Connection { api, token, .. } = connection.clone();
        let task = match draft.target {
            Target::Work => {
                let Some(new) = draft.new_work() else {
                    return Task::none();
                };
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
            Target::Edition(work) => {
                let Some(new) = draft.new_edition() else {
                    return Task::none();
                };
                Task::perform(
                    async move { api.add_edition(&token, work, &new).await.map_err(problem) },
                    Message::EditionAdded,
                )
            }
        };
        draft.sending = true;
        self.trouble = None;
        task
    }

    fn send_file(&mut self, connection: &Connection) -> Task<Message> {
        let Some(draft) = &mut self.draft else {
            return Task::none();
        };
        let Some(edition) = draft.edition else {
            return Task::none();
        };
        draft.sending = true;
        self.trouble = None;
        let Connection { api, token, .. } = connection.clone();
        let bytes = Arc::clone(&draft.file.bytes);
        Task::perform(
            async move {
                api.upload_file(&token, edition, bytes.as_ref().clone())
                    .await
                    .map_err(problem)
            },
            Message::Uploaded,
        )
    }

    fn make_standard(&mut self, edition: Uuid, connection: &Connection) -> Task<Message> {
        self.trouble = None;
        let Connection { api, token, .. } = connection.clone();
        Task::perform(
            async move { api.set_standard(&token, edition).await.map_err(problem) },
            Message::Changed,
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
                .on_press_maybe(self.free().then_some(Message::Pick(Target::Work))),
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
            content = content.push(self.draft_form(draft, localizer));
        }
        if let Some(form) = &self.page_form {
            content = content.push(page_form(form, localizer));
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
            Some(works) => column(
                works
                    .iter()
                    .map(|work| work_entry(work, self.free(), localizer)),
            )
            .spacing(gap::M)
            .into(),
        }
    }

    fn draft_form<'a>(&'a self, draft: &'a Draft, localizer: &Localizer) -> Element<'a, Message> {
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
        let creators_example = localizer.text("library-form-creators-example");
        let translators_example = localizer.text("library-form-translators-example");
        let heading = match draft.target {
            Target::Work => draft.file.name.clone(),
            Target::Edition(work) => {
                let mut args = FluentArgs::new();
                let title = self
                    .works
                    .as_ref()
                    .and_then(|works| works.iter().find(|known| known.id == work))
                    .map_or_else(String::new, |known| known.title.clone());
                args.set("title", title);
                args.set("file", draft.file.name.clone());
                localizer.format("library-form-new-edition", &args)
            }
        };
        let mut form = column![text(heading).font(font::UI_SEMIBOLD)].spacing(gap::S);
        if draft.target == Target::Work {
            form = form
                .push(field(
                    "library-form-title",
                    "",
                    &draft.title,
                    Message::Title,
                ))
                .push(field(
                    "library-form-creators",
                    &creators_example,
                    &draft.creators,
                    Message::Creators,
                ));
        }
        form = form
            .push(
                row![
                    field("library-form-year", "", &draft.year, Message::Year).width(YEAR_WIDTH),
                    field(
                        "library-form-publisher",
                        "",
                        &draft.publisher,
                        Message::Publisher
                    ),
                    field(
                        "library-form-language",
                        "",
                        &draft.language,
                        Message::Language
                    )
                    .width(LANGUAGE_WIDTH),
                ]
                .spacing(gap::S),
            )
            .push(field(
                "library-form-translators",
                &translators_example,
                &draft.translators,
                Message::Translators,
            ));
        if draft.target == Target::Work {
            form = form.push(field("library-form-key", "", &draft.key, Message::Key));
        }
        let actions: Element<'a, Message> = if draft.sending {
            text(localizer.text("library-uploading"))
                .style(style::muted)
                .into()
        } else {
            row![
                button(text(localizer.text("library-form-add")))
                    .padding([gap::XS + gap::XXS, gap::M])
                    .style(style::primary)
                    .on_press_maybe(draft.ready().then_some(Message::Upload)),
                button(text(localizer.text("library-form-cancel")))
                    .padding([gap::XS + gap::XXS, gap::M])
                    .style(style::secondary)
                    .on_press(Message::Cancel),
            ]
            .spacing(gap::S)
            .into()
        };
        container(form.push(actions))
            .padding(gap::M)
            .style(style::outline)
            .into()
    }
}

fn pick(target: Target) -> Task<Message> {
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
        move |file| Message::Picked(target, file),
    )
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
        args.set("page", hit.label.clone());
        let action: Element<'a, Message> = if hit.standard {
            button(text(localizer.text("library-cite")))
                .padding([gap::XS, gap::M])
                .style(style::secondary)
                .on_press(Message::Cite(hit.clone()))
                .into()
        } else {
            text(localizer.text("library-other-edition"))
                .size(font_size::SMALL)
                .style(style::muted)
                .into()
        };
        row![
            column![
                text(localizer.format("library-hit", &args)).font(font::UI_SEMIBOLD),
                text(hit.snippet.as_str())
                    .size(font_size::SMALL)
                    .style(style::muted),
            ]
            .spacing(gap::XXS)
            .width(Fill),
            action,
        ]
        .spacing(gap::M)
        .align_y(Vertical::Center)
        .into()
    }))
    .spacing(gap::S)
    .into()
}

fn work_entry<'a>(work: &'a Work, free: bool, localizer: &Localizer) -> Element<'a, Message> {
    let mut about = work.creators.clone();
    about.push(format!("@{}", work.key));
    let header = row![
        text(work.title.as_str()).font(font::UI_SEMIBOLD),
        space::horizontal(),
        button(text(localizer.text("library-add-edition")).size(font_size::SMALL))
            .padding([gap::XXS, gap::S])
            .style(style::ghost)
            .on_press_maybe(free.then_some(Message::Pick(Target::Edition(work.id)))),
    ]
    .align_y(Vertical::Center);
    column![
        header,
        text(about.join(" · "))
            .size(font_size::SMALL)
            .style(style::muted),
        column(
            work.editions
                .iter()
                .map(|edition| edition_line(edition, free, localizer))
        )
        .spacing(gap::XXS),
    ]
    .spacing(gap::XXS)
    .into()
}

fn edition_line<'a>(
    edition: &'a Edition,
    free: bool,
    localizer: &Localizer,
) -> Element<'a, Message> {
    let mut parts = Vec::new();
    if let Some(year) = edition.year {
        parts.push(year.to_string());
    }
    if !edition.publisher.is_empty() {
        parts.push(edition.publisher.clone());
    }
    if !edition.language.is_empty() {
        parts.push(edition.language.clone());
    }
    if !edition.translators.is_empty() {
        let mut args = FluentArgs::new();
        args.set("names", edition.translators.join("; "));
        parts.push(localizer.format("library-translators", &args));
    }
    let mut state = None;
    match &edition.file {
        Some(file) => {
            let mut args = FluentArgs::new();
            args.set("pages", file.pages);
            args.set("text", file.pages_with_text);
            match file.state {
                FileState::Ready => {
                    parts.push(localizer.format("library-pages", &args));
                    state = file.problem.clone().map(|problem| {
                        text(problem)
                            .size(font_size::SMALL)
                            .style(style::tone(Tone::Warning))
                    });
                }
                FileState::Recognizing => {
                    parts.push(localizer.format("library-pages", &args));
                    state = Some(
                        text(localizer.text("library-recognizing"))
                            .size(font_size::SMALL)
                            .style(style::tone(Tone::Info)),
                    );
                }
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
    let mut details = column![text(parts.join(" · ")).size(font_size::SMALL)].spacing(gap::XXS);
    if let Some(state) = state {
        details = details.push(state);
    }
    let mark: Element<'a, Message> = if edition.standard {
        text(localizer.text("library-standard"))
            .size(font_size::SMALL)
            .style(style::tone(Tone::Accent))
            .into()
    } else {
        button(text(localizer.text("library-make-standard")).size(font_size::SMALL))
            .padding([gap::XXS, gap::S])
            .style(style::secondary)
            .on_press(Message::MakeStandard(edition.id))
            .into()
    };
    let ready = edition
        .file
        .as_ref()
        .is_some_and(|file| matches!(file.state, FileState::Ready | FileState::Recognizing));
    let mut line = row![details.width(Fill)]
        .spacing(gap::M)
        .align_y(Vertical::Center)
        .padding([0.0, gap::M]);
    if ready {
        line = line.push(
            button(text(localizer.text("library-page-numbers")).size(font_size::SMALL))
                .padding([gap::XXS, gap::S])
                .style(style::ghost)
                .on_press_maybe(free.then_some(Message::EditPageMap(edition.id))),
        );
    }
    line.push(mark).into()
}

fn page_form<'a>(form: &'a PageForm, localizer: &Localizer) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("title", form.title.clone());
    let mut content =
        column![text(localizer.format("library-page-map-title", &args)).font(font::UI_SEMIBOLD)]
            .spacing(gap::S);
    if form.corrections.is_empty() {
        content = content.push(
            text(localizer.text("library-page-map-empty"))
                .size(font_size::SMALL)
                .style(style::muted),
        );
    }
    for (index, correction) in form.corrections.iter().enumerate() {
        let mut args = FluentArgs::new();
        args.set("page", correction.first_page);
        args.set("label", printed(correction));
        content = content.push(
            row![
                text(localizer.format("library-correction", &args)).size(font_size::SMALL),
                space::horizontal(),
                button(text(localizer.text("library-correction-remove")).size(font_size::SMALL))
                    .padding([gap::XXS, gap::S])
                    .style(style::ghost)
                    .on_press(Message::RemoveCorrection(index)),
            ]
            .align_y(Vertical::Center),
        );
    }
    let field = |label: &str, value: &'a str, on_input: fn(String) -> Message| {
        column![
            text(localizer.text(label))
                .size(font_size::SMALL)
                .style(style::muted),
            text_input("", value)
                .on_input(on_input)
                .padding([gap::XS + gap::XXS, gap::S])
                .style(style::field),
        ]
        .spacing(gap::XXS)
        .width(YEAR_WIDTH)
    };
    let style_button = |label: &str, style: LabelStyle| {
        button(text(localizer.text(label)).size(font_size::SMALL))
            .padding([gap::XS, gap::S])
            .style(style::tab(form.style == style))
            .on_press(Message::CorrectionStyle(style))
    };
    content = content.push(
        row![
            field(
                "library-correction-page",
                &form.page,
                Message::CorrectionPage
            ),
            field(
                "library-correction-start",
                &form.start,
                Message::CorrectionStart
            ),
            style_button("library-style-arabic", LabelStyle::Arabic),
            style_button("library-style-lower-roman", LabelStyle::LowerRoman),
            style_button("library-style-upper-roman", LabelStyle::UpperRoman),
            space::horizontal(),
            button(text(localizer.text("library-correction-add")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::secondary)
                .on_press_maybe(form.correction().map(|_| Message::AddCorrection)),
        ]
        .spacing(gap::S)
        .align_y(Vertical::Bottom),
    );
    content = content.push(
        row![
            button(text(localizer.text("library-page-map-save")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe((!form.sending).then_some(Message::SavePageMap)),
            button(text(localizer.text("library-form-cancel")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::secondary)
                .on_press(Message::Cancel),
        ]
        .spacing(gap::S),
    );
    container(content)
        .padding(gap::M)
        .style(style::outline)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_api::LibraryFile;
    use presswork_client::Api;

    use super::*;
    use crate::testing::{Route, run_task, serve};

    const PRESS: Uuid = Uuid::from_u128(1);
    const NOTES: Uuid = Uuid::from_u128(10);
    const NOTES_1455: Uuid = Uuid::from_u128(20);
    const NOTES_1922: Uuid = Uuid::from_u128(21);

    fn connection() -> Connection {
        Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
            user_name: "Ann".to_owned(),
        }
    }

    fn ready_file() -> LibraryFile {
        LibraryFile {
            size: 12_470,
            pages: 2,
            pages_with_text: 2,
            state: FileState::Ready,
            problem: None,
        }
    }

    fn edition(id: Uuid, year: i32, standard: bool, file: Option<LibraryFile>) -> Edition {
        Edition {
            id,
            year: Some(year),
            publisher: String::new(),
            language: String::new(),
            translators: Vec::new(),
            standard,
            corrections: Vec::new(),
            file,
        }
    }

    fn work(id: Uuid, key: &str, title: &str) -> Work {
        Work {
            id,
            organization_id: PRESS,
            key: key.to_owned(),
            title: title.to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            editions: vec![edition(NOTES_1455, 1455, true, Some(ready_file()))],
        }
    }

    fn in_state(state: FileState, problem: Option<&str>) -> Work {
        let mut work = work(NOTES, "field-notes", "Field Notes");
        if let Some(file) = &mut work.editions[0].file {
            file.state = state;
            file.pages_with_text = 0;
            file.problem = problem.map(str::to_owned);
        }
        work
    }

    fn loaded() -> LibraryView {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let _ = view.update(
            Message::Loaded(Ok(vec![work(NOTES, "field-notes", "Field Notes")])),
            &connection(),
        );
        view
    }

    fn picked(target: Target, name: &str, size: usize) -> Message {
        Message::Picked(
            target,
            Some(PickedFile {
                name: name.to_owned(),
                bytes: Arc::new(vec![0; size]),
            }),
        )
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

    fn hit(standard: bool) -> SearchHit {
        SearchHit {
            work_id: NOTES,
            edition_id: if standard { NOTES_1455 } else { NOTES_1922 },
            key: "field-notes".to_owned(),
            title: "Field Notes".to_owned(),
            page: 3,
            label: "iii".to_owned(),
            snippet: "The fine grain of the paper".to_owned(),
            standard,
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
    fn the_view_shows_the_works_and_their_editions() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        assert!(shows(&view, "Presswork gets the works of the library."));
        let _ = view.update(Message::Loaded(Ok(Vec::new())), &connection());
        assert!(shows(
            &view,
            "The library has no works. Add the first PDF file."
        ));
        let mut notes = work(NOTES, "field-notes", "Field Notes");
        notes.editions[0].publisher = "Field Press".to_owned();
        let mut translation = edition(NOTES_1922, 1922, false, None);
        translation.language = "de".to_owned();
        translation.translators = vec!["Mann, Erika".to_owned()];
        notes.editions.push(translation);

        let _ = view.update(Message::Loaded(Ok(vec![notes])), &connection());

        for words in [
            "Library · Field Press",
            "Field Notes",
            "Gutenberg, Johannes · @field-notes",
            "1455 · Field Press · 2 pages, 2 with text",
            "Standard edition",
            "1922 · de · Translated by Mann, Erika · No file",
        ] {
            assert!(shows(&view, words), "{words} is not in the view");
        }
        assert!(matches!(
            clicks(&view, "Make standard").as_slice(),
            [Message::MakeStandard(NOTES_1922)]
        ));
    }

    #[test]
    fn a_picked_file_gives_a_form_with_a_title_and_a_key() {
        let mut view = loaded();
        assert!(matches!(
            clicks(&view, "Add PDF…").as_slice(),
            [Message::Pick(Target::Work)]
        ));

        let _ = view.update(
            picked(Target::Work, "field_notes-2019.pdf", 10),
            &connection(),
        );

        assert!(shows(&view, "field_notes-2019.pdf"));
        assert_eq!(
            view.proposal(),
            Some(NewWork {
                key: "field-notes-2019".to_owned(),
                title: "field notes 2019".to_owned(),
                creators: Vec::new(),
                edition: NewEdition::default(),
            })
        );
    }

    #[test]
    fn the_key_follows_the_title_until_the_key_changes() {
        let mut view = loaded();
        let _ = view.update(picked(Target::Work, "scan.pdf", 10), &connection());

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
    fn the_form_reads_the_creators_and_the_edition() {
        let mut view = loaded();
        let _ = view.update(picked(Target::Work, "scan.pdf", 10), &connection());

        for message in [
            Message::Creators(" Tolstoy, Leo; Maude, Louise ;".to_owned()),
            Message::Year(" 1869 ".to_owned()),
            Message::Publisher(" Russky Vestnik ".to_owned()),
            Message::Language(" ru ".to_owned()),
            Message::Translators("Maude, Louise;".to_owned()),
        ] {
            let _ = view.update(message, &connection());
        }

        let proposal = view.proposal().unwrap();
        assert_eq!(proposal.creators, ["Tolstoy, Leo", "Maude, Louise"]);
        assert_eq!(
            proposal.edition,
            NewEdition {
                year: Some(1869),
                publisher: "Russky Vestnik".to_owned(),
                language: "ru".to_owned(),
                translators: vec!["Maude, Louise".to_owned()],
            }
        );
    }

    #[test]
    fn add_waits_for_a_valid_form() {
        for message in [
            Message::Year("soon".to_owned()),
            Message::Key("War".to_owned()),
            Message::Title("  ".to_owned()),
        ] {
            let mut view = loaded();
            let _ = view.update(picked(Target::Work, "scan.pdf", 10), &connection());
            let _ = view.update(message, &connection());
            assert_eq!(view.proposal(), None);
            assert!(clicks(&view, "Add").is_empty());
        }
        let mut view = loaded();
        let _ = view.update(picked(Target::Work, "scan.pdf", 10), &connection());
        assert!(matches!(clicks(&view, "Add").as_slice(), [Message::Upload]));
    }

    #[test]
    fn a_file_larger_than_the_limit_is_refused_at_once() {
        let mut view = loaded();

        let _ = view.update(
            picked(Target::Work, "big.pdf", MAX_FILE_BYTES + 1),
            &connection(),
        );

        assert_eq!(view.proposal(), None);
        assert!(shows(
            &view,
            "The file is larger than 64 MiB. Select a smaller file."
        ));
    }

    #[test]
    fn an_added_work_joins_the_list_and_closes_the_form() {
        let mut view = loaded();
        let _ = view.update(picked(Target::Work, "atlas.pdf", 10), &connection());

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
        let _ = view.update(picked(Target::Work, "atlas.pdf", 10), &connection());
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
    fn a_new_edition_gets_its_own_form_and_joins_its_work() {
        let mut view = loaded();
        assert!(matches!(
            clicks(&view, "Add edition…").as_slice(),
            [Message::Pick(Target::Edition(NOTES))]
        ));

        let _ = view.update(
            picked(Target::Edition(NOTES), "notes-1922.pdf", 10),
            &connection(),
        );
        assert!(shows(&view, "New edition of Field Notes: notes-1922.pdf"));
        let _ = view.update(Message::Year("1922".to_owned()), &connection());
        assert_eq!(
            view.edition_proposal(),
            Some(NewEdition {
                year: Some(1922),
                ..NewEdition::default()
            })
        );
        let _ = view.update(Message::Upload, &connection());
        let _ = view.update(
            Message::EditionAdded(Ok(edition(NOTES_1922, 1922, false, None))),
            &connection(),
        );

        assert_eq!(view.works().unwrap()[0].editions.len(), 2);
        assert!(shows(
            &view,
            "Presswork uploads the file and reads its pages."
        ));
    }

    #[test]
    fn a_search_cites_only_pages_of_the_standard_edition() {
        let mut view = loaded();
        let _ = view.update(Message::Query("fine grain".to_owned()), &connection());
        assert!(matches!(
            clicks(&view, "Find").as_slice(),
            [Message::Search]
        ));

        let _ = view.update(Message::Found(Ok(vec![hit(true)])), &connection());

        assert!(shows(&view, "Field Notes · p. iii"));
        assert!(shows(&view, "The fine grain of the paper"));
        let messages = clicks(&view, "Cite");
        assert!(
            matches!(messages.as_slice(), [Message::Cite(cited)] if *cited == hit(true)),
            "{messages:?}"
        );
        let _ = view.update(Message::Found(Ok(vec![hit(false)])), &connection());
        assert!(shows(&view, "Not the standard edition"));
        assert!(!shows(&view, "Cite"));
        let _ = view.update(Message::Found(Ok(Vec::new())), &connection());
        assert!(shows(&view, "No page has all these words."));
    }

    #[test]
    fn page_numbers_of_an_edition_get_corrections() {
        let mut view = loaded();
        assert!(matches!(
            clicks(&view, "Page numbers…").as_slice(),
            [Message::EditPageMap(NOTES_1455)]
        ));
        let _ = view.update(Message::EditPageMap(NOTES_1455), &connection());
        assert!(shows(&view, "Page numbers of Field Notes"));
        assert!(shows(&view, "The page numbers come from the PDF file."));

        for message in [
            Message::CorrectionPage("2".to_owned()),
            Message::CorrectionStart("1".to_owned()),
            Message::CorrectionStyle(LabelStyle::LowerRoman),
            Message::AddCorrection,
        ] {
            let _ = view.update(message, &connection());
        }

        assert!(shows(&view, "PDF page 2 is printed page i"));
        assert!(matches!(
            clicks(&view, "Save").as_slice(),
            [Message::SavePageMap]
        ));
        let mut corrected = work(NOTES, "field-notes", "Field Notes");
        corrected.editions[0].corrections = vec![PageRange {
            first_page: 2,
            style: LabelStyle::LowerRoman,
            prefix: String::new(),
            start: 1,
        }];
        let _ = view.update(Message::SavePageMap, &connection());
        let _ = view.update(Message::PageMapSaved(Ok(corrected)), &connection());
        assert!(!shows(&view, "Page numbers of Field Notes"));
        assert_eq!(view.works().unwrap()[0].editions[0].corrections.len(), 1);
    }

    #[test]
    fn a_correction_needs_page_numbers() {
        let mut view = loaded();
        let _ = view.update(Message::EditPageMap(NOTES_1455), &connection());

        let _ = view.update(Message::CorrectionPage("two".to_owned()), &connection());
        let _ = view.update(Message::CorrectionStart("1".to_owned()), &connection());

        assert!(clicks(&view, "Add correction").is_empty());
        let _ = view.update(Message::CorrectionPage("3".to_owned()), &connection());
        assert!(matches!(
            clicks(&view, "Add correction").as_slice(),
            [Message::AddCorrection]
        ));
    }

    #[test]
    fn a_file_that_the_server_reads_shows_its_state() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());

        let _ = view.update(
            Message::Loaded(Ok(vec![in_state(FileState::Reading, None)])),
            &connection(),
        );

        assert!(shows(&view, "1455 · 2 pages"));
        assert!(shows(&view, "The server reads the text of the pages."));
    }

    #[test]
    fn a_file_in_ocr_shows_its_state_and_lets_a_member_correct_the_page_numbers() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let mut recognizing = in_state(FileState::Recognizing, None);
        if let Some(file) = &mut recognizing.editions[0].file {
            file.pages_with_text = 1;
        }

        let _ = view.update(Message::Loaded(Ok(vec![recognizing])), &connection());

        assert!(shows(&view, "1455 · 2 pages, 1 with text"));
        assert!(shows(
            &view,
            "The server finds the text of the scanned pages with OCR."
        ));
        assert!(matches!(
            clicks(&view, "Page numbers…").as_slice(),
            [Message::EditPageMap(NOTES_1455)]
        ));
    }

    #[test]
    fn a_ready_file_shows_the_problem_of_its_ocr() {
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let problem = "OCR did not complete. The pages without a text layer have no text.";

        let _ = view.update(
            Message::Loaded(Ok(vec![in_state(FileState::Ready, Some(problem))])),
            &connection(),
        );

        assert!(shows(&view, "1455 · 2 pages, 0 with text"));
        assert!(shows(&view, problem));
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
        assert!(shows(&view, "1455 · 2 pages, 2 with text"));
        assert!(!shows(&view, "The server reads the text of the pages."));
        let elsewhere = Work {
            organization_id: Uuid::from_u128(99),
            ..work(Uuid::from_u128(12), "atlas", "Atlas")
        };
        assert!(!view.changed(elsewhere));
        assert_eq!(view.works().unwrap().len(), 1);
    }

    fn json(value: impl serde::Serialize) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    fn both_editions() -> Work {
        let mut notes = work(NOTES, "field-notes", "Field Notes");
        notes
            .editions
            .push(edition(NOTES_1922, 1922, false, Some(ready_file())));
        notes
    }

    fn server() -> Connection {
        let notes = || work(NOTES, "field-notes", "Field Notes");
        let base = serve(vec![
            Route::json(
                &format!("GET /api/v1/organizations/{PRESS}/works"),
                &json([notes()]),
            ),
            Route::json(
                &format!("GET /api/v1/organizations/{PRESS}/library/search"),
                &json([hit(true)]),
            ),
            Route::json(
                &format!("POST /api/v1/organizations/{PRESS}/works"),
                &json(notes()),
            ),
            Route::json(
                &format!("POST /api/v1/works/{NOTES}/editions"),
                &json(edition(NOTES_1922, 1922, false, None)),
            ),
            Route::json(
                &format!("PUT /api/v1/editions/{NOTES_1455}/file"),
                &json(notes()),
            ),
            Route::json(
                &format!("PUT /api/v1/editions/{NOTES_1922}/file"),
                &json(both_editions()),
            ),
            Route::json(
                &format!("POST /api/v1/editions/{NOTES_1922}/standard"),
                &json(both_editions()),
            ),
            Route::json(
                &format!("PUT /api/v1/editions/{NOTES_1455}/page-map"),
                &json(notes()),
            ),
        ]);
        Connection {
            api: Api::new(&base).unwrap(),
            token: "pws_ann".to_owned(),
            user_name: "Ann".to_owned(),
        }
    }

    #[test]
    fn the_works_come_from_the_server() {
        let connection = server();
        let view = LibraryView::new(PRESS, "Field Press".to_owned());

        let loaded = run_task(view.load(&connection));

        assert!(
            matches!(loaded.as_slice(), [Message::Loaded(Ok(works))] if works.len() == 1),
            "{loaded:?}"
        );
    }

    #[test]
    fn a_search_asks_the_server_and_an_empty_search_clears_the_hits() {
        let connection = server();
        let mut view = loaded();
        let _ = view.update(Message::Query("grain".to_owned()), &connection);

        let found = run_task(view.update(Message::Search, &connection));
        let [Message::Found(Ok(hits))] = found.as_slice() else {
            panic!("{found:?}");
        };
        assert_eq!(*hits, [hit(true)]);
        let _ = view.update(Message::Found(Ok(hits.clone())), &connection);
        let _ = view.update(Message::Query("  ".to_owned()), &connection);
        let cleared = run_task(view.update(Message::Search, &connection));

        assert_eq!(cleared.len(), 0);
        assert!(view.hits.is_none());
    }

    #[test]
    fn a_new_work_goes_to_the_server_and_then_its_file() {
        let connection = server();
        let mut view = LibraryView::new(PRESS, "Field Press".to_owned());
        let _ = view.update(picked(Target::Work, "Field_Notes.PDF", 10), &connection);
        let _ = view.update(Message::Year("1455".to_owned()), &connection);

        let created = run_task(view.update(Message::Upload, &connection));
        let [Message::Created(Ok(work))] = created.as_slice() else {
            panic!("{created:?}");
        };
        assert_eq!(run_task(view.update(Message::Upload, &connection)).len(), 0);
        let uploaded = run_task(view.update(Message::Created(Ok(work.clone())), &connection));
        let [Message::Uploaded(Ok(work))] = uploaded.as_slice() else {
            panic!("{uploaded:?}");
        };
        let _ = view.update(Message::Uploaded(Ok(work.clone())), &connection);

        assert!(view.draft.is_none());
        assert_eq!(view.works().map(<[Work]>::len), Some(1));
    }

    #[test]
    fn a_new_edition_goes_to_the_server_and_then_its_file() {
        let connection = server();
        let mut view = loaded();
        let _ = view.update(
            picked(Target::Edition(NOTES), "notes-1922.pdf", 10),
            &connection,
        );
        let _ = view.update(Message::Year("1922".to_owned()), &connection);

        let added = run_task(view.update(Message::Upload, &connection));
        let [Message::EditionAdded(Ok(added))] = added.as_slice() else {
            panic!("{added:?}");
        };
        let uploaded = run_task(view.update(Message::EditionAdded(Ok(added.clone())), &connection));

        assert_eq!(view.works().unwrap()[0].editions.len(), 2);
        assert!(
            matches!(uploaded.as_slice(), [Message::Uploaded(Ok(work))] if *work == both_editions()),
            "{uploaded:?}"
        );
    }

    #[test]
    fn a_failed_file_can_go_again_without_a_new_work() {
        let connection = server();
        let mut view = loaded();
        let _ = view.update(picked(Target::Work, "Field_Notes.pdf", 10), &connection);
        let _ = view.update(Message::Year("1455".to_owned()), &connection);
        let _ = view.update(
            Message::Created(Ok(work(NOTES, "field-notes", "Field Notes"))),
            &connection,
        );
        let _ = view.update(Message::Uploaded(Err(Problem::Unreachable)), &connection);

        let again = run_task(view.update(Message::Upload, &connection));

        assert!(
            matches!(again.as_slice(), [Message::Uploaded(Ok(_))]),
            "{again:?}"
        );
    }

    #[test]
    fn make_standard_asks_the_server() {
        let connection = server();
        let mut view = loaded();

        let changed = run_task(view.update(Message::MakeStandard(NOTES_1922), &connection));
        let [Message::Changed(Ok(work))] = changed.as_slice() else {
            panic!("{changed:?}");
        };
        let _ = view.update(Message::Changed(Ok(work.clone())), &connection);

        assert_eq!(view.works().unwrap()[0].editions.len(), 2);
    }

    #[test]
    fn the_page_map_goes_to_the_server() {
        let connection = server();
        let mut view = loaded();
        let _ = view.update(Message::EditPageMap(NOTES_1455), &connection);
        let _ = view.update(Message::CorrectionPage("3".to_owned()), &connection);
        let _ = view.update(Message::AddCorrection, &connection);

        let saved = run_task(view.update(Message::SavePageMap, &connection));
        let [Message::PageMapSaved(Ok(work))] = saved.as_slice() else {
            panic!("{saved:?}");
        };
        let _ = view.update(Message::PageMapSaved(Ok(work.clone())), &connection);

        assert!(view.page_form.is_none());
        assert_eq!(
            run_task(view.update(Message::SavePageMap, &connection)).len(),
            0
        );
    }

    #[test]
    fn a_failed_page_map_keeps_the_form_and_shows_the_problem() {
        let mut view = loaded();
        let _ = view.update(Message::EditPageMap(NOTES_1455), &connection());
        let _ = view.update(Message::SavePageMap, &connection());

        let _ = view.update(
            Message::PageMapSaved(Err(Problem::Status(500))),
            &connection(),
        );

        assert!(view.page_form.as_ref().is_some_and(|form| !form.sending));
        assert!(shows(&view, "The server answered with the status 500."));
        let _ = view.update(Message::Cancel, &connection());
        assert!(view.page_form.is_none());
    }

    #[test]
    fn a_correction_can_be_removed() {
        let mut view = loaded();
        let _ = view.update(Message::EditPageMap(NOTES_1455), &connection());
        let _ = view.update(Message::CorrectionPage("3".to_owned()), &connection());
        let _ = view.update(Message::AddCorrection, &connection());

        let _ = view.update(Message::RemoveCorrection(5), &connection());
        assert_eq!(view.page_form.as_ref().unwrap().corrections.len(), 1);
        let _ = view.update(Message::RemoveCorrection(0), &connection());

        assert_eq!(view.page_form.as_ref().unwrap().corrections.len(), 0);
    }

    #[test]
    fn an_unknown_edition_opens_no_page_form() {
        let mut view = loaded();

        let _ = view.update(Message::EditPageMap(Uuid::nil()), &connection());
        let _ = view.update(Message::AddCorrection, &connection());

        assert!(view.page_form.is_none());
    }

    #[test]
    fn a_failed_list_and_a_failed_change_show_the_problem() {
        let mut view = loaded();

        let _ = view.update(Message::Changed(Err(Problem::Status(409))), &connection());
        assert!(shows(&view, "The server answered with the status 409."));
        let _ = view.update(Message::Loaded(Err(Problem::Status(503))), &connection());

        assert!(shows(&view, "The server answered with the status 503."));
    }

    #[test]
    fn a_work_of_another_organization_is_not_kept() {
        let mut view = loaded();
        let mut other = work(Uuid::from_u128(99), "other", "Other");
        other.organization_id = Uuid::nil();

        assert!(!view.changed(other));
        assert_eq!(view.works().map(<[Work]>::len), Some(1));
    }

    #[test]
    fn a_cancelled_pick_and_a_citation_change_nothing() {
        let mut view = loaded();

        let _ = view.update(Message::Picked(Target::Work, None), &connection());
        let _ = view.update(Message::Cite(hit(true)), &connection());
        let _ = view.update(Message::Title("Ignored".to_owned()), &connection());

        assert!(view.draft.is_none());
        assert!(view.free());
    }
}
